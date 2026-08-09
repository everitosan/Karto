//! Casos de uso del workspace: CRUD de carpetas, diagramas, nodos, aristas,
//! propiedades y credenciales. Operan sobre la conexión ya descifrada que
//! entrega `VaultService::with_conn`, por lo que se prueban con una SQLite en
//! memoria migrada (sin SQLCipher ni disco).
//!
//! El SQL vive aquí por pragmatismo del proyecto (la persistencia es SQLite y
//! el puerto del dominio ya expone `rusqlite::Connection`). Cada función es
//! pequeña y testeable de forma aislada.

use crate::domain::{Credential, Edge, Folder, Graph, Map, Node};
use crate::error::AppResult;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;

/// Genera un identificador aleatorio (16 bytes hex) usando el PRNG de SQLite,
/// evitando una dependencia extra solo para UUIDs.
fn new_id(conn: &Connection) -> AppResult<String> {
    let id = conn.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
    Ok(id)
}

/// Siguiente posición libre entre hermanos (append al final).
fn next_position(conn: &Connection, sql: &str, parent: Option<&str>) -> AppResult<i64> {
    let pos = conn.query_row(sql, params![parent], |r| r.get(0))?;
    Ok(pos)
}

// --- Carpetas ---------------------------------------------------------------

pub fn folder_list(conn: &Connection) -> AppResult<Vec<Folder>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, name, color, position FROM folders ORDER BY position, name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Folder {
            id: r.get(0)?,
            parent_id: r.get(1)?,
            name: r.get(2)?,
            color: r.get(3)?,
            position: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn folder_create(
    conn: &Connection,
    name: &str,
    parent_id: Option<&str>,
) -> AppResult<Folder> {
    let id = new_id(conn)?;
    let position = next_position(
        conn,
        "SELECT COALESCE(MAX(position) + 1, 0) FROM folders WHERE parent_id IS ?1",
        parent_id,
    )?;
    conn.execute(
        "INSERT INTO folders (id, parent_id, name, color, position) VALUES (?1, ?2, ?3, NULL, ?4)",
        params![id, parent_id, name, position],
    )?;
    Ok(Folder {
        id,
        parent_id: parent_id.map(str::to_string),
        name: name.to_string(),
        color: None,
        position,
    })
}

pub fn folder_rename(conn: &Connection, id: &str, name: &str) -> AppResult<()> {
    conn.execute("UPDATE folders SET name = ?2 WHERE id = ?1", params![id, name])?;
    Ok(())
}

pub fn folder_set_color(conn: &Connection, id: &str, color: Option<&str>) -> AppResult<()> {
    conn.execute(
        "UPDATE folders SET color = ?2 WHERE id = ?1",
        params![id, color],
    )?;
    Ok(())
}

/// Mueve una carpeta a otro padre (anidación) y fija su posición.
pub fn folder_move(
    conn: &Connection,
    id: &str,
    parent_id: Option<&str>,
    position: i64,
) -> AppResult<()> {
    conn.execute(
        "UPDATE folders SET parent_id = ?2, position = ?3 WHERE id = ?1",
        params![id, parent_id, position],
    )?;
    Ok(())
}

pub fn folder_delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM folders WHERE id = ?1", params![id])?;
    Ok(())
}

// --- Diagramas (mapas) ------------------------------------------------------

pub fn map_list(conn: &Connection) -> AppResult<Vec<Map>> {
    let mut stmt = conn.prepare(
        "SELECT id, folder_id, name, viewport, position FROM maps ORDER BY position, name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Map {
            id: r.get(0)?,
            folder_id: r.get(1)?,
            name: r.get(2)?,
            viewport: r.get(3)?,
            position: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn map_create(conn: &Connection, name: &str, folder_id: Option<&str>) -> AppResult<Map> {
    let id = new_id(conn)?;
    let position = next_position(
        conn,
        "SELECT COALESCE(MAX(position) + 1, 0) FROM maps WHERE folder_id IS ?1",
        folder_id,
    )?;
    conn.execute(
        "INSERT INTO maps (id, folder_id, name, viewport, position) VALUES (?1, ?2, ?3, '{}', ?4)",
        params![id, folder_id, name, position],
    )?;
    Ok(Map {
        id,
        folder_id: folder_id.map(str::to_string),
        name: name.to_string(),
        viewport: "{}".to_string(),
        position,
    })
}

pub fn map_rename(conn: &Connection, id: &str, name: &str) -> AppResult<()> {
    conn.execute("UPDATE maps SET name = ?2 WHERE id = ?1", params![id, name])?;
    Ok(())
}

pub fn map_move(
    conn: &Connection,
    id: &str,
    folder_id: Option<&str>,
    position: i64,
) -> AppResult<()> {
    conn.execute(
        "UPDATE maps SET folder_id = ?2, position = ?3 WHERE id = ?1",
        params![id, folder_id, position],
    )?;
    Ok(())
}

pub fn map_delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM maps WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn map_set_viewport(conn: &Connection, id: &str, viewport: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE maps SET viewport = ?2 WHERE id = ?1",
        params![id, viewport],
    )?;
    Ok(())
}

// --- Grafo: nodos y aristas -------------------------------------------------

/// Carga el grafo completo de un diagrama (nodos con sus propiedades + aristas).
pub fn graph_load(conn: &Connection, map_id: &str) -> AppResult<Graph> {
    let mut node_stmt =
        conn.prepare("SELECT id, kind, label, x, y, parent_id FROM nodes WHERE map_id = ?1")?;
    let mut nodes: Vec<Node> = node_stmt
        .query_map(params![map_id], |r| {
            Ok(Node {
                id: r.get(0)?,
                map_id: map_id.to_string(),
                kind: r.get(1)?,
                label: r.get(2)?,
                x: r.get(3)?,
                y: r.get(4)?,
                parent_id: r.get(5)?,
                properties: HashMap::new(),
            })
        })?
        .collect::<Result<_, _>>()?;

    // Propiedades de todos los nodos del mapa en una sola consulta.
    let mut prop_stmt = conn.prepare(
        "SELECT p.node_id, p.key, p.value FROM node_properties p \
         JOIN nodes n ON n.id = p.node_id WHERE n.map_id = ?1",
    )?;
    let mut props: HashMap<String, HashMap<String, String>> = HashMap::new();
    let prop_rows = prop_stmt.query_map(params![map_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    for row in prop_rows {
        let (node_id, key, value) = row?;
        props.entry(node_id).or_default().insert(key, value);
    }
    for node in &mut nodes {
        if let Some(p) = props.remove(&node.id) {
            node.properties = p;
        }
    }

    let mut edge_stmt = conn
        .prepare("SELECT id, source_id, target_id, label, style FROM edges WHERE map_id = ?1")?;
    let edges = edge_stmt
        .query_map(params![map_id], |r| {
            Ok(Edge {
                id: r.get(0)?,
                map_id: map_id.to_string(),
                source_id: r.get(1)?,
                target_id: r.get(2)?,
                label: r.get(3)?,
                style: r.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    Ok(Graph { nodes, edges })
}

pub fn node_create(
    conn: &Connection,
    map_id: &str,
    kind: &str,
    label: &str,
    x: f64,
    y: f64,
) -> AppResult<Node> {
    let id = new_id(conn)?;
    conn.execute(
        "INSERT INTO nodes (id, map_id, kind, label, x, y) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, map_id, kind, label, x, y],
    )?;
    Ok(Node {
        id,
        map_id: map_id.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        x,
        y,
        parent_id: None,
        properties: HashMap::new(),
    })
}

pub fn node_set_position(conn: &Connection, id: &str, x: f64, y: f64) -> AppResult<()> {
    conn.execute(
        "UPDATE nodes SET x = ?2, y = ?3 WHERE id = ?1",
        params![id, x, y],
    )?;
    Ok(())
}

/// Cambia el grupo (padre) de un nodo. `parent_id = None` lo saca del grupo.
pub fn node_set_parent(conn: &Connection, id: &str, parent_id: Option<&str>) -> AppResult<()> {
    conn.execute(
        "UPDATE nodes SET parent_id = ?2 WHERE id = ?1",
        params![id, parent_id],
    )?;
    Ok(())
}

pub fn node_rename(conn: &Connection, id: &str, label: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE nodes SET label = ?2 WHERE id = ?1",
        params![id, label],
    )?;
    Ok(())
}

/// Reemplaza el conjunto completo de propiedades de un nodo (borra e inserta).
pub fn node_set_properties(
    conn: &Connection,
    id: &str,
    properties: &HashMap<String, String>,
) -> AppResult<()> {
    conn.execute("DELETE FROM node_properties WHERE node_id = ?1", params![id])?;
    for (key, value) in properties {
        conn.execute(
            "INSERT INTO node_properties (node_id, key, value) VALUES (?1, ?2, ?3)",
            params![id, key, value],
        )?;
    }
    Ok(())
}

pub fn node_delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM nodes WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn edge_create(
    conn: &Connection,
    map_id: &str,
    source_id: &str,
    target_id: &str,
    label: Option<&str>,
) -> AppResult<Edge> {
    let id = new_id(conn)?;
    conn.execute(
        "INSERT INTO edges (id, map_id, source_id, target_id, label) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, map_id, source_id, target_id, label],
    )?;
    Ok(Edge {
        id,
        map_id: map_id.to_string(),
        source_id: source_id.to_string(),
        target_id: target_id.to_string(),
        label: label.map(str::to_string),
        style: "{}".to_string(),
    })
}

pub fn edge_set_label(conn: &Connection, id: &str, label: Option<&str>) -> AppResult<()> {
    conn.execute(
        "UPDATE edges SET label = ?2 WHERE id = ?1",
        params![id, label],
    )?;
    Ok(())
}

/// Actualiza el estilo (JSON) de una arista: forma de la línea, etc.
pub fn edge_set_style(conn: &Connection, id: &str, style: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE edges SET style = ?2 WHERE id = ?1",
        params![id, style],
    )?;
    Ok(())
}

pub fn edge_delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM edges WHERE id = ?1", params![id])?;
    Ok(())
}

// --- Credenciales -----------------------------------------------------------

/// Lista las credenciales de un nodo **sin el secreto**.
pub fn credential_list(conn: &Connection, node_id: &str) -> AppResult<Vec<Credential>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, username, port, key_path, is_default, options, extras \
         FROM credentials WHERE node_id = ?1 ORDER BY is_default DESC, kind",
    )?;
    let rows = stmt.query_map(params![node_id], |r| {
        Ok(Credential {
            id: r.get(0)?,
            node_id: node_id.to_string(),
            kind: r.get(1)?,
            username: r.get(2)?,
            port: r.get::<_, Option<i64>>(3)?.map(|p| p as u16),
            key_path: r.get(4)?,
            is_default: r.get::<_, i64>(5)? != 0,
            options: r.get(6)?,
            extras: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Datos de entrada para crear/actualizar una credencial (incluye el secreto).
pub struct CredentialInput<'a> {
    pub id: Option<&'a str>,
    pub node_id: &'a str,
    pub kind: &'a str,
    pub username: Option<&'a str>,
    pub secret: Option<&'a str>,
    pub port: Option<u16>,
    pub key_path: Option<&'a str>,
    pub is_default: bool,
    /// Opciones SSH extra (texto libre, una por línea). `None`/vacío = ninguna.
    pub options: Option<&'a str>,
}

/// Crea o actualiza una credencial. Si `is_default`, desmarca las demás del nodo.
pub fn credential_upsert(conn: &Connection, input: CredentialInput) -> AppResult<Credential> {
    if input.is_default {
        conn.execute(
            "UPDATE credentials SET is_default = 0 WHERE node_id = ?1",
            params![input.node_id],
        )?;
    }
    let port = input.port.map(|p| p as i64);
    let is_default = input.is_default as i64;
    let id = match input.id {
        Some(existing) => {
            conn.execute(
                "UPDATE credentials SET kind = ?2, username = ?3, secret = ?4, port = ?5, \
                 key_path = ?6, is_default = ?7, options = ?8 WHERE id = ?1",
                params![
                    existing,
                    input.kind,
                    input.username,
                    input.secret,
                    port,
                    input.key_path,
                    is_default,
                    input.options
                ],
            )?;
            existing.to_string()
        }
        None => {
            let id = new_id(conn)?;
            conn.execute(
                "INSERT INTO credentials \
                 (id, node_id, kind, username, secret, port, key_path, is_default, options) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    input.node_id,
                    input.kind,
                    input.username,
                    input.secret,
                    port,
                    input.key_path,
                    is_default,
                    input.options
                ],
            )?;
            id
        }
    };
    Ok(Credential {
        id,
        node_id: input.node_id.to_string(),
        kind: input.kind.to_string(),
        username: input.username.map(str::to_string),
        port: input.port,
        key_path: input.key_path.map(str::to_string),
        is_default: input.is_default,
        options: input.options.map(str::to_string),
        extras: "{}".to_string(),
    })
}

/// Devuelve el secreto de una credencial bajo demanda explícita (botón copiar).
pub fn credential_reveal(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    let secret = conn
        .query_row(
            "SELECT secret FROM credentials WHERE id = ?1",
            params![id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    Ok(secret)
}

pub fn credential_delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM credentials WHERE id = ?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::migrations;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn folders_nest_and_position_appends() {
        let conn = db();
        let root = folder_create(&conn, "Proyecto", None).unwrap();
        assert_eq!(root.position, 0);
        let root2 = folder_create(&conn, "Otro", None).unwrap();
        assert_eq!(root2.position, 1);
        let child = folder_create(&conn, "Producción", Some(&root.id)).unwrap();
        assert_eq!(child.parent_id.as_deref(), Some(root.id.as_str()));
        assert_eq!(child.position, 0);

        assert_eq!(folder_list(&conn).unwrap().len(), 3);
    }

    #[test]
    fn folder_move_reparents_and_cascade_deletes() {
        let conn = db();
        let a = folder_create(&conn, "A", None).unwrap();
        let b = folder_create(&conn, "B", None).unwrap();
        let map = map_create(&conn, "Diagrama", Some(&b.id)).unwrap();

        folder_move(&conn, &b.id, Some(&a.id), 0).unwrap();
        let moved = folder_list(&conn)
            .unwrap()
            .into_iter()
            .find(|f| f.id == b.id)
            .unwrap();
        assert_eq!(moved.parent_id.as_deref(), Some(a.id.as_str()));

        // Al borrar A, B (hija) cae por cascada; su mapa queda huérfano en raíz
        // (ON DELETE SET NULL en maps.folder_id).
        folder_delete(&conn, &a.id).unwrap();
        assert!(folder_list(&conn).unwrap().is_empty());
        let orphan = map_list(&conn)
            .unwrap()
            .into_iter()
            .find(|m| m.id == map.id)
            .unwrap();
        assert_eq!(orphan.folder_id, None);
    }

    #[test]
    fn graph_roundtrip_with_properties_and_edges() {
        let conn = db();
        let map = map_create(&conn, "Red", None).unwrap();
        let web = node_create(&conn, &map.id, "server", "Web", 10.0, 20.0).unwrap();
        let db_node = node_create(&conn, &map.id, "database", "DB", 200.0, 20.0).unwrap();

        let mut props = HashMap::new();
        props.insert("ip".to_string(), "10.0.0.5".to_string());
        props.insert("hostname".to_string(), "web01".to_string());
        node_set_properties(&conn, &web.id, &props).unwrap();

        edge_create(&conn, &map.id, &web.id, &db_node.id, Some("5432/tcp")).unwrap();

        let graph = graph_load(&conn, &map.id).unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].label.as_deref(), Some("5432/tcp"));
        let loaded_web = graph.nodes.iter().find(|n| n.id == web.id).unwrap();
        assert_eq!(loaded_web.properties.get("ip").map(String::as_str), Some("10.0.0.5"));
        assert_eq!(loaded_web.x, 10.0);
    }

    #[test]
    fn node_position_and_delete_cascades_edges() {
        let conn = db();
        let map = map_create(&conn, "Red", None).unwrap();
        let a = node_create(&conn, &map.id, "server", "A", 0.0, 0.0).unwrap();
        let b = node_create(&conn, &map.id, "server", "B", 0.0, 0.0).unwrap();
        edge_create(&conn, &map.id, &a.id, &b.id, None).unwrap();

        node_set_position(&conn, &a.id, 42.0, 84.0).unwrap();
        node_delete(&conn, &a.id).unwrap();

        let graph = graph_load(&conn, &map.id).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.edges.is_empty(), "la arista cae por cascada al borrar el nodo");
    }

    #[test]
    fn credential_secret_never_listed_but_revealable() {
        let conn = db();
        let map = map_create(&conn, "Red", None).unwrap();
        let node = node_create(&conn, &map.id, "server", "A", 0.0, 0.0).unwrap();

        let cred = credential_upsert(
            &conn,
            CredentialInput {
                id: None,
                node_id: &node.id,
                kind: "ssh",
                username: Some("root"),
                secret: Some("s3cr3t"),
                port: Some(22),
                key_path: None,
                is_default: true,
                options: Some("ServerAliveInterval=60"),
            },
        )
        .unwrap();

        let listed = credential_list(&conn, &node.id).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].username.as_deref(), Some("root"));
        assert_eq!(listed[0].port, Some(22));
        assert!(listed[0].is_default);
        assert_eq!(listed[0].options.as_deref(), Some("ServerAliveInterval=60"));

        assert_eq!(
            credential_reveal(&conn, &cred.id).unwrap().as_deref(),
            Some("s3cr3t")
        );
    }

    #[test]
    fn credential_default_is_exclusive_per_node() {
        let conn = db();
        let map = map_create(&conn, "Red", None).unwrap();
        let node = node_create(&conn, &map.id, "server", "A", 0.0, 0.0).unwrap();

        let base = CredentialInput {
            id: None,
            node_id: &node.id,
            kind: "ssh",
            username: None,
            secret: None,
            port: None,
            key_path: None,
            is_default: true,
            options: None,
        };
        credential_upsert(&conn, CredentialInput { kind: "ssh", ..cred_clone(&base) }).unwrap();
        credential_upsert(&conn, CredentialInput { kind: "web", ..cred_clone(&base) }).unwrap();

        let listed = credential_list(&conn, &node.id).unwrap();
        let defaults = listed.iter().filter(|c| c.is_default).count();
        assert_eq!(defaults, 1, "solo una credencial por defecto por nodo");
    }

    // Helper para clonar el input en tests (CredentialInput no es Clone por los &str).
    fn cred_clone<'a>(input: &CredentialInput<'a>) -> CredentialInput<'a> {
        CredentialInput {
            id: input.id,
            node_id: input.node_id,
            kind: input.kind,
            username: input.username,
            secret: input.secret,
            port: input.port,
            key_path: input.key_path,
            is_default: input.is_default,
            options: input.options,
        }
    }
}
