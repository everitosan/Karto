//! Implementación de `VaultStore` sobre SQLCipher (SQLite cifrado).
//!
//! De momento usa la KDF nativa de SQLCipher (PBKDF2-HMAC-SHA512, salt en la
//! cabecera del archivo). El endurecimiento con Argon2id (Fase 5) requerirá
//! almacenar el salt/params de Argon2 de forma legible antes de descifrar.

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

/// Aplica la clave y fuerza el descifrado real; si la clave es incorrecta falla.
fn key_and_verify(conn: &Connection, passphrase: &str) -> AppResult<()> {
    conn.pragma_update(None, "key", passphrase)?;
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
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "key", passphrase)?;
        migrations::run(&conn)?;
        Ok(conn)
    }

    fn open(&self, path: &Path, passphrase: &str) -> AppResult<Connection> {
        if !path.exists() {
            return Err(AppError::NotFound);
        }
        let conn = Connection::open(path)?;
        key_and_verify(&conn, passphrase)?;
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
}
