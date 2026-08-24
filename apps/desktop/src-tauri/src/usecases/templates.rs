//! Plantillas de comando de conexión: renderizado y resolución.
//!
//! Modelo (decisión 2026-08-22, "split interno/envoltura"):
//!   - La **biblioteca** de plantillas vive a nivel de app/máquina
//!     (`app_store::templates`); es donde se editan.
//!   - Ligar una plantilla a un vault **copia** su comando al `launch_templates`
//!     del vault (portable: viaja con el `.karto` vía `VACUUM INTO`, sin lógica
//!     extra de export). Copia, no referencia, para que funcione autónoma en otra
//!     máquina.
//!   - Al conectar, el **comando interno** se resuelve así:
//!     `override del vault → (futuro) plantilla de app → default cableado`.
//!     La **envoltura** (terminal/cliente) sigue siendo de la máquina.
//!
//! `render` es puro (sustituye placeholders); el resto solo toca `launch_templates`.

use crate::domain::ConnectionRequest;
use crate::error::AppResult;
use rusqlite::{params, Connection, OptionalExtension};

/// Sustituye los placeholders de una plantilla con los datos de la conexión y
/// devuelve el argv resultante. Placeholders: `{host}`, `{port}`, `{user}`,
/// `{key}`, `{userhost}` (= `user@host` o `host` si no hay usuario).
///
/// Se sustituye **por token** (separado por espacios); los tokens que quedan
/// vacíos tras sustituir se descartan. Es responsabilidad del autor que la
/// plantilla case con la credencial (p. ej. no usar `{key}` sin llave).
pub fn render(command: &str, req: &ConnectionRequest) -> Vec<String> {
    let userhost = match &req.user {
        Some(u) => format!("{u}@{}", req.host),
        None => req.host.clone(),
    };
    let subs: [(&str, String); 5] = [
        ("{host}", req.host.clone()),
        ("{user}", req.user.clone().unwrap_or_default()),
        ("{port}", req.port.map(|p| p.to_string()).unwrap_or_default()),
        ("{key}", req.key_path.clone().unwrap_or_default()),
        ("{userhost}", userhost),
    ];
    command
        .split_whitespace()
        .filter_map(|tok| {
            let mut t = tok.to_string();
            for (k, v) in &subs {
                t = t.replace(k, v);
            }
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        })
        .collect()
}

/// Override del comando interno para un tipo de conexión, leído del vault.
/// Prefiere una fila específica del SO y cae a `'any'` (las ligadas se guardan
/// como `'any'` por ser portables). `None` si el vault no tiene override.
pub fn vault_override(conn: &Connection, connection: &str, os: &str) -> AppResult<Option<String>> {
    let cmd = conn
        .query_row(
            "SELECT command FROM launch_templates
             WHERE connection = ?1 AND os IN (?2, 'any')
             ORDER BY CASE os WHEN ?2 THEN 0 ELSE 1 END LIMIT 1",
            params![connection, os],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(cmd)
}

/// Copia (liga) un comando de plantilla al vault para un tipo de conexión. Se
/// guarda como `os = 'any'` (portable). Upsert por `(connection, os)`.
pub fn set_vault_override(conn: &Connection, connection: &str, command: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO launch_templates (id, connection, os, command)
         VALUES (lower(hex(randomblob(16))), ?1, 'any', ?2)
         ON CONFLICT(connection, os) DO UPDATE SET command = excluded.command",
        params![connection, command],
    )?;
    Ok(())
}

/// Overrides ligados en el vault (para mostrarlos / permitir desligar).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultTemplate {
    pub connection: String,
    pub os: String,
    pub command: String,
}

pub fn list_vault_overrides(conn: &Connection) -> AppResult<Vec<VaultTemplate>> {
    let mut stmt =
        conn.prepare("SELECT connection, os, command FROM launch_templates ORDER BY connection, os")?;
    let rows = stmt.query_map([], |r| {
        Ok(VaultTemplate {
            connection: r.get(0)?,
            os: r.get(1)?,
            command: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Desliga (borra) el override del vault para un tipo de conexión.
pub fn delete_vault_override(conn: &Connection, connection: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM launch_templates WHERE connection = ?1",
        [connection],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ConnectionKind, Os};
    use crate::infra::migrations;

    fn req() -> ConnectionRequest {
        ConnectionRequest {
            kind: ConnectionKind::Ssh,
            user: Some("root".into()),
            host: "10.0.0.5".into(),
            port: Some(2222),
            key_path: Some("/home/me/.ssh/id_ed25519".into()),
            url: None,
            ssh_options: Vec::new(),
        }
    }

    #[test]
    fn render_substitutes_placeholders() {
        let argv = render("ssh -D 1080 -i {key} -p {port} {userhost}", &req());
        assert_eq!(
            argv,
            vec!["ssh", "-D", "1080", "-i", "/home/me/.ssh/id_ed25519", "-p", "2222", "root@10.0.0.5"]
        );
    }

    #[test]
    fn render_drops_empty_tokens_when_no_value() {
        // Sin usuario ni puerto: `{port}` y `{user}` quedan vacíos y se descartan.
        let r = ConnectionRequest {
            user: None,
            port: None,
            key_path: None,
            ..req()
        };
        let argv = render("ssh {user} {port} {userhost}", &r);
        assert_eq!(argv, vec!["ssh", "10.0.0.5"]);
    }

    fn seeded_vault() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrations::run(&c).unwrap();
        c
    }

    #[test]
    fn link_lookup_and_unlink_roundtrip() {
        let c = seeded_vault();
        assert!(vault_override(&c, "ssh", "linux").unwrap().is_none());

        set_vault_override(&c, "ssh", "ssh -A {userhost}").unwrap();
        assert_eq!(
            vault_override(&c, "ssh", "linux").unwrap().as_deref(),
            Some("ssh -A {userhost}")
        );

        // Actualiza (upsert) sin duplicar.
        set_vault_override(&c, "ssh", "ssh -D 1080 {userhost}").unwrap();
        assert_eq!(list_vault_overrides(&c).unwrap().len(), 1);
        assert_eq!(
            vault_override(&c, "ssh", "linux").unwrap().as_deref(),
            Some("ssh -D 1080 {userhost}")
        );

        delete_vault_override(&c, "ssh").unwrap();
        assert!(vault_override(&c, "ssh", "linux").unwrap().is_none());
    }

    #[test]
    fn render_end_to_end_from_vault_override() {
        let c = seeded_vault();
        set_vault_override(&c, "ssh", "ssh -D 1080 -p {port} {userhost}").unwrap();
        let cmd = vault_override(&c, "ssh", Os::Linux.as_key()).unwrap().unwrap();
        assert_eq!(render(&cmd, &req()), vec!["ssh", "-D", "1080", "-p", "2222", "root@10.0.0.5"]);
    }
}
