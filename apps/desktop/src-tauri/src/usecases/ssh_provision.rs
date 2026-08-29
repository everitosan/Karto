//! Aprovisionamiento de acceso por llave SSH (endurecimiento, Fase 5).
//!
//! Cambia una conexión basada en contraseña por una basada en llave:
//!   1. Genera una llave **ed25519 sin passphrase** con `ssh-keygen` (una por
//!      credencial, en `~/.ssh/karto/`; reusa la existente si ya está).
//!   2. Copia la pública al servidor con `ssh-copy-id`, lanzado en la terminal
//!      del SO: el usuario teclea su contraseña **una sola vez**. Karto no
//!      intermedia el secreto (misma política que el connect por contraseña).
//!   3. Registra la ruta de la llave en la credencial para que las conexiones
//!      futuras usen `-i <llave>`.
//!
//! El armado de comandos es puro y testeable; la orquestación (`provision`) es
//! la única parte con efectos (genera archivos y lanza procesos).

use crate::domain::{ConnectionKind, ConnectionRequest, Os};
use crate::error::{AppError, AppResult};
use crate::usecases::connections::{
    detect_terminal, hold_line, program_in_path, pwsh_quote, require_terminal, resolve,
    ssh_inner_command, terminals_for, to_pwsh_line, to_shell_line, wrap_in_terminal, LaunchSpec,
};
use crate::usecases::ssh_import::ssh_dir;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Opciones del aprovisionamiento (mapean los checkboxes de la UI).
#[derive(Debug, Clone, Copy, Default)]
pub struct ProvisionOptions {
    /// Registrar la llave como método por defecto de la credencial (fija `key_path`).
    pub set_default_key: bool,
    /// Guardar el material de la llave privada dentro del vault (portable).
    pub store_in_vault: bool,
}

/// argv de `ssh-keygen` para una llave ed25519 sin passphrase (no interactivo).
pub fn keygen_command(key_path: &str, comment: &str) -> Vec<String> {
    vec![
        "ssh-keygen".into(),
        "-t".into(),
        "ed25519".into(),
        "-f".into(),
        key_path.into(),
        "-N".into(),
        String::new(), // passphrase vacía
        "-C".into(),
        comment.into(),
    ]
}


/// Directorio gestionado por Karto para sus llaves: `~/.ssh/karto`.
pub fn karto_keys_dir() -> Option<PathBuf> {
    ssh_dir().map(|d| d.join("karto"))
}

/// Ruta de la llave privada asociada a una credencial.
fn key_path_for(credential_id: &str) -> AppResult<PathBuf> {
    let dir =
        karto_keys_dir().ok_or_else(|| AppError::Other("no se encontró el HOME del usuario".into()))?;
    Ok(dir.join(format!("{credential_id}_ed25519")))
}

/// Marcador que la terminal deja en disco cuando `ssh-copy-id` **sí** funcionó.
///
/// Karto lanza la terminal con `spawn` y no puede esperarla (la sesión queda
/// abierta e interactiva), así que sin esta señal no hay forma de saber si la
/// llave llegó al servidor. Mismo patrón que el sondeo de facts.
pub fn marker_path(credential_id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("karto-provision-{credential_id}.ok"))
}

/// Script que encadena **instalar la llave**, el marcador de éxito y la sesión
/// interactiva ya con la llave nueva.
///
/// El marcador va **entre** la instalación y la sesión a propósito: así aparece
/// en cuanto la copia acaba, sin esperar a que el usuario cierre la terminal.
///
/// El encadenado difiere por SO y no es cosmético: Windows PowerShell 5.1 —el que
/// trae Windows de serie— **no tiene el operador `&&`** (llegó en PowerShell 7),
/// así que la condición se escribe con `$LASTEXITCODE`.
pub fn onboarding_script(
    install: &[String],
    ssh_key: &[String],
    marker: &str,
    os: Os,
) -> String {
    match os {
        Os::Windows => format!(
            "{}; if ($LASTEXITCODE -eq 0) {{ New-Item -ItemType File -Force -Path {} | Out-Null; {} }}",
            to_pwsh_line(install),
            pwsh_quote(marker),
            to_pwsh_line(ssh_key)
        ),
        _ => format!(
            "{} && touch {} && {}",
            to_shell_line(install),
            to_shell_line(&[marker.to_string()]),
            to_shell_line(ssh_key)
        ),
    }
}

/// Envuelve el script en la terminal del SO. macOS sigue sin soporte: su lista
/// de terminales está vacía y `require_terminal` da el error explicativo.
fn launch_in_terminal(script: &str) -> AppResult<LaunchSpec> {
    let os = Os::current();
    let held = hold_line(script, os);
    let term = detect_terminal(terminals_for(os), program_in_path);
    Ok(wrap_in_terminal(require_terminal(term)?, &held))
}

/// Orquesta el aprovisionamiento (efectos: genera la llave y lanza en una
/// terminal `ssh-copy-id && touch <marca> && ssh -i llave`; el usuario teclea la
/// contraseña una vez y queda conectado por llave). Devuelve la ruta de la llave
/// privada generada.
///
/// **No persiste nada en la credencial**: eso lo hace `commit_if_provisioned`
/// cuando aparece el marcador. La terminal es interactiva y se lanza con `spawn`,
/// así que aquí todavía no se sabe si el servidor aceptó la llave.
pub fn provision(
    conn: &Connection,
    node_id: &str,
    credential_id: &str,
    context_id: Option<&str>,
) -> AppResult<String> {
    // El host al que copiar la llave depende del contexto activo (misma
    // resolución que al conectar): en oficina o por VPN la dirección difiere.
    let req = resolve(conn, node_id, Some(credential_id), context_id)?;
    if req.kind != ConnectionKind::Ssh {
        return Err(AppError::Other(
            "solo las credenciales SSH admiten acceso por llave".into(),
        ));
    }

    let key_path = key_path_for(credential_id)?;
    let key_path_str = key_path.to_string_lossy().to_string();
    let pub_key_str = format!("{key_path_str}.pub");

    // 1) Generar la llave si aún no existe.
    if !key_path.exists() {
        if let Some(parent) = key_path.parent() {
            // `~/.ssh/karto` es directorio nuestro: se crea restringido al usuario
            // (0700 en Unix, DACL sin herencia en Windows) para que las llaves que
            // cuelguen de él no nazcan legibles por otras cuentas.
            let created = !parent.exists();
            std::fs::create_dir_all(parent)?;
            if created {
                crate::infra::file_perms::restrict_to_owner(
                    parent,
                    crate::infra::file_perms::Kind::Dir,
                )?;
            }
        }
        let argv = keygen_command(&key_path_str, &format!("karto:{node_id}"));
        let status = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .status()
            .map_err(|e| AppError::Other(format!("no se pudo ejecutar ssh-keygen: {e}")))?;
        if !status.success() {
            return Err(AppError::Other("ssh-keygen falló al generar la llave".into()));
        }
    }

    // 2) Instalar la pública + abrir la sesión por llave, en una sola terminal.
    //
    // Arranque: si la credencial ya trae una llave **que existe en este equipo**,
    // se usa para autenticar y el usuario no teclea nada. Si no la hay (o el
    // archivo no está), se omite y `ssh` pide la contraseña, como siempre. Que la
    // llave del usuario sirva de arranque es lo que permite no llevársela nunca
    // al vault: sólo viaja la que genera Karto.
    let bootstrap = req
        .key_path
        .as_deref()
        .filter(|k| !k.is_empty() && Path::new(k).exists() && *k != key_path_str);

    let public_key = std::fs::read_to_string(&pub_key_str)
        .map_err(|e| AppError::Other(format!("no se pudo leer la llave pública generada: {e}")))?;
    let install = crate::usecases::key_install::install_command(
        &public_key,
        req.user.as_deref(),
        &req.host,
        req.port,
        &req.ssh_options,
        bootstrap,
    )?;

    let key_req = ConnectionRequest {
        key_path: Some(key_path_str.clone()),
        ..req.clone()
    };
    let ssh_key = ssh_inner_command(&key_req);
    // Marca de una tanda anterior: se limpia para no dar por buena una copia vieja.
    let marker = marker_path(credential_id);
    let _ = std::fs::remove_file(&marker);
    let spec = launch_in_terminal(&onboarding_script(
        &install,
        &ssh_key,
        &marker.to_string_lossy(),
        Os::current(),
    ))?;
    std::process::Command::new(&spec.program)
        .args(&spec.args)
        .spawn()
        .map_err(|e| AppError::Other(format!("no se pudo lanzar el aprovisionamiento: {e}")))?;

    // 3) La credencial **no** se toca todavía: hasta que no exista el marcador no
    //    sabemos si el servidor aceptó la llave, y repuntar `key_path` a una llave
    //    que no está en `authorized_keys` deja al usuario sin acceso. Lo hace
    //    `commit_if_provisioned`, que el frontend sondea igual que los facts.
    Ok(key_path_str)
}

/// Si la terminal dejó el marcador de éxito, aplica los cambios pedidos sobre la
/// credencial y lo consume. Devuelve `true` si se aplicaron, `false` si la copia
/// aún no ha terminado (o falló y el usuario cerró la terminal).
///
/// Es idempotente: consumido el marcador, las llamadas siguientes dan `false`.
pub fn commit_if_provisioned(
    conn: &Connection,
    credential_id: &str,
    opts: ProvisionOptions,
) -> AppResult<bool> {
    let marker = marker_path(credential_id);
    if !marker.exists() {
        return Ok(false);
    }
    let key_path = key_path_for(credential_id)?;
    let key_path_str = key_path.to_string_lossy().to_string();

    if opts.store_in_vault {
        let private_key = std::fs::read_to_string(&key_path)
            .map_err(|e| AppError::Other(format!("no se pudo leer la llave generada: {e}")))?;
        conn.execute(
            "UPDATE credentials SET private_key = ?1 WHERE id = ?2",
            params![private_key, credential_id],
        )?;
    }
    if opts.set_default_key {
        conn.execute(
            "UPDATE credentials SET key_path = ?1 WHERE id = ?2",
            params![key_path_str, credential_id],
        )?;
    }
    let _ = std::fs::remove_file(&marker);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::migrations;

    #[test]
    fn keygen_command_is_noninteractive_ed25519() {
        assert_eq!(
            keygen_command("/home/me/.ssh/karto/c1_ed25519", "karto:n1"),
            vec![
                "ssh-keygen",
                "-t",
                "ed25519",
                "-f",
                "/home/me/.ssh/karto/c1_ed25519",
                "-N",
                "",
                "-C",
                "karto:n1",
            ]
        );
    }

    // El marcador va ENTRE la instalación y la sesión interactiva: así aparece
    // en cuanto la copia termina, sin esperar a que se cierre la terminal.
    #[test]
    fn onboarding_script_chains_install_marker_then_connection_on_linux() {
        let install = vec!["ssh".to_string(), "root@h".into(), "umask 077; ...".into()];
        let ssh = vec!["ssh".to_string(), "-i".into(), "/k".into(), "root@h".into()];
        assert_eq!(
            onboarding_script(&install, &ssh, "/tmp/m.ok", Os::Linux),
            "ssh root@h 'umask 077; ...' && touch /tmp/m.ok && ssh -i /k root@h"
        );
    }

    // Windows PowerShell 5.1 no tiene `&&` (llegó en PowerShell 7): la condición
    // se escribe con $LASTEXITCODE o el encadenado no se ejecuta.
    #[test]
    fn onboarding_script_uses_lastexitcode_on_windows() {
        let install = vec!["ssh".to_string(), "root@h".into()];
        let ssh = vec!["ssh".to_string(), "-i".into(), "C:/k".into(), "root@h".into()];
        let s = onboarding_script(&install, &ssh, "C:/tmp/m.ok", Os::Windows);
        assert!(!s.contains("&&"), "PowerShell 5.1 no lo soporta: {s}");
        assert!(s.starts_with("& 'ssh' 'root@h'; if ($LASTEXITCODE -eq 0) {"), "{s}");
        assert!(s.contains("New-Item -ItemType File -Force -Path 'C:/tmp/m.ok'"));
        assert!(s.contains("& 'ssh' '-i' 'C:/k' 'root@h'"));
    }

    // --- Atomicidad (Fase 3b.0) ---

    fn seed_cred() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn.execute("INSERT INTO maps (id, name) VALUES ('m1','M')", []).unwrap();
        conn.execute(
            "INSERT INTO nodes (id, map_id, kind, label, x, y) VALUES ('n1','m1','server','N',0,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO credentials (id, node_id, kind, username, key_path, is_default)              VALUES ('c1','n1','ssh','root','/ruta/previa',1)",
            [],
        )
        .unwrap();
        conn
    }

    fn key_path_of(cred: &str) -> std::path::PathBuf {
        key_path_for(cred).unwrap()
    }

    /// Sin marcador la credencial no se toca: es el caso de `ssh-copy-id` que
    /// falla y el usuario cierra la terminal. Antes se repuntaba `key_path` a una
    /// llave que el servidor nunca aceptó, dejando al usuario sin acceso.
    #[test]
    fn commit_does_nothing_without_the_success_marker() {
        let conn = seed_cred();
        let _ = std::fs::remove_file(marker_path("c1"));
        let opts = ProvisionOptions { set_default_key: true, store_in_vault: true };

        assert!(!commit_if_provisioned(&conn, "c1", opts).unwrap());

        let (kp, pk): (String, Option<String>) = conn
            .query_row(
                "SELECT key_path, private_key FROM credentials WHERE id = 'c1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(kp, "/ruta/previa", "no debe repuntar la llave");
        assert!(pk.is_none(), "no debe guardar material");
    }

    /// Con marcador sí aplica, y consume la marca: una segunda pasada da false
    /// (idempotente, el sondeo del frontend puede llamarlo varias veces).
    #[test]
    fn commit_applies_options_once_when_the_marker_is_present() {
        let conn = seed_cred();
        let key = key_path_of("c1");
        std::fs::create_dir_all(key.parent().unwrap()).unwrap();
        std::fs::write(&key, "MATERIAL-DE-LLAVE").unwrap();
        std::fs::write(marker_path("c1"), "").unwrap();
        let opts = ProvisionOptions { set_default_key: true, store_in_vault: true };

        assert!(commit_if_provisioned(&conn, "c1", opts).unwrap());

        let (kp, pk): (String, Option<String>) = conn
            .query_row(
                "SELECT key_path, private_key FROM credentials WHERE id = 'c1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(kp, key.to_string_lossy());
        assert_eq!(pk.as_deref(), Some("MATERIAL-DE-LLAVE"));

        assert!(!commit_if_provisioned(&conn, "c1", opts).unwrap(), "marcador consumido");
        let _ = std::fs::remove_file(&key);
    }
}
