//! Capa de casos de uso (aplicación): orquesta el dominio y los puertos.
//! Depende de abstracciones (traits del dominio), no de infraestructura, por lo
//! que es testeable sin SQLCipher, terminales ni el archivo real del usuario.

pub mod connections;
pub mod ssh_import;
pub mod vault;
pub mod workspace;
