//! Puertos (interfaces) que la capa de casos de uso necesita y que la
//! infraestructura implementa. Invierte la dependencia: los casos de uso
//! dependen de estos traits, no de SQLCipher.

use crate::error::AppResult;
use rusqlite::Connection;
use std::path::Path;

/// Persistencia del vault. Un caso de uso pide crear o abrir un vault y recibe
/// una conexión ya lista (con clave aplicada y migraciones corridas), sin saber
/// si por detrás hay SQLCipher, un archivo en memoria de test, etc.
pub trait VaultStore: Send + Sync + 'static {
    /// Crea un vault nuevo en `path` y devuelve la conexión migrada.
    fn create(&self, path: &Path, passphrase: &str) -> AppResult<Connection>;

    /// Abre un vault existente verificando la clave; aplica migraciones pendientes.
    fn open(&self, path: &Path, passphrase: &str) -> AppResult<Connection>;
}
