//! Preferencias de la app guardadas dentro del vault cifrado (tabla `settings`).
//! Clave/valor libre; el frontend interpreta cada clave (intervalo de
//! auto-bloqueo, segundos de limpieza de portapapeles, etc.). Al vivir en el
//! vault viajan con el `.karto` y quedan cifradas.

use crate::error::AppResult;
use rusqlite::Connection;
use std::collections::HashMap;

/// Devuelve todas las preferencias como un mapa clave→valor.
pub fn get_all(conn: &Connection) -> AppResult<HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = HashMap::new();
    for row in rows {
        let (k, v) = row?;
        out.insert(k, v);
    }
    Ok(out)
}

/// Fija (o reemplaza) el valor de una preferencia.
pub fn set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::migrations;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrations::run(&c).unwrap();
        c
    }

    #[test]
    fn get_all_empty_by_default() {
        assert!(get_all(&conn()).unwrap().is_empty());
    }

    #[test]
    fn set_inserts_and_updates() {
        let c = conn();
        set(&c, "autoLockMinutes", "10").unwrap();
        set(&c, "clipboardClearSeconds", "20").unwrap();
        let all = get_all(&c).unwrap();
        assert_eq!(all.get("autoLockMinutes").map(String::as_str), Some("10"));
        assert_eq!(all.get("clipboardClearSeconds").map(String::as_str), Some("20"));

        // Reescribe la misma clave (upsert, no duplica).
        set(&c, "autoLockMinutes", "5").unwrap();
        let all = get_all(&c).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all.get("autoLockMinutes").map(String::as_str), Some("5"));
    }
}
