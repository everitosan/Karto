//! Instalación de una llave pública en el `authorized_keys` del servidor, sin
//! depender de `ssh-copy-id`.
//!
//! `ssh-copy-id` es un script de shell que **no viene con el OpenSSH de Windows**,
//! así que apoyarse en él dejaba el aprovisionamiento como funcionalidad sólo de
//! Linux. Aquí se hace lo mismo que hace él, pero con un `ssh` a secas: mismo
//! camino en los tres SO y una dependencia menos.
//!
//! El script remoto viaja como **un único elemento del argv** de `ssh` (igual que
//! `facts::remote_script`), de modo que el quoting del shell local lo resuelve
//! `to_shell_line`/`to_pwsh_line` sin que aquí haya que saber en qué SO estamos.

use crate::error::{AppError, AppResult};

/// Script que corre en el **servidor**. Replica lo que hace `ssh-copy-id`:
///
/// - `umask 077` para que lo que se cree nazca privado (sshd rechaza un
///   `authorized_keys` accesible por otros, igual que el cliente rechaza una
///   llave privada abierta).
/// - `mkdir -p ~/.ssh` y `chmod 700`, por si es el primer acceso por llave.
/// - `grep -qxF` (cadena fija, línea completa) para **no duplicar** la entrada si
///   ya estaba: aprovisionar dos veces no debe dejar el archivo creciendo.
///
/// Falla si la pública trae una comilla simple: se incrusta entre comillas
/// simples en el script remoto y no hay forma segura de escaparla ahí. Las llaves
/// que genera Karto (`-C karto:<node_id>`) nunca la traen, así que esto es una
/// red de seguridad frente a una pública de origen inesperado.
pub fn remote_script(public_key: &str) -> AppResult<String> {
    let key = public_key.trim();
    if key.is_empty() {
        return Err(AppError::Other("la llave pública está vacía".into()));
    }
    if key.contains('\'') || key.contains('\n') {
        return Err(AppError::Other(
            "la llave pública contiene caracteres no admitidos para instalarla".into(),
        ));
    }
    Ok([
        "umask 077",
        "mkdir -p ~/.ssh",
        "chmod 700 ~/.ssh",
        "touch ~/.ssh/authorized_keys",
        &format!(
            "grep -qxF '{key}' ~/.ssh/authorized_keys || printf '%s\\n' '{key}' >> ~/.ssh/authorized_keys"
        ),
    ]
    .join("; "))
}

/// argv de `ssh` que instala `public_key` en el servidor.
///
/// `bootstrap_key` es la llave con la que **autenticarse para poder instalar**, no
/// la que se instala. Cuando la credencial ya tiene una llave que funciona en este
/// equipo se pasa aquí y el usuario no teclea nada; si no hay, se omite y `ssh`
/// pide la contraseña en la terminal (el flujo de siempre).
///
/// `IdentitiesOnly=yes` acompaña siempre a la llave de arranque: sin él `ssh`
/// ofrecería antes las identidades del agente y podría autenticar con otra
/// distinta de la que el usuario eligió, o agotar los intentos antes de llegar a
/// la buena.
pub fn install_command(
    public_key: &str,
    user: Option<&str>,
    host: &str,
    port: Option<u16>,
    ssh_options: &[String],
    bootstrap_key: Option<&str>,
) -> AppResult<Vec<String>> {
    let script = remote_script(public_key)?;
    let mut cmd = vec!["ssh".to_string()];
    if let Some(key) = bootstrap_key.filter(|k| !k.is_empty()) {
        cmd.push("-i".into());
        cmd.push(key.to_string());
        cmd.push("-o".into());
        cmd.push("IdentitiesOnly=yes".into());
    }
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
    cmd.push(script);
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA karto:n1";

    #[test]
    fn remote_script_creates_the_dir_and_avoids_duplicates() {
        let s = remote_script(KEY).unwrap();
        assert!(s.starts_with("umask 077"), "{s}");
        assert!(s.contains("mkdir -p ~/.ssh"));
        assert!(s.contains("chmod 700 ~/.ssh"));
        // Sin el grep, reaprovisionar dejaría la entrada duplicada.
        assert!(s.contains(&format!("grep -qxF '{KEY}' ~/.ssh/authorized_keys ||")));
        assert!(s.contains(">> ~/.ssh/authorized_keys"));
    }

    #[test]
    fn remote_script_rejects_a_key_that_could_break_out_of_the_quotes() {
        assert!(remote_script("ssh-ed25519 AAA 'rm -rf /'").is_err());
        assert!(remote_script("ssh-ed25519 AAA\nssh-ed25519 BBB").is_err());
        assert!(remote_script("   ").is_err());
    }

    #[test]
    fn install_command_without_bootstrap_lets_ssh_ask_for_the_password() {
        let argv = install_command(KEY, Some("root"), "10.0.0.5", Some(2222), &[], None).unwrap();
        assert_eq!(argv[0], "ssh");
        assert!(!argv.contains(&"-i".to_string()), "{argv:?}");
        assert!(argv.contains(&"root@10.0.0.5".to_string()));
        assert_eq!(argv[argv.len() - 2], "root@10.0.0.5", "el script va al final");
    }

    // El caso que evita que el usuario teclee nada: ya tiene una llave que
    // funciona, y sirve de arranque para instalar la de Karto.
    #[test]
    fn install_command_authenticates_with_the_bootstrap_key() {
        let argv = install_command(
            KEY,
            Some("root"),
            "10.0.0.5",
            None,
            &["ServerAliveInterval=60".into()],
            Some("/home/me/.ssh/id_ed25519"),
        )
        .unwrap();
        let joined = argv.join(" ");
        assert!(joined.contains("-i /home/me/.ssh/id_ed25519"));
        // Sin esto, ssh podría autenticar con otra identidad del agente.
        assert!(joined.contains("-o IdentitiesOnly=yes"));
        assert!(joined.contains("-o ServerAliveInterval=60"));
    }

    #[test]
    fn install_command_omits_an_empty_bootstrap_key() {
        let argv = install_command(KEY, None, "h", None, &[], Some("")).unwrap();
        assert!(!argv.contains(&"-i".to_string()), "{argv:?}");
        assert_eq!(argv[1], "h");
    }
}
