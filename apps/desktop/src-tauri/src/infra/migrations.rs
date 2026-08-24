//! Migraciones de esquema versionadas mediante `PRAGMA user_version`.

use crate::error::AppResult;
use rusqlite::Connection;

pub const SCHEMA_VERSION: i64 = 6;

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
    // 3 -> 4: preferencias de la app (endurecimiento, Fase 5). Clave/valor libre
    // dentro del vault cifrado y portable con el `.karto`. Guarda cosas como el
    // intervalo de auto-bloqueo o los segundos de limpieza del portapapeles.
    r#"
    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
    // 4 -> 5: material de llave SSH privada opcional guardado en el vault
    // (endurecimiento, Fase 5). Cuando el usuario elige "guardar la llave en el
    // diagrama", aquí va la privada (cifrada por SQLCipher, portable con el
    // `.karto`); al conectar se materializa a disco con permisos 0600 si falta.
    r#"
    ALTER TABLE credentials ADD COLUMN private_key TEXT;
    "#,
    // 5 -> 6: direcciones contextuales. La `ip` de un nodo deja de ser un
    // atributo fijo (se rompía al cambiar de sitio o entrar por VPN) y pasa a ser
    // un *localizador por contexto de acceso*: la dirección con la que alcanzas el
    // nodo depende de desde dónde te conectas. `access_contexts` son esos puntos
    // de vista (Oficina, VPN, Público…); `node_endpoints` guarda la dirección de
    // cada nodo en cada contexto. El `hostname` sigue en `node_properties` como
    // identidad estable y respaldo al resolver. Se migra la `ip` existente a un
    // contexto por defecto "Principal" para no perder datos.
    r#"
    CREATE TABLE access_contexts (
        id       TEXT PRIMARY KEY,
        name     TEXT NOT NULL,
        position INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE node_endpoints (
        node_id    TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
        context_id TEXT NOT NULL REFERENCES access_contexts(id) ON DELETE CASCADE,
        address    TEXT NOT NULL,
        PRIMARY KEY (node_id, context_id)
    );

    INSERT INTO access_contexts (id, name, position) VALUES ('default', 'Principal', 0);

    INSERT INTO node_endpoints (node_id, context_id, address)
        SELECT node_id, 'default', value FROM node_properties WHERE key = 'ip';

    DELETE FROM node_properties WHERE key = 'ip';
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
