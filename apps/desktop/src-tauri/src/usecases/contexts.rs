//! Casos de uso de contextos de acceso (puntos de vista de red): "Oficina",
//! "VPN", "Público"… Cada contexto determina qué dirección (`node_endpoints`) de
//! un nodo se usa al conectar, resolviendo el problema de que una IP privada
//! cambia según desde dónde te conectes. El catálogo de contextos vive en el
//! vault (viaja con el `.karto`); el contexto *activo* es estado local de cada
//! equipo y lo gestiona el frontend.

use crate::domain::AccessContext;
use crate::error::AppResult;
use rusqlite::{params, Connection};

/// Genera un identificador aleatorio (16 bytes hex) con el PRNG de SQLite.
fn new_id(conn: &Connection) -> AppResult<String> {
    let id = conn.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
    Ok(id)
}

/// Lista los contextos de acceso en orden de posición.
pub fn context_list(conn: &Connection) -> AppResult<Vec<AccessContext>> {
    let mut stmt =
        conn.prepare("SELECT id, name, position FROM access_contexts ORDER BY position, name")?;
    let rows = stmt.query_map([], |r| {
        Ok(AccessContext {
            id: r.get(0)?,
            name: r.get(1)?,
            position: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Crea un contexto nuevo (se añade al final).
pub fn context_create(conn: &Connection, name: &str) -> AppResult<AccessContext> {
    let id = new_id(conn)?;
    let position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM access_contexts",
        [],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO access_contexts (id, name, position) VALUES (?1, ?2, ?3)",
        params![id, name, position],
    )?;
    Ok(AccessContext {
        id,
        name: name.to_string(),
        position,
    })
}

pub fn context_rename(conn: &Connection, id: &str, name: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE access_contexts SET name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    Ok(())
}

/// Elimina un contexto. Sus endpoints caen por cascada (`ON DELETE CASCADE`).
pub fn context_delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM access_contexts WHERE id = ?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::migrations;
    use crate::usecases::workspace;
    use std::collections::HashMap;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn default_context_exists_after_migration() {
        let conn = db();
        let list = context_list(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "default");
        assert_eq!(list[0].name, "Principal");
    }

    #[test]
    fn create_appends_and_lists_in_order() {
        let conn = db();
        let office = context_create(&conn, "Oficina").unwrap();
        let vpn = context_create(&conn, "VPN").unwrap();
        assert!(vpn.position > office.position);
        let list = context_list(&conn).unwrap();
        let names: Vec<_> = list.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["Principal", "Oficina", "VPN"]);
    }

    #[test]
    fn rename_updates_name() {
        let conn = db();
        let ctx = context_create(&conn, "Oficina").unwrap();
        context_rename(&conn, &ctx.id, "Sede central").unwrap();
        let list = context_list(&conn).unwrap();
        assert!(list.iter().any(|c| c.name == "Sede central"));
    }

    #[test]
    fn delete_cascades_endpoints() {
        let conn = db();
        let map = workspace::map_create(&conn, "Red", None).unwrap();
        let node = workspace::node_create(&conn, &map.id, "server", "A", 0.0, 0.0).unwrap();
        let ctx = context_create(&conn, "VPN").unwrap();

        let mut endpoints = HashMap::new();
        endpoints.insert(ctx.id.clone(), "172.16.0.5".to_string());
        workspace::node_set_endpoints(&conn, &node.id, &endpoints).unwrap();

        context_delete(&conn, &ctx.id).unwrap();
        let loaded = workspace::graph_load(&conn, &map.id).unwrap();
        assert!(loaded.nodes[0].endpoints.is_empty(), "el endpoint cae con el contexto");
    }
}
