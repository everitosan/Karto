//! Migraciones de esquema versionadas mediante `PRAGMA user_version`.

use crate::error::AppResult;
use rusqlite::Connection;

pub const SCHEMA_VERSION: i64 = 3;

/// Cada entrada es el SQL que lleva del índice N al N+1.
const MIGRATIONS: &[&str] = &[
    // 0 -> 1: esquema inicial.
    r#"
    CREATE TABLE folders (
        id        TEXT PRIMARY KEY,
        parent_id TEXT REFERENCES folders(id) ON DELETE CASCADE,
        name      TEXT NOT NULL,
        color     TEXT,
        position  INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE maps (
        id        TEXT PRIMARY KEY,
        folder_id TEXT REFERENCES folders(id) ON DELETE SET NULL,
        name      TEXT NOT NULL,
        viewport  TEXT NOT NULL DEFAULT '{}',
        position  INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE nodes (
        id     TEXT PRIMARY KEY,
        map_id TEXT NOT NULL REFERENCES maps(id) ON DELETE CASCADE,
        kind   TEXT NOT NULL,
        label  TEXT NOT NULL,
        x      REAL NOT NULL DEFAULT 0,
        y      REAL NOT NULL DEFAULT 0
    );

    CREATE TABLE node_properties (
        node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
        key     TEXT NOT NULL,
        value   TEXT NOT NULL,
        PRIMARY KEY (node_id, key)
    );

    CREATE TABLE credentials (
        id         TEXT PRIMARY KEY,
        node_id    TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
        kind       TEXT NOT NULL,
        username   TEXT,
        secret     TEXT,
        port       INTEGER,
        key_path   TEXT,
        is_default INTEGER NOT NULL DEFAULT 0,
        extras     TEXT NOT NULL DEFAULT '{}'
    );

    CREATE TABLE edges (
        id        TEXT PRIMARY KEY,
        map_id    TEXT NOT NULL REFERENCES maps(id) ON DELETE CASCADE,
        source_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
        target_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
        label     TEXT,
        style     TEXT NOT NULL DEFAULT '{}'
    );

    CREATE TABLE launch_templates (
        id            TEXT PRIMARY KEY,
        connection    TEXT NOT NULL,
        os            TEXT NOT NULL,
        command       TEXT NOT NULL,
        UNIQUE (connection, os)
    );
    "#,
    // 1 -> 2: opciones SSH extra por credencial. Texto libre, una opción por
    // línea (p. ej. `ServerAliveInterval=60`); al conectar se prefija `-o` a
    // cada línea. Cubre keepalive, ProxyJump, ConnectTimeout, etc.
    r#"
    ALTER TABLE credentials ADD COLUMN options TEXT;
    "#,
    // 2 -> 3: agrupación de nodos. `parent_id` apunta a un nodo "zona"; los
    // hijos se mueven con el grupo. `x`/`y` pasan a ser relativos al padre
    // cuando hay `parent_id` (absolutos si es NULL). ON DELETE SET NULL para no
    // borrar los hijos al eliminar la zona.
    r#"
    ALTER TABLE nodes ADD COLUMN parent_id TEXT REFERENCES nodes(id) ON DELETE SET NULL;
    "#,
];

pub fn current_version(conn: &Connection) -> AppResult<i64> {
    let v: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(v)
}

pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let mut version = current_version(conn)?;
    while (version as usize) < MIGRATIONS.len() {
        conn.execute_batch(MIGRATIONS[version as usize])?;
        version += 1;
        conn.pragma_update(None, "user_version", version)?;
    }
    Ok(())
}
