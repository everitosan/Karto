//! Capa de infraestructura: implementaciones concretas de los puertos del
//! dominio (persistencia SQLCipher, migraciones de esquema y permisos de
//! archivos privados).

pub mod file_perms;
pub mod kdf;
pub mod migrations;
mod sqlcipher;

pub use sqlcipher::SqlcipherStore;
