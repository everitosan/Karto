//! Capa de casos de uso (aplicación): orquesta el dominio y los puertos.
//! Depende de abstracciones (traits del dominio), no de infraestructura, por lo
//! que es testeable sin SQLCipher, terminales ni el archivo real del usuario.

pub mod app_store;
pub mod connections;
pub mod contexts;
pub mod diagnostics;
pub mod export_subset;
pub mod facts;
pub mod health;
pub mod scripts;
pub mod settings;
pub mod ssh_import;
pub mod ssh_provision;
pub mod templates;
pub mod vault;
pub mod workspace;
