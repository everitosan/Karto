//! Punto de entrada de la librería de Karto (backend Tauri).
//! `lib.rs` es la capa de adaptadores: traduce comandos de Tauri a llamadas de
//! la capa de casos de uso. La lógica real vive en `usecases/`, apoyada en
//! `domain/` (entidades + puertos) e `infra/` (SQLCipher).

mod domain;
mod error;
mod infra;
mod usecases;

use domain::{Credential, Edge, Folder, Graph, ImportedHost, Map, Node, VaultInfo};
use error::{AppError, AppResult};
use infra::SqlcipherStore;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;
use usecases::connections;
use usecases::vault::VaultService;
use usecases::workspace;

/// Tipo concreto del servicio de vault con la persistencia real (SQLCipher).
type Vault = VaultService<SqlcipherStore>;

#[tauri::command]
fn vault_status(vault: tauri::State<Vault>) -> VaultInfo {
    vault.status()
}

#[tauri::command]
fn vault_create(
    vault: tauri::State<Vault>,
    path: String,
    password: String,
) -> AppResult<VaultInfo> {
    vault.create(&PathBuf::from(path), &password)
}

#[tauri::command]
fn vault_unlock(
    vault: tauri::State<Vault>,
    path: String,
    password: String,
) -> AppResult<VaultInfo> {
    vault.unlock(&PathBuf::from(path), &password)
}

#[tauri::command]
fn vault_lock(vault: tauri::State<Vault>) -> VaultInfo {
    vault.lock()
}

#[tauri::command]
fn ssh_import_preview() -> AppResult<Vec<ImportedHost>> {
    let path = usecases::ssh_import::default_config_path()
        .ok_or_else(|| AppError::Other("no se encontró el HOME del usuario".into()))?;
    let file = std::fs::File::open(&path)?;
    let mut reader = std::io::BufReader::new(file);
    usecases::ssh_import::parse_hosts(&mut reader)
}

// --- Comandos del workspace (carpetas, diagramas, grafo, credenciales) ------
// Cada uno toma la conexión descifrada del vault; falla si está bloqueado.

#[tauri::command]
fn folder_list(vault: tauri::State<Vault>) -> AppResult<Vec<Folder>> {
    vault.with_conn(|c| workspace::folder_list(c))
}

#[tauri::command]
fn folder_create(
    vault: tauri::State<Vault>,
    name: String,
    parent_id: Option<String>,
) -> AppResult<Folder> {
    vault.with_conn(|c| workspace::folder_create(c, &name, parent_id.as_deref()))
}

#[tauri::command]
fn folder_rename(vault: tauri::State<Vault>, id: String, name: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::folder_rename(c, &id, &name))
}

#[tauri::command]
fn folder_set_color(
    vault: tauri::State<Vault>,
    id: String,
    color: Option<String>,
) -> AppResult<()> {
    vault.with_conn(|c| workspace::folder_set_color(c, &id, color.as_deref()))
}

#[tauri::command]
fn folder_move(
    vault: tauri::State<Vault>,
    id: String,
    parent_id: Option<String>,
    position: i64,
) -> AppResult<()> {
    vault.with_conn(|c| workspace::folder_move(c, &id, parent_id.as_deref(), position))
}

#[tauri::command]
fn folder_delete(vault: tauri::State<Vault>, id: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::folder_delete(c, &id))
}

#[tauri::command]
fn map_list(vault: tauri::State<Vault>) -> AppResult<Vec<Map>> {
    vault.with_conn(|c| workspace::map_list(c))
}

#[tauri::command]
fn map_create(
    vault: tauri::State<Vault>,
    name: String,
    folder_id: Option<String>,
) -> AppResult<Map> {
    vault.with_conn(|c| workspace::map_create(c, &name, folder_id.as_deref()))
}

#[tauri::command]
fn map_rename(vault: tauri::State<Vault>, id: String, name: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::map_rename(c, &id, &name))
}

#[tauri::command]
fn map_move(
    vault: tauri::State<Vault>,
    id: String,
    folder_id: Option<String>,
    position: i64,
) -> AppResult<()> {
    vault.with_conn(|c| workspace::map_move(c, &id, folder_id.as_deref(), position))
}

#[tauri::command]
fn map_delete(vault: tauri::State<Vault>, id: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::map_delete(c, &id))
}

#[tauri::command]
fn map_set_viewport(vault: tauri::State<Vault>, id: String, viewport: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::map_set_viewport(c, &id, &viewport))
}

#[tauri::command]
fn graph_load(vault: tauri::State<Vault>, map_id: String) -> AppResult<Graph> {
    vault.with_conn(|c| workspace::graph_load(c, &map_id))
}

#[tauri::command]
fn node_create(
    vault: tauri::State<Vault>,
    map_id: String,
    kind: String,
    label: String,
    x: f64,
    y: f64,
) -> AppResult<Node> {
    vault.with_conn(|c| workspace::node_create(c, &map_id, &kind, &label, x, y))
}

#[tauri::command]
fn node_set_position(vault: tauri::State<Vault>, id: String, x: f64, y: f64) -> AppResult<()> {
    vault.with_conn(|c| workspace::node_set_position(c, &id, x, y))
}

#[tauri::command]
fn node_set_parent(
    vault: tauri::State<Vault>,
    id: String,
    parent_id: Option<String>,
) -> AppResult<()> {
    vault.with_conn(|c| workspace::node_set_parent(c, &id, parent_id.as_deref()))
}

#[tauri::command]
fn node_rename(vault: tauri::State<Vault>, id: String, label: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::node_rename(c, &id, &label))
}

#[tauri::command]
fn node_set_properties(
    vault: tauri::State<Vault>,
    id: String,
    properties: HashMap<String, String>,
) -> AppResult<()> {
    vault.with_conn(|c| workspace::node_set_properties(c, &id, &properties))
}

#[tauri::command]
fn node_delete(vault: tauri::State<Vault>, id: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::node_delete(c, &id))
}

#[tauri::command]
fn edge_create(
    vault: tauri::State<Vault>,
    map_id: String,
    source_id: String,
    target_id: String,
    label: Option<String>,
) -> AppResult<Edge> {
    vault.with_conn(|c| workspace::edge_create(c, &map_id, &source_id, &target_id, label.as_deref()))
}

#[tauri::command]
fn edge_set_label(vault: tauri::State<Vault>, id: String, label: Option<String>) -> AppResult<()> {
    vault.with_conn(|c| workspace::edge_set_label(c, &id, label.as_deref()))
}

#[tauri::command]
fn edge_set_style(vault: tauri::State<Vault>, id: String, style: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::edge_set_style(c, &id, &style))
}

#[tauri::command]
fn edge_delete(vault: tauri::State<Vault>, id: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::edge_delete(c, &id))
}

#[tauri::command]
fn credential_list(vault: tauri::State<Vault>, node_id: String) -> AppResult<Vec<Credential>> {
    vault.with_conn(|c| workspace::credential_list(c, &node_id))
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn credential_upsert(
    vault: tauri::State<Vault>,
    id: Option<String>,
    node_id: String,
    kind: String,
    username: Option<String>,
    secret: Option<String>,
    port: Option<u16>,
    key_path: Option<String>,
    is_default: bool,
    options: Option<String>,
) -> AppResult<Credential> {
    vault.with_conn(|c| {
        workspace::credential_upsert(
            c,
            workspace::CredentialInput {
                id: id.as_deref(),
                node_id: &node_id,
                kind: &kind,
                username: username.as_deref(),
                secret: secret.as_deref(),
                port,
                key_path: key_path.as_deref(),
                is_default,
                options: options.as_deref(),
            },
        )
    })
}

#[tauri::command]
fn credential_reveal(vault: tauri::State<Vault>, id: String) -> AppResult<Option<String>> {
    vault.with_conn(|c| workspace::credential_reveal(c, &id))
}

#[tauri::command]
fn credential_delete(vault: tauri::State<Vault>, id: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::credential_delete(c, &id))
}

/// Lanza la conexión de un nodo usando la credencial indicada (o la
/// predeterminada si `credential_id` es `None`). El secreto nunca vuelve al
/// frontend: el backend lo revela, arma el comando y arranca el proceso.
#[tauri::command]
fn connect_node(
    vault: tauri::State<Vault>,
    node_id: String,
    credential_id: Option<String>,
) -> AppResult<()> {
    vault.with_conn(|c| connections::connect_node(c, &node_id, credential_id.as_deref()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(VaultService::new(SqlcipherStore::new()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            vault_status,
            vault_create,
            vault_unlock,
            vault_lock,
            ssh_import_preview,
            folder_list,
            folder_create,
            folder_rename,
            folder_set_color,
            folder_move,
            folder_delete,
            map_list,
            map_create,
            map_rename,
            map_move,
            map_delete,
            map_set_viewport,
            graph_load,
            node_create,
            node_set_position,
            node_set_parent,
            node_rename,
            node_set_properties,
            node_delete,
            edge_create,
            edge_set_label,
            edge_set_style,
            edge_delete,
            credential_list,
            credential_upsert,
            credential_reveal,
            credential_delete,
            connect_node
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar la aplicación Karto");
}
