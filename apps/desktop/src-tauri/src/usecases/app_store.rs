//! Estado de app a nivel de equipo (NO secreto): vaults recientes y plantillas.
//! Vive en una **SQLite sin cifrar** en el directorio de datos del SO, separada
//! del `.karto` (que es el vault cifrado con los diagramas/credenciales).
//!
//! Se eligió SQLite sobre un JSON pensando en un **futuro CLI**: GUI y CLI son
//! procesos distintos que pueden escribir a la vez; SQLite da locking y
//! transacciones atómicas (evita carreras/corrupción), y consultas parciales.
//!
//! La **ruta se resuelve por variables de entorno** (no vía el runtime de Tauri)
//! para que un CLI, sin Tauri, apunte exactamente al mismo archivo. En Linux
//! coincide con el esquema de Tauri (`$XDG_DATA_HOME|~/.local/share` +
//! identificador). mac/Windows convencionales → Fase 7.

use crate::error::{AppError, AppResult};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Identificador de la app (igual al `identifier` de `tauri.conf.json`), usado
/// como subdirectorio en los dirs del SO.
const IDENTIFIER: &str = "app.karto.desktop";

/// Máximo de vaults recientes conservados.
const MAX_RECENTS: i64 = 10;

/// Una entrada de la lista de recientes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentVault {
    pub path: String,
    #[serde(default)]
    pub last_opened: i64,
}

/// Plantilla de comando de conexión (biblioteca a nivel de app, por máquina).
/// Guarda el **comando interno** con placeholders (`{host}`, `{port}`, `{user}`,
/// `{key}`, `{userhost}`). Se puede **ligar a un vault**: al ligar se *copia* al
/// `launch_templates` del vault (portable, viaja con el `.karto`). Ver
/// `usecases::templates` para el renderizado y la resolución.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub name: String,
    /// Tipo de conexión al que aplica: `ssh`/`vnc`/`web`/`rdp`.
    pub connection: String,
    /// Comando con placeholders. Ej.: `ssh -D 1080 {userhost}`.
    pub command: String,
    pub is_default: bool,
    pub position: i64,
}

/// Script remoto de la biblioteca (a nivel de app, por máquina). `body` es shell
/// que se ejecuta por SSH sobre los equipos de un diagrama (ver `usecases::scripts`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    pub id: String,
    pub name: String,
    /// Cuerpo del script, pasado por stdin al intérprete al ejecutar.
    pub body: String,
    pub position: i64,
    /// Carpeta a la que pertenece, o `None` si está suelto.
    pub folder_id: Option<String>,
    /// Intérprete: `bash` | `python` (y en el futuro `sql`/`mongo`/`redis`).
    pub interpreter: String,
}

/// Carpeta (plana, un nivel) para agrupar scripts en la biblioteca.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptFolder {
    pub id: String,
    pub name: String,
    pub position: i64,
}

// --- Resolución de rutas (independiente de Tauri, para compartir con el CLI) ---

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

fn base_from_env(var: &str, fallback: &str) -> Option<PathBuf> {
    match std::env::var_os(var) {
        Some(v) if !v.is_empty() => Some(PathBuf::from(v)),
        _ => home().map(|h| h.join(fallback)),
    }
}

/// Directorio de datos de la app (`~/.local/share/app.karto.desktop` en Linux).
pub fn data_dir() -> Option<PathBuf> {
    base_from_env("XDG_DATA_HOME", ".local/share").map(|b| b.join(IDENTIFIER))
}

/// Directorio de config de la app (`~/.config/app.karto.desktop` en Linux).
/// Solo se usa para localizar el `recent.json` heredado y migrarlo.
pub fn config_dir() -> Option<PathBuf> {
    base_from_env("XDG_CONFIG_HOME", ".config").map(|b| b.join(IDENTIFIER))
}

/// Ruta de la base de datos de estado de app.
pub fn db_path() -> AppResult<PathBuf> {
    let dir = data_dir().ok_or_else(|| AppError::Other("no se pudo resolver el dir de datos".into()))?;
    Ok(dir.join("app.db"))
}

/// Abre (creando si falta) la SQLite de estado de app y corre sus migraciones.
/// WAL para mejor concurrencia GUI/CLI. Sin cifrado (no hay secretos aquí).
pub fn open() -> AppResult<Connection> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&path)?;
    init(&conn)?;
    Ok(conn)
}

/// Inicializa PRAGMAs + migraciones sobre una conexión ya abierta (reutilizable
/// en tests con `Connection::open_in_memory`).
pub fn init(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
    let mut version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    while (version as usize) < MIGRATIONS.len() {
        conn.execute_batch(MIGRATIONS[version as usize])?;
        version += 1;
        conn.pragma_update(None, "user_version", version)?;
    }
    Ok(())
}

/// Migraciones de la DB de estado de app (versionadas con `user_version`, propio
/// e independiente del esquema del vault).
const MIGRATIONS: &[&str] = &[
    // 0 -> 1: recientes.
    r#"
    CREATE TABLE recent_vaults (
        path        TEXT PRIMARY KEY,
        last_opened INTEGER NOT NULL DEFAULT 0
    );
    "#,
    // 1 -> 2: biblioteca de plantillas de comando (a nivel de app/máquina). El
    // comando lleva placeholders que `usecases::templates::render` sustituye.
    // Se siembran ejemplos para descubribilidad (borrables por el usuario).
    r#"
    CREATE TABLE templates (
        id         TEXT PRIMARY KEY,
        name       TEXT NOT NULL,
        connection TEXT NOT NULL,
        command    TEXT NOT NULL,
        is_default INTEGER NOT NULL DEFAULT 0,
        position   INTEGER NOT NULL DEFAULT 0
    );

    INSERT INTO templates (id, name, connection, command, is_default, position) VALUES
        ('tpl-socks', 'SSH + túnel SOCKS (-D 1080)', 'ssh', 'ssh -D 1080 -i {key} -p {port} {userhost}', 1, 0),
        ('tpl-agent', 'SSH con reenvío de agente',    'ssh', 'ssh -A -i {key} -p {port} {userhost}',      1, 1);
    "#,
    // 2 -> 3: se retira la tabla `shortcuts` (los "alias" se descartaron; los
    // atajos ahora son de teclado por sección, gestionados en el frontend).
    // `IF EXISTS` para DBs nuevas que ya no la crean.
    r#"
    DROP TABLE IF EXISTS shortcuts;
    "#,
    // 3 -> 4: biblioteca de scripts remotos (a nivel de app/máquina). Se ejecutan
    // por SSH sobre los equipos de un diagrama; el `body` es shell (se pasa por
    // stdin a `bash -s`, ver `usecases::scripts`). Semillas descubribles (borrables).
    r#"
    CREATE TABLE scripts (
        id       TEXT PRIMARY KEY,
        name     TEXT NOT NULL,
        body     TEXT NOT NULL,
        position INTEGER NOT NULL DEFAULT 0
    );

    INSERT INTO scripts (id, name, body, position) VALUES
        ('scr-sysinfo', 'Info del sistema', 'uname -a
uptime', 0),
        ('scr-disk',    'Uso de disco',     'df -h', 1),
        ('scr-top',     'Procesos top',     'ps aux --sort=-%cpu | head', 2);
    "#,
    // 4 -> 5: carpetas (planas, un nivel) para agrupar scripts. `ON DELETE SET
    // NULL` (como `maps.folder_id` en el vault): borrar una carpeta no borra sus
    // scripts, quedan sueltos (`folder_id = NULL`).
    r#"
    CREATE TABLE script_folders (
        id       TEXT PRIMARY KEY,
        name     TEXT NOT NULL,
        position INTEGER NOT NULL DEFAULT 0
    );

    ALTER TABLE scripts ADD COLUMN folder_id TEXT
        REFERENCES script_folders(id) ON DELETE SET NULL;
    "#,
    // 5 -> 6: intérprete del script. Determina el comando remoto al ejecutar por
    // SSH: `bash` → `bash -s`, `python` → `python3 -` (cuerpo por stdin). Sienta
    // la base para futuros intérpretes de BD (sql/mongo/redis, otra tubería).
    r#"
    ALTER TABLE scripts ADD COLUMN interpreter TEXT NOT NULL DEFAULT 'bash';
    "#,
    // 6 -> 7: configuración de app a nivel de **máquina** (KV sin cifrar). Vive
    // aquí (no en el vault) porque aplica al equipo, no a un `.karto` concreto y
    // debe existir aun sin vault abierto. Hoy: nivel del log de diagnóstico.
    r#"
    CREATE TABLE app_config (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
];

// --- Config de app (KV a nivel de máquina) ---

/// Lee un valor de configuración de app (o `None` si no está fijado).
pub fn get_config(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM app_config WHERE key = ?1",
            rusqlite::params![key],
            |r| r.get(0),
        )
        .optional()?)
}

/// Fija (o reemplaza) un valor de configuración de app.
pub fn set_config(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

/// Epoch actual en milisegundos (impuro; la capa adaptadora lo pasa a `remember`).
pub fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// --- Recientes ---

/// Lista los recientes por apertura desc.
pub fn list_recents(conn: &Connection) -> AppResult<Vec<RecentVault>> {
    let mut stmt =
        conn.prepare("SELECT path, last_opened FROM recent_vaults ORDER BY last_opened DESC")?;
    let rows = stmt.query_map([], |r| {
        Ok(RecentVault {
            path: r.get(0)?,
            last_opened: r.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Registra/actualiza un vault como reciente (upsert) y recorta a `MAX_RECENTS`.
pub fn remember(conn: &Connection, path: &str, now: i64) -> AppResult<()> {
    conn.execute(
        "INSERT INTO recent_vaults (path, last_opened) VALUES (?1, ?2)
         ON CONFLICT(path) DO UPDATE SET last_opened = excluded.last_opened",
        rusqlite::params![path, now],
    )?;
    conn.execute(
        "DELETE FROM recent_vaults WHERE path NOT IN
            (SELECT path FROM recent_vaults ORDER BY last_opened DESC LIMIT ?1)",
        rusqlite::params![MAX_RECENTS],
    )?;
    Ok(())
}

/// Elimina un vault de recientes.
pub fn forget(conn: &Connection, path: &str) -> AppResult<()> {
    conn.execute("DELETE FROM recent_vaults WHERE path = ?1", [path])?;
    Ok(())
}

/// Borra de recientes los que ya no existen en disco. `exists` se inyecta para
/// poder probarlo sin tocar el sistema de archivos.
pub fn prune_missing(conn: &Connection, exists: impl Fn(&str) -> bool) -> AppResult<()> {
    let gone: Vec<String> = list_recents(conn)?
        .into_iter()
        .map(|r| r.path)
        .filter(|p| !exists(p))
        .collect();
    for path in gone {
        forget(conn, &path)?;
    }
    Ok(())
}

// --- Plantillas (biblioteca a nivel de app) ---

/// Lista las plantillas (defaults primero, luego por posición/nombre).
pub fn list_templates(conn: &Connection) -> AppResult<Vec<Template>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, connection, command, is_default, position
         FROM templates ORDER BY is_default DESC, position, name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Template {
            id: r.get(0)?,
            name: r.get(1)?,
            connection: r.get(2)?,
            command: r.get(3)?,
            is_default: r.get::<_, i64>(4)? != 0,
            position: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Lee una plantilla por id (para ligarla a un vault).
pub fn get_template(conn: &Connection, id: &str) -> AppResult<Option<Template>> {
    let row = conn
        .query_row(
            "SELECT id, name, connection, command, is_default, position FROM templates WHERE id = ?1",
            [id],
            |r| {
                Ok(Template {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    connection: r.get(2)?,
                    command: r.get(3)?,
                    is_default: r.get::<_, i64>(4)? != 0,
                    position: r.get(5)?,
                })
            },
        )
        .optional()?;
    Ok(row)
}

/// Crea o actualiza una plantilla. Con `id = None` genera uno nuevo.
pub fn upsert_template(
    conn: &Connection,
    id: Option<&str>,
    name: &str,
    connection: &str,
    command: &str,
) -> AppResult<()> {
    match id {
        Some(id) => conn.execute(
            "INSERT INTO templates (id, name, connection, command) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name, connection = excluded.connection, command = excluded.command",
            rusqlite::params![id, name, connection, command],
        )?,
        None => conn.execute(
            "INSERT INTO templates (id, name, connection, command)
             VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3)",
            rusqlite::params![name, connection, command],
        )?,
    };
    Ok(())
}

/// Elimina una plantilla de la biblioteca de app.
pub fn delete_template(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM templates WHERE id = ?1", [id])?;
    Ok(())
}

// --- Scripts (biblioteca a nivel de app) ---

/// Lista los scripts de la biblioteca (por carpeta, luego posición/nombre).
pub fn list_scripts(conn: &Connection) -> AppResult<Vec<Script>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, body, position, folder_id, interpreter FROM scripts ORDER BY position, name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Script {
            id: r.get(0)?,
            name: r.get(1)?,
            body: r.get(2)?,
            position: r.get(3)?,
            folder_id: r.get(4)?,
            interpreter: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

// --- Carpetas de scripts ---

/// Lista las carpetas de scripts (por posición/nombre).
pub fn list_script_folders(conn: &Connection) -> AppResult<Vec<ScriptFolder>> {
    let mut stmt =
        conn.prepare("SELECT id, name, position FROM script_folders ORDER BY position, name")?;
    let rows = stmt.query_map([], |r| {
        Ok(ScriptFolder {
            id: r.get(0)?,
            name: r.get(1)?,
            position: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Crea una carpeta (al final) y devuelve su id.
pub fn create_script_folder(conn: &Connection, name: &str) -> AppResult<String> {
    let id = conn.query_row(
        "INSERT INTO script_folders (id, name, position)
         VALUES (lower(hex(randomblob(16))), ?1,
                 COALESCE((SELECT MAX(position) + 1 FROM script_folders), 0))
         RETURNING id",
        rusqlite::params![name],
        |r| r.get::<_, String>(0),
    )?;
    Ok(id)
}

/// Renombra una carpeta.
pub fn rename_script_folder(conn: &Connection, id: &str, name: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE script_folders SET name = ?2 WHERE id = ?1",
        rusqlite::params![id, name],
    )?;
    Ok(())
}

/// Borra una carpeta. Sus scripts no se borran: quedan sueltos (`folder_id = NULL`
/// por `ON DELETE SET NULL`).
pub fn delete_script_folder(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM script_folders WHERE id = ?1", [id])?;
    Ok(())
}

/// Mueve un script a una carpeta (`Some`) o lo deja suelto (`None`).
pub fn set_script_folder(
    conn: &Connection,
    script_id: &str,
    folder_id: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE scripts SET folder_id = ?2 WHERE id = ?1",
        rusqlite::params![script_id, folder_id],
    )?;
    Ok(())
}

/// Crea o actualiza un script. Con `id = None` genera uno nuevo.
pub fn upsert_script(
    conn: &Connection,
    id: Option<&str>,
    name: &str,
    body: &str,
    interpreter: &str,
) -> AppResult<()> {
    match id {
        Some(id) => conn.execute(
            "INSERT INTO scripts (id, name, body, interpreter) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name, body = excluded.body, interpreter = excluded.interpreter",
            rusqlite::params![id, name, body, interpreter],
        )?,
        None => conn.execute(
            "INSERT INTO scripts (id, name, body, interpreter)
             VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3)",
            rusqlite::params![name, body, interpreter],
        )?,
    };
    Ok(())
}

/// Elimina un script de la biblioteca de app.
pub fn delete_script(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM scripts WHERE id = ?1", [id])?;
    Ok(())
}

// --- Migración del recent.json heredado ---

/// Importa (una vez) el `recent.json` que escribía la versión anterior a la DB.
/// Best-effort: si no existe o está corrupto, no hace nada. Tras importar,
/// renombra el archivo a `.migrated` para no repetir. `json_path` se inyecta.
pub fn import_legacy_recents(conn: &Connection, json_path: &std::path::Path) -> AppResult<()> {
    let Ok(text) = std::fs::read_to_string(json_path) else {
        return Ok(());
    };
    #[derive(Deserialize)]
    struct Legacy {
        #[serde(default)]
        recents: Vec<RecentVault>,
    }
    if let Ok(legacy) = serde_json::from_str::<Legacy>(&text) {
        for r in legacy.recents {
            remember(conn, &r.path, r.last_opened)?;
        }
    }
    let _ = std::fs::rename(json_path, json_path.with_extension("json.migrated"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        init(&c).unwrap();
        c
    }

    #[test]
    fn remember_upserts_orders_and_dedups() {
        let c = conn();
        remember(&c, "/a.karto", 100).unwrap();
        remember(&c, "/b.karto", 200).unwrap();
        remember(&c, "/a.karto", 300).unwrap(); // reabrir /a lo sube

        let list = list_recents(&c).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].path, "/a.karto");
        assert_eq!(list[0].last_opened, 300);
        assert_eq!(list[1].path, "/b.karto");
    }

    #[test]
    fn remember_trims_to_max() {
        let c = conn();
        for i in 0..(MAX_RECENTS + 5) {
            remember(&c, &format!("/v{i}.karto"), i).unwrap();
        }
        let list = list_recents(&c).unwrap();
        assert_eq!(list.len() as i64, MAX_RECENTS);
        assert_eq!(list[0].path, format!("/v{}.karto", MAX_RECENTS + 4));
    }

    #[test]
    fn forget_and_prune_remove_entries() {
        let c = conn();
        remember(&c, "/keep.karto", 1).unwrap();
        remember(&c, "/drop.karto", 2).unwrap();
        remember(&c, "/gone.karto", 3).unwrap();

        forget(&c, "/drop.karto").unwrap();
        prune_missing(&c, |p| p == "/keep.karto").unwrap();

        let list = list_recents(&c).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "/keep.karto");
    }

    #[test]
    fn import_legacy_recents_reads_json_and_marks_migrated() {
        let c = conn();
        let dir = std::env::temp_dir().join(format!("karto-legacy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let json = dir.join("recent.json");
        std::fs::write(
            &json,
            r#"{"recents":[{"path":"/old.karto","lastOpened":42}]}"#,
        )
        .unwrap();

        import_legacy_recents(&c, &json).unwrap();

        let list = list_recents(&c).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "/old.karto");
        assert_eq!(list[0].last_opened, 42);
        // El original se renombró (no se reimporta la próxima vez).
        assert!(!json.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn templates_seed_and_crud() {
        let c = conn();
        // Defaults sembrados por la migración v2.
        let seeded = list_templates(&c).unwrap();
        assert_eq!(seeded.len(), 2);
        assert!(seeded.iter().all(|t| t.is_default && t.connection == "ssh"));

        // Alta (id autogenerado) + lectura de vuelta.
        upsert_template(&c, None, "Mi plantilla", "ssh", "ssh {userhost}").unwrap();
        let all = list_templates(&c).unwrap();
        assert_eq!(all.len(), 3);
        let mine = all.iter().find(|t| t.name == "Mi plantilla").unwrap();
        assert!(!mine.is_default);

        // Edición por id.
        upsert_template(&c, Some(&mine.id), "Renombrada", "ssh", "ssh -v {userhost}").unwrap();
        let got = get_template(&c, &mine.id).unwrap().unwrap();
        assert_eq!(got.name, "Renombrada");
        assert_eq!(got.command, "ssh -v {userhost}");

        // Borrado.
        delete_template(&c, &mine.id).unwrap();
        assert_eq!(list_templates(&c).unwrap().len(), 2);
    }

    #[test]
    fn scripts_seed_and_crud() {
        let c = conn();
        // Semillas de la migración v4.
        let seeded = list_scripts(&c).unwrap();
        assert_eq!(seeded.len(), 3);
        assert_eq!(seeded[0].id, "scr-sysinfo");

        // Semillas por defecto en bash.
        assert!(seeded.iter().all(|s| s.interpreter == "bash"));

        // Alta (id autogenerado) + lectura de vuelta.
        upsert_script(&c, None, "Mi script", "echo hola", "bash").unwrap();
        let all = list_scripts(&c).unwrap();
        assert_eq!(all.len(), 4);
        let mine = all.iter().find(|s| s.name == "Mi script").unwrap();
        assert_eq!(mine.body, "echo hola");

        // Edición por id (cambia también el intérprete).
        upsert_script(&c, Some(&mine.id), "Renombrado", "print('x')", "python").unwrap();
        let got = list_scripts(&c)
            .unwrap()
            .into_iter()
            .find(|s| s.id == mine.id)
            .unwrap();
        assert_eq!(got.name, "Renombrado");
        assert_eq!(got.interpreter, "python");
        assert_eq!(got.body, "print('x')");

        // Borrado.
        delete_script(&c, &mine.id).unwrap();
        assert_eq!(list_scripts(&c).unwrap().len(), 3);
    }

    #[test]
    fn script_folders_crud_move_and_delete_unfiles() {
        let c = conn();
        assert!(list_script_folders(&c).unwrap().is_empty());

        // Alta + renombrado.
        let fid = create_script_folder(&c, "Redes").unwrap();
        rename_script_folder(&c, &fid, "Red").unwrap();
        let folders = list_script_folders(&c).unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].name, "Red");

        // Mover un script sembrado a la carpeta y de vuelta a suelto.
        set_script_folder(&c, "scr-disk", Some(&fid)).unwrap();
        let filed = list_scripts(&c)
            .unwrap()
            .into_iter()
            .find(|s| s.id == "scr-disk")
            .unwrap();
        assert_eq!(filed.folder_id.as_deref(), Some(fid.as_str()));

        // Borrar la carpeta: el script NO se borra, queda suelto (folder_id NULL).
        delete_script_folder(&c, &fid).unwrap();
        assert!(list_script_folders(&c).unwrap().is_empty());
        let unfiled = list_scripts(&c)
            .unwrap()
            .into_iter()
            .find(|s| s.id == "scr-disk")
            .unwrap();
        assert_eq!(unfiled.folder_id, None);
        assert_eq!(list_scripts(&c).unwrap().len(), 3, "no se borró ningún script");
    }

    #[test]
    fn import_legacy_recents_no_file_is_noop() {
        let c = conn();
        let missing = std::env::temp_dir().join("karto-legacy-missing-xyz.json");
        let _ = std::fs::remove_file(&missing);
        import_legacy_recents(&c, &missing).unwrap();
        assert!(list_recents(&c).unwrap().is_empty());
    }
}
