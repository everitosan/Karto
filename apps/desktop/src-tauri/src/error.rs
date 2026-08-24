use serde::Serialize;

/// Error de aplicación. Se serializa como string para cruzar el puente a JS.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("contraseña incorrecta")]
    WrongPassword,
    #[error("archivo de vault no encontrado")]
    NotFound,
    #[error("no hay ningún vault abierto")]
    NoVaultOpen,
    /// Este vault trae una plantilla de conexión personalizada y aún no se ha
    /// confirmado su ejecución. El texto es el comando a mostrar en la confirmación.
    #[error("confirmación de plantilla requerida: {0}")]
    TemplateConfirmationRequired(String),
    #[error("error de base de datos: {0}")]
    Db(String),
    #[error("error de E/S: {0}")]
    Io(String),
    #[error("{0}")]
    Other(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        // SQLCipher devuelve un error de "not a database" cuando la clave es incorrecta.
        let msg = e.to_string();
        if msg.contains("not a database") || msg.contains("HMAC") {
            AppError::WrongPassword
        } else {
            AppError::Db(msg)
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
