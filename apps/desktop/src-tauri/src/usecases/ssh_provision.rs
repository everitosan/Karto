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
    detect_terminal, hold_line, program_in_path, resolve, ssh_inner_command, to_shell_line,
    wrap_in_terminal, LaunchSpec, LINUX_TERMINALS,
};
use crate::usecases::ssh_import::ssh_dir;
use rusqlite::{params, Connection};
use std::path::PathBuf;

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

/// argv **interno** de `ssh-copy-id` (luego se envuelve en terminal + hold para
/// que el usuario teclee la contraseña y pueda ver el resultado).
pub fn copy_id_inner_command(
    pub_key_path: &str,
    user: Option<&str>,
    host: &str,
    port: Option<u16>,
    ssh_options: &[String],
) -> Vec<String> {
    let mut cmd = vec!["ssh-copy-id".to_string(), "-i".into(), pub_key_path.into()];
    if let Some(port) = port {
        cmd.push("-p".into());
        cmd.push(port.to_string());
    }
    for opt in ssh_options {
        cmd.push("-o".into());
        cmd.push(opt.clone());
    }
    cmd.push(match user {
        Some(u) => format!("{u}@{host}"),
        None => host.to_string(),
    });
    cmd
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

/// Script de shell que **encadena** `ssh-copy-id` y la conexión por llave: copia
/// la pública (el usuario teclea la contraseña) y, si tiene éxito, abre la sesión
/// ya con la llave (`copy && ssh -i …`). Función pura y testeable.
pub fn onboarding_script(copy_id: &[String], ssh_key: &[String]) -> String {
    format!("{} && {}", to_shell_line(copy_id), to_shell_line(ssh_key))
}

/// Envuelve un script en la terminal del SO (con `bash -c "…; read"`).
fn launch_in_terminal(script: &str) -> AppResult<LaunchSpec> {
    // El script viene de `onboarding_script`, en sintaxis POSIX y con
    // `ssh-copy-id`, que no existe en el OpenSSH de Windows: por eso el hold se
    // pide para Linux y el resto de SO sigue rechazado (ver Fase 3 del plan).
    let held = hold_line(script, Os::Linux);
    match Os::current() {
        Os::Linux => {
            let term = detect_terminal(LINUX_TERMINALS, program_in_path).ok_or_else(|| {
                AppError::Other(
                    "no se encontró una terminal soportada (instala gnome-terminal, konsole, kitty…)"
                        .into(),
                )
            })?;
            Ok(wrap_in_terminal(term, &held))
        }
        // mac/Windows: Fase 3 de docs/specs/windows-adapt.md (Windows necesita
        // además sustituir `ssh-copy-id`, que no viene con su OpenSSH).
        _ => Err(AppError::Other(
            "el aprovisionamiento de llave aún no está soportado en este sistema operativo".into(),
        )),
    }
}

/// Orquesta el aprovisionamiento completo (efectos: genera llave, lanza en una
/// terminal `ssh-copy-id && ssh -i llave` —el usuario teclea la contraseña una
/// vez y queda conectado por llave— y persiste según las opciones). Devuelve la
/// ruta de la llave privada.
pub fn provision(
    conn: &Connection,
    node_id: &str,
    credential_id: &str,
    context_id: Option<&str>,
    opts: ProvisionOptions,
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
            std::fs::create_dir_all(parent)?;
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

    // 2) Encadenar copia de la pública + conexión por llave en una sola terminal.
    let copy_id = copy_id_inner_command(
        &pub_key_str,
        req.user.as_deref(),
        &req.host,
        req.port,
        &req.ssh_options,
    );
    let key_req = ConnectionRequest {
        key_path: Some(key_path_str.clone()),
        ..req.clone()
    };
    let ssh_key = ssh_inner_command(&key_req);
    let spec = launch_in_terminal(&onboarding_script(&copy_id, &ssh_key))?;
    std::process::Command::new(&spec.program)
        .args(&spec.args)
        .spawn()
        .map_err(|e| AppError::Other(format!("no se pudo lanzar el aprovisionamiento: {e}")))?;

    // 3) Guardar la privada en el vault (portable) si se pidió.
    if opts.store_in_vault {
        let private_key = std::fs::read_to_string(&key_path)
            .map_err(|e| AppError::Other(format!("no se pudo leer la llave generada: {e}")))?;
        conn.execute(
            "UPDATE credentials SET private_key = ?1 WHERE id = ?2",
            params![private_key, credential_id],
        )?;
    }

    // 4) Fijar la llave como método por defecto de la credencial si se pidió.
    if opts.set_default_key {
        conn.execute(
            "UPDATE credentials SET key_path = ?1 WHERE id = ?2",
            params![key_path_str, credential_id],
        )?;
    }

    Ok(key_path_str)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn copy_id_includes_key_port_options_and_destination() {
        let cmd = copy_id_inner_command(
            "/k.pub",
            Some("root"),
            "10.0.0.5",
            Some(2222),
            &["ProxyJump bastion".into()],
        );
        assert_eq!(
            cmd,
            vec![
                "ssh-copy-id",
                "-i",
                "/k.pub",
                "-p",
                "2222",
                "-o",
                "ProxyJump bastion",
                "root@10.0.0.5",
            ]
        );
    }

    #[test]
    fn copy_id_without_user_or_port_uses_bare_host() {
        let cmd = copy_id_inner_command("/k.pub", None, "srv.local", None, &[]);
        assert_eq!(cmd, vec!["ssh-copy-id", "-i", "/k.pub", "srv.local"]);
    }

    #[test]
    fn onboarding_script_chains_copy_then_key_connection() {
        let copy = copy_id_inner_command("/k.pub", Some("root"), "h", Some(22), &[]);
        let ssh = vec![
            "ssh".to_string(),
            "-i".into(),
            "/k".into(),
            "root@h".into(),
        ];
        assert_eq!(
            onboarding_script(&copy, &ssh),
            "ssh-copy-id -i /k.pub -p 22 root@h && ssh -i /k root@h"
        );
    }
}
