//! Capa de infraestructura: implementaciones concretas de los puertos del
//! dominio (persistencia SQLCipher y migraciones de esquema).

pub mod migrations;
mod sqlcipher;

pub use sqlcipher::SqlcipherStore;
