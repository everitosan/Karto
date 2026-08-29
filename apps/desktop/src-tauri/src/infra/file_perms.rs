//! Permisos de archivos privados (llaves SSH) en Unix y Windows.
//!
//! Una llave privada en claro no debe ser legible por nadie más que su dueño, y
//! no es sólo higiene: `ssh` **se niega a usarla** si lo es. En Unix eso es
//! `0600`; en Windows es una DACL sin herencia, y el equivalente no es obvio:
//!
//! - `icacls <f> /grant:r usuario:F` —la receta que circula— **no basta**: sólo
//!   reemplaza las ACEs *de ese usuario*, así que una ACE explícita de `Everyone`
//!   sobrevive y OpenSSH sigue rechazando la llave (comprobado contra
//!   OpenSSH_for_Windows_9.5p2: *UNPROTECTED PRIVATE KEY FILE!*).
//! - Hace falta un `/reset` **antes**, que tira las ACEs explícitas, y sólo
//!   después `/inheritance:r` con los permisos definitivos.
//! - Los grupos van por **SID**, no por nombre: los nombres están localizados (en
//!   un Windows en español el grupo es `BUILTIN\Administradores`), así que
//!   pasarlos por nombre falla fuera de un sistema en inglés.

use crate::error::{AppError, AppResult};
use std::path::Path;

/// ¿La ruta es archivo o directorio? Cambia el modo Unix (0600 vs 0700) y la
/// herencia en Windows (un directorio debe propagar su DACL a lo que cuelgue).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Dir,
}

/// Restringe la ruta al usuario actual: en Unix `0600`/`0700`; en Windows una
/// DACL explícita con Administradores, SYSTEM y la cuenta actual, sin herencia.
///
/// Falla si no lo consigue. Dejar una llave privada con permisos desconocidos es
/// peor que no escribirla, así que el error se propaga en vez de ignorarse.
pub fn restrict_to_owner(path: &Path, kind: Kind) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = match kind {
            Kind::File => 0o600,
            Kind::Dir => 0o700,
        };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
        Ok(())
    }
    #[cfg(windows)]
    {
        windows_impl::restrict(path, kind)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, kind);
        Ok(())
    }
}

/// Resultado de auditar un archivo que **ya existía** antes de que Karto lo
/// tocara (p. ej. un vault abierto en otro equipo que apunta a una llave que ya
/// está en disco).
// En Windows `audit` sólo devuelve `Unknown`, así que las otras dos variantes
// quedan sin construir en esa plataforma: es correcto, no código muerto.
#[cfg_attr(not(unix), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audit {
    /// Sólo el dueño tiene acceso: es seguro reusarlo.
    Private,
    /// Accesible por más cuentas de las debidas; `detail` lo describe.
    Exposed { detail: String },
    /// No se pudo determinar en esta plataforma.
    Unknown,
}

/// Audita los permisos de un archivo existente. En Unix mira el modo. En Windows
/// leer la DACL exigiría una dependencia nueva de la API del SO, así que devuelve
/// `Unknown` y el llamador decide qué hacer con esa incertidumbre.
pub fn audit(path: &Path) -> AppResult<Audit> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)?.permissions().mode() & 0o777;
        return Ok(if mode & 0o077 != 0 {
            Audit::Exposed {
                detail: format!("permisos {mode:o}; deben ser 0600"),
            }
        } else {
            Audit::Private
        });
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(Audit::Unknown)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{AppError, AppResult, Kind, Path};

    /// Grupo local de administradores, por SID: el nombre está localizado.
    const SID_ADMINS: &str = "*S-1-5-32-544";
    /// Cuenta SYSTEM.
    const SID_SYSTEM: &str = "*S-1-5-18";

    /// Ruta absoluta de `icacls.exe`, deliberadamente **sin** pasar por el `PATH`:
    /// un `icacls.exe` colocado antes en el `PATH` por otro programa se ejecutaría
    /// con nuestros privilegios y sobre la ruta de una llave privada.
    fn icacls() -> std::path::PathBuf {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        std::path::PathBuf::from(root).join("System32\\icacls.exe")
    }

    /// Cuenta actual como `DOMINIO\usuario`, o a secas si no hay dominio.
    fn current_account() -> AppResult<String> {
        let user = std::env::var("USERNAME")
            .ok()
            .filter(|u| !u.is_empty())
            .ok_or_else(|| AppError::Other("no se pudo determinar la cuenta actual".into()))?;
        Ok(match std::env::var("USERDOMAIN") {
            Ok(domain) if !domain.is_empty() => format!("{domain}\\{user}"),
            _ => user,
        })
    }

    /// Lanza `icacls` con los argumentos dados y traduce un fallo en error.
    fn run(args: &[String]) -> AppResult<()> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let out = std::process::Command::new(icacls())
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| AppError::Other(format!("no se pudo ejecutar icacls: {e}")))?;
        if out.status.success() {
            return Ok(());
        }
        // icacls escribe el detalle del fallo en stdout, no en stderr.
        let detail = String::from_utf8_lossy(&out.stdout);
        Err(AppError::Other(format!(
            "icacls falló ({}): {}",
            out.status,
            detail.trim()
        )))
    }

    pub(super) fn restrict(path: &Path, kind: Kind) -> AppResult<()> {
        // `icacls` lee un argumento que empieza por `/` como modificador suyo
        // (`/reset`, `/grant`…), así que una ruta POSIX —la que trae un vault
        // exportado desde Linux— le da error de sintaxis en vez de aplicarse.
        // Exigir una ruta absoluta de este SO lo descarta y, de paso, evita
        // operar sobre algo relativo al directorio de trabajo del proceso.
        if !path.is_absolute() {
            return Err(AppError::Other(format!(
                "ruta no absoluta para este sistema: {}",
                path.display()
            )));
        }
        let target = path.to_string_lossy().to_string();
        let account = current_account()?;
        // (OI)(CI) hace que un directorio propague su DACL a lo que cuelgue de él.
        // Un archivo no lleva marcas de herencia.
        let perm = match kind {
            Kind::File => "(F)",
            Kind::Dir => "(OI)(CI)(F)",
        };

        // 1) Tira las ACEs **explícitas**. Sin esto, un `Everyone:(R)` explícito
        //    sobrevive al /grant:r del paso 2 y ssh sigue rechazando la llave.
        run(&[target.clone(), "/reset".into()])?;
        // 2) Corta la herencia y deja exactamente estas tres ACEs.
        run(&[
            target,
            "/inheritance:r".into(),
            "/grant:r".into(),
            format!("{SID_ADMINS}:{perm}"),
            format!("{SID_SYSTEM}:{perm}"),
            format!("{account}:{perm}"),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("karto-perms-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn restrict_file_then_audit_is_consistent_per_platform() {
        let f = tmp("clave");
        std::fs::write(&f, "material").unwrap();
        restrict_to_owner(&f, Kind::File).unwrap();
        // Unix puede leer el modo y confirmar 0600; Windows no puede leer la DACL
        // sin una dependencia nueva, así que informa Unknown (nunca Exposed).
        match audit(&f).unwrap() {
            Audit::Private => assert!(cfg!(unix)),
            Audit::Unknown => assert!(!cfg!(unix)),
            other => panic!("inesperado: {other:?}"),
        }
        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn restrict_dir_succeeds() {
        let d = tmp("subdir");
        std::fs::create_dir_all(&d).unwrap();
        restrict_to_owner(&d, Kind::Dir).unwrap();
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn audit_flags_a_group_readable_file() {
        use std::os::unix::fs::PermissionsExt;
        let f = tmp("abierta");
        std::fs::write(&f, "material").unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(audit(&f).unwrap(), Audit::Exposed { .. }));
        let _ = std::fs::remove_file(&f);
    }
}
