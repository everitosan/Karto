//! Capa de dominio: entidades puras y puertos (traits).
//! No depende de Tauri ni de detalles de infraestructura; el único tipo de
//! infraestructura que asoma es `rusqlite::Connection` en el puerto de
//! persistencia, por ser la tecnología de almacenamiento elegida del proyecto.

pub mod ports;
pub mod workspace;

pub use workspace::{Credential, Edge, Folder, Graph, Map, Node};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum VaultStatus {
    NoVault,
    Locked,
    Unlocked,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct VaultInfo {
    pub path: Option<String>,
    pub status: VaultStatus,
}

// --- Conexiones ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionKind {
    Ssh,
    Vnc,
    Rdp,
    Web,
}

impl ConnectionKind {
    /// Interpreta el `kind` textual de una credencial (guardado en la BD).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ssh" => Some(Self::Ssh),
            "vnc" => Some(Self::Vnc),
            "rdp" => Some(Self::Rdp),
            "web" => Some(Self::Web),
            _ => None,
        }
    }
}

/// Sistema operativo destino del lanzamiento. Se parametriza (en vez de usar
/// `cfg!` directamente) para poder probar el armado de comandos de las 3
/// plataformas sin compilar para cada una.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    Macos,
    Windows,
}

impl Os {
    /// SO en el que corre el proceso actual.
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Macos
        } else {
            Os::Linux
        }
    }
}

/// Petición de conexión ya resuelta desde el vault: host derivado de las
/// propiedades del nodo y datos de la credencial (con el secreto revelado, que
/// el backend consume y limpia; nunca vuelve al frontend).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionRequest {
    pub kind: ConnectionKind,
    pub user: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    pub key_path: Option<String>,
    /// URL de administración (solo para `Web`).
    pub url: Option<String>,
    /// Opciones SSH extra ya normalizadas (una por elemento, sin el `-o`).
    /// Se inyectan como `-o <opción>` antes del destino.
    pub ssh_options: Vec<String>,
}

// --- Import SSH ---

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ImportedHost {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
}
