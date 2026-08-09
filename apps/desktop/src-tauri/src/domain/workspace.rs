//! Entidades del workspace: carpetas, diagramas (mapas), nodos, aristas y
//! credenciales. Tipos puros de negocio, serializables hacia el frontend.
//! Se serializan en `camelCase` para casar con el dominio TypeScript.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Carpeta del árbol de organización. `parent_id = None` ⇒ carpeta raíz.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub color: Option<String>,
    pub position: i64,
}

/// Diagrama. `folder_id = None` ⇒ vive en la raíz del árbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Map {
    pub id: String,
    pub folder_id: Option<String>,
    pub name: String,
    /// Viewport de Svelte Flow serializado (JSON `{x,y,zoom}`).
    pub viewport: String,
    pub position: i64,
}

/// Nodo de infraestructura dentro de un diagrama.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub map_id: String,
    /// server/router/database/firewall/cdn/generic (validado en el frontend).
    pub kind: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    /// Nodo "zona" contenedor (agrupación). `x`/`y` son relativos al padre si
    /// está presente; absolutos si es `None`.
    pub parent_id: Option<String>,
    /// Propiedades tipadas clave/valor (ip, hostname, url_admin, notas…).
    pub properties: HashMap<String, String>,
}

/// Conexión etiquetada entre dos nodos.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub id: String,
    pub map_id: String,
    pub source_id: String,
    pub target_id: String,
    pub label: Option<String>,
    /// Estilo de la arista serializado (JSON).
    pub style: String,
}

/// Grafo completo de un diagrama: lo que el canvas carga y persiste.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// Credencial de un nodo **sin el secreto** (el secreto solo viaja bajo
/// demanda explícita vía `credential_reveal`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Credential {
    pub id: String,
    pub node_id: String,
    pub kind: String,
    pub username: Option<String>,
    pub port: Option<u16>,
    pub key_path: Option<String>,
    pub is_default: bool,
    /// Opciones SSH extra (texto libre, una por línea; se prefijan con `-o`).
    pub options: Option<String>,
    /// Extras serializados (JSON).
    pub extras: String,
}
