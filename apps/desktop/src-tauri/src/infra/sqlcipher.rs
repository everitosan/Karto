//! Implementación de `VaultStore` sobre SQLCipher (SQLite cifrado).
//!
//! La clave se deriva con **Argon2id** (módulo `kdf`) y se entrega a SQLCipher
//! en modo *raw key* (`PRAGMA key = "x'…'"`), evitando su PBKDF2 nativo. El salt
//! (16 bytes) se reutiliza del salt de cabecera de SQLCipher, que vive en claro
//! al principio del archivo: en `create` lo fijamos con `PRAGMA cipher_salt` y
//! en `open` lo leemos del header para re-derivar la misma clave.

use super::kdf;
use super::migrations;
use crate::domain::ports::VaultStore;
use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::Path;

#[derive(Default)]
pub struct SqlcipherStore;

impl SqlcipherStore {
    pub fn new() -> Self {
        Self
    }
}

/// Aplica una clave cruda ya derivada y fuerza el descifrado real; si es
/// incorrecta, la lectura de `sqlite_master` falla.
fn key_and_verify(conn: &Connection, key: &[u8; kdf::KEY_LEN]) -> AppResult<()> {
    conn.pragma_update(None, "key", kdf::key_pragma(key).as_str())?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    })?;
    Ok(())
}

impl VaultStore for SqlcipherStore {
    fn create(&self, path: &Path, passphrase: &str) -> AppResult<Connection> {
        if path.exists() {
            return Err(AppError::Other("ya existe un archivo en esa ruta".into()));
        }
        let salt = kdf::random_salt();
        let key = kdf::derive_key(passphrase, &salt)?;
        let conn = Connection::open(path)?;
        // La clave cruda va primero; luego fijamos nuestro salt como salt de
        // cabecera (SQLCipher solo respeta `cipher_salt` puesto DESPUÉS de `key`).
        // Queda escrito en claro en los primeros 16 bytes → releíble al abrir.
        conn.pragma_update(None, "key", kdf::key_pragma(&key).as_str())?;
        conn.pragma_update(None, "cipher_salt", kdf::salt_pragma(&salt))?;
        migrations::run(&conn)?;
        Ok(conn)
    }

    fn open(&self, path: &Path, passphrase: &str) -> AppResult<Connection> {
        if !path.exists() {
            return Err(AppError::NotFound);
        }
        let salt = kdf::read_header_salt(path)?;
        let key = kdf::derive_key(passphrase, &salt)?;
        let conn = Connection::open(path)?;
        key_and_verify(&conn, &key)?;
        migrations::run(&conn)?;
        Ok(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encrypts_and_rejects_wrong_password() {
        let dir = std::env::temp_dir().join(format!("karto-sqlcipher-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("v.karto");
        let _ = std::fs::remove_file(&path);

        let store = SqlcipherStore::new();
        {
            let conn = store.create(&path, "correcto-horse").unwrap();
            assert_eq!(
                migrations::current_version(&conn).unwrap(),
                migrations::SCHEMA_VERSION
            );
        }

        // Contraseña correcta → abre.
        assert!(store.open(&path, "correcto-horse").is_ok());
        // Contraseña incorrecta → WrongPassword.
        assert!(matches!(
            store.open(&path, "malo").unwrap_err(),
            AppError::WrongPassword
        ));

        let _ = std::fs::remove_file(&path);
    }

    /// El salt de cabecera escrito en `create` debe coincidir con los primeros 16
    /// bytes en claro del archivo (base de la re-derivación al abrir).
    #[test]
    fn header_salt_is_persisted_in_plaintext() {
        let dir = std::env::temp_dir().join(format!("karto-salt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("v.karto");
        let _ = std::fs::remove_file(&path);

        let store = SqlcipherStore::new();
        drop(store.create(&path, "pw").unwrap());

        // Reabrir dos veces produce el mismo resultado (salt estable, no aleatorio
        // en cada open) → la re-derivación desde el header funciona.
        assert!(store.open(&path, "pw").is_ok());
        assert!(store.open(&path, "pw").is_ok());
        // El header tiene al menos 16 bytes legibles.
        assert!(kdf::read_header_salt(&path).is_ok());

        let _ = std::fs::remove_file(&path);
    }
}
