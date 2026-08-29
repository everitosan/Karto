//! Punto de entrada de la librería de Karto (backend Tauri).
//! `lib.rs` es la capa de adaptadores: traduce comandos de Tauri a llamadas de
//! la capa de casos de uso. La lógica real vive en `usecases/`, apoyada en
//! `domain/` (entidades + puertos) e `infra/` (SQLCipher).

mod domain;
mod error;
mod infra;
mod usecases;

use domain::{
    AccessContext, Credential, Edge, Folder, Graph, ImportedHost, Map, Node, SearchHit, VaultInfo,
};
use error::{AppError, AppResult};
use infra::SqlcipherStore;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;
use usecases::app_store::{self, RecentVault};
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
    let info = vault.create(&PathBuf::from(&path), &password)?;
    remember_recent(&path);
    // Un vault creado localmente es de confianza para sus plantillas (el usuario es
    // su autor). La marca es machine-local por ruta; no viaja con el `.karto`.
    if let Ok(store) = open_app_store() {
        let _ = app_store::trust_vault_templates(&store, &path);
    }
    Ok(info)
}

#[tauri::command]
fn vault_unlock(
    vault: tauri::State<Vault>,
    path: String,
    password: String,
) -> AppResult<VaultInfo> {
    let info = vault.unlock(&PathBuf::from(&path), &password)?;
    remember_recent(&path);
    Ok(info)
}

#[tauri::command]
fn vault_lock(vault: tauri::State<Vault>) -> VaultInfo {
    vault.lock()
}

/// Cierra el vault por completo (olvida path + conexión) → vuelve a la selección.
#[tauri::command]
fn vault_close(vault: tauri::State<Vault>) -> VaultInfo {
    vault.close()
}

/// Cambia la contraseña maestra (re-cifra el vault con `PRAGMA rekey`). Verifica
/// la contraseña actual antes de aplicar. Requiere el vault desbloqueado.
#[tauri::command]
fn vault_rekey(vault: tauri::State<Vault>, current: String, new: String) -> AppResult<()> {
    vault.rekey(&current, &new)
}

/// Exporta una copia de respaldo cifrada del vault a `dest` (falla si ya existe).
#[tauri::command]
fn vault_export(vault: tauri::State<Vault>, dest: String) -> AppResult<()> {
    vault.export(&PathBuf::from(dest))
}

/// Exporta un subconjunto de nodos (los seleccionados en el lienzo) a un `.karto`
/// nuevo cifrado con `password`, filtrando el contenido con los flags `include_*`.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn vault_export_subset(
    vault: tauri::State<Vault>,
    dest: String,
    password: String,
    node_ids: Vec<String>,
    map_name: String,
    include_credentials: bool,
    include_facts: bool,
    include_ip: bool,
    include_notes: bool,
) -> AppResult<()> {
    let opts = usecases::export_subset::ExportOptions {
        credentials: include_credentials,
        facts: include_facts,
        ip: include_ip,
        notes: include_notes,
    };
    vault.export_subset(&PathBuf::from(dest), &password, &node_ids, &map_name, opts)
}

/// Escribe bytes en `path` (sobrescribe). Usado por el export del diagrama: la
/// imagen se genera en el frontend y aquí solo se persiste al disco. No toca el
/// vault, por eso no requiere `State<Vault>`.
#[tauri::command]
fn export_write(path: String, data: Vec<u8>) -> AppResult<()> {
    std::fs::write(&path, &data)?;
    Ok(())
}

/// Devuelve todas las preferencias guardadas en el vault.
#[tauri::command]
fn settings_get(vault: tauri::State<Vault>) -> AppResult<HashMap<String, String>> {
    vault.with_conn(usecases::settings::get_all)
}

/// Fija (o reemplaza) una preferencia en el vault.
#[tauri::command]
fn settings_set(vault: tauri::State<Vault>, key: String, value: String) -> AppResult<()> {
    vault.with_conn(|c| usecases::settings::set(c, &key, &value))
}

#[tauri::command]
fn ssh_import_preview() -> AppResult<Vec<ImportedHost>> {
    let path = usecases::ssh_import::default_config_path()
        .ok_or_else(|| AppError::Other("no se encontró el HOME del usuario".into()))?;
    let file = std::fs::File::open(&path)?;
    let mut reader = std::io::BufReader::new(file);
    usecases::ssh_import::parse_hosts(&mut reader)
}

/// Candidatos a importar hallados bajo `~/.ssh` (config y `config.d/**`).
#[tauri::command]
fn ssh_import_candidates() -> Vec<domain::CandidateFile> {
    usecases::ssh_import::discover_candidates()
}

/// Parsea un archivo de config SSH concreto (sugerencia o soltado por el usuario).
#[tauri::command]
fn ssh_import_parse_file(path: String) -> AppResult<Vec<ImportedHost>> {
    usecases::ssh_import::parse_file(&PathBuf::from(path))
}

/// Crea un nodo (con credencial SSH) por cada host seleccionado en el diagrama
/// destino, con auto-layout en rejilla. Devuelve los nodos creados.
#[tauri::command]
fn ssh_import_hosts(
    vault: tauri::State<Vault>,
    map_id: String,
    context_id: String,
    hosts: Vec<ImportedHost>,
) -> AppResult<Vec<Node>> {
    vault.with_conn(|c| usecases::ssh_import::import_hosts(c, &map_id, &context_id, &hosts))
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
fn node_search(vault: tauri::State<Vault>, query: String) -> AppResult<Vec<SearchHit>> {
    vault.with_conn(|c| workspace::search_nodes(c, &query))
}

/// Comprueba si el nodo responde (TCP) en su puerto de servicio para el contexto.
#[tauri::command]
async fn node_health(
    vault: tauri::State<'_, Vault>,
    node_id: String,
    context_id: Option<String>,
) -> AppResult<usecases::health::HealthStatus> {
    // BD con cliente de sonda (Mongo con SRV/TLS/auth, Redis con PING autenticado):
    // se comprueba con su cliente, que da mejor señal que una sonda TCP. Solo si el
    // cliente está en el PATH; si no, cae a la sonda TCP (no requiere cliente).
    let client_ping = vault
        .with_conn(|c| usecases::health::client_ping_command(c, &node_id, context_id.as_deref()))?
        .filter(|cmd| usecases::connections::program_in_path(&cmd.program));

    let status = if let Some(cmd) = client_ping {
        tauri::async_runtime::spawn_blocking(move || usecases::health::run_client_ping(cmd))
            .await
            .map_err(|e| AppError::Other(format!("sonda de salud interrumpida: {e}")))?
    } else {
        // Resolución rápida bajo el lock (antes de cualquier await, no cruza hilos).
        let target = vault
            .with_conn(|c| usecases::health::resolve_probe(c, &node_id, context_id.as_deref()))?;
        match target {
            None => usecases::health::HealthStatus::simple("noTarget", 0),
            // La sonda TCP bloquea hasta 3s: fuera del hilo principal para no congelar la UI.
            Some((host, port)) => {
                tauri::async_runtime::spawn_blocking(move || usecases::health::tcp_probe(&host, port))
                    .await
                    .map_err(|e| AppError::Other(format!("sonda de salud interrumpida: {e}")))?
            }
        }
    };

    // Una comprobación fallida deja rastro en el log de soporte (sin host/IP).
    usecases::health::log_probe_result(&node_id, &status);
    Ok(status)
}

/// Lee el sondeo del equipo (hostname/SO/kernel…) que la conexión SSH dejó en un
/// archivo local; si está listo lo fusiona en las propiedades del nodo y lo
/// devuelve. `None` mientras el sondeo aún no termina (el frontend reintenta).
#[tauri::command]
fn facts_poll(
    vault: tauri::State<Vault>,
    node_id: String,
) -> AppResult<Option<std::collections::HashMap<String, String>>> {
    vault.with_conn(|c| usecases::facts::poll(c, &node_id))
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
fn node_set_endpoints(
    vault: tauri::State<Vault>,
    id: String,
    endpoints: HashMap<String, String>,
) -> AppResult<()> {
    vault.with_conn(|c| workspace::node_set_endpoints(c, &id, &endpoints))
}

#[tauri::command]
fn node_delete(vault: tauri::State<Vault>, id: String) -> AppResult<()> {
    vault.with_conn(|c| workspace::node_delete(c, &id))
}

// --- Contextos de acceso (puntos de vista de red) ---------------------------

#[tauri::command]
fn context_list(vault: tauri::State<Vault>) -> AppResult<Vec<AccessContext>> {
    vault.with_conn(usecases::contexts::context_list)
}

#[tauri::command]
fn context_create(vault: tauri::State<Vault>, name: String) -> AppResult<AccessContext> {
    vault.with_conn(|c| usecases::contexts::context_create(c, &name))
}

#[tauri::command]
fn context_rename(vault: tauri::State<Vault>, id: String, name: String) -> AppResult<()> {
    vault.with_conn(|c| usecases::contexts::context_rename(c, &id, &name))
}

#[tauri::command]
fn context_delete(vault: tauri::State<Vault>, id: String) -> AppResult<()> {
    vault.with_conn(|c| usecases::contexts::context_delete(c, &id))
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
    context_id: Option<String>,
) -> AppResult<()> {
    // Confianza de plantillas: a nivel de máquina y por ruta del vault (no viaja en
    // el `.karto`). Un vault importado no está autorizado hasta que el usuario lo
    // confirme; entonces `connect_node` devuelve `TemplateConfirmationRequired`.
    let trusted = match vault.status().path {
        Some(path) => open_app_store()
            .and_then(|s| app_store::vault_templates_trusted(&s, &path))
            .unwrap_or(false),
        None => false,
    };
    vault.with_conn(|c| {
        connections::connect_node(
            c,
            &node_id,
            credential_id.as_deref(),
            context_id.as_deref(),
            trusted,
        )
    })
}

/// Autoriza (en esta máquina) la ejecución de las plantillas de conexión que trae
/// el vault abierto. El frontend lo llama tras mostrar el comando y que el usuario
/// confirme, en respuesta a un error `TemplateConfirmationRequired`.
#[tauri::command]
fn vault_trust_templates(vault: tauri::State<Vault>) -> AppResult<()> {
    let path = vault
        .status()
        .path
        .ok_or(AppError::NoVaultOpen)?;
    let store = open_app_store()?;
    app_store::trust_vault_templates(&store, &path)
}

/// Abre en el navegador la URL de administración del nodo (`url_admin`/`url`).
/// No usa credenciales: es un simple lanzamiento de enlace.
#[tauri::command]
fn open_node_url(vault: tauri::State<Vault>, node_id: String) -> AppResult<()> {
    vault.with_conn(|c| connections::open_node_url(c, &node_id))
}

/// Abre un enlace externo (http/https) en el navegador del sistema. Sin vault:
/// se usa para los enlaces fijos de la app (sección "Acerca de").
#[tauri::command]
fn open_external_url(url: String) -> AppResult<()> {
    connections::open_external_url(&url)
}

/// Aprovisiona acceso por llave SSH para una credencial: genera una llave
/// ed25519, la copia al servidor con `ssh-copy-id` (el usuario teclea la
/// contraseña una vez en la terminal) y registra la llave en la credencial.
/// Devuelve la ruta de la llave privada generada.
#[tauri::command]
fn ssh_provision_key(
    vault: tauri::State<Vault>,
    node_id: String,
    credential_id: String,
    context_id: Option<String>,
    set_default_key: bool,
    store_in_vault: bool,
) -> AppResult<String> {
    // Las opciones no se aplican aquí: viajan de vuelta en `ssh_provision_poll`,
    // que es quien toca la credencial una vez confirmada la copia.
    let _ = (set_default_key, store_in_vault);
    vault.with_conn(|c| {
        usecases::ssh_provision::provision(c, &node_id, &credential_id, context_id.as_deref())
    })
}

/// Comprueba si el aprovisionamiento lanzado antes ya terminó bien y, en ese
/// caso, aplica los cambios pedidos sobre la credencial. Devuelve `true` cuando
/// los aplica. El frontend lo sondea igual que hace con los facts.
#[tauri::command]
fn ssh_provision_poll(
    vault: tauri::State<Vault>,
    credential_id: String,
    set_default_key: bool,
    store_in_vault: bool,
) -> AppResult<bool> {
    let opts = usecases::ssh_provision::ProvisionOptions {
        set_default_key,
        store_in_vault,
    };
    vault.with_conn(|c| {
        usecases::ssh_provision::commit_if_provisioned(c, &credential_id, opts)
    })
}

/// Abre la SQLite de estado de app (recientes/shortcuts), importando de paso el
/// `recent.json` heredado (una vez; tras importarlo se renombra y no se repite).
fn open_app_store() -> AppResult<rusqlite::Connection> {
    let conn = app_store::open()?;
    if let Some(dir) = app_store::config_dir() {
        app_store::import_legacy_recents(&conn, &dir.join("recent.json"))?;
    }
    Ok(conn)
}

/// Registra un vault como reciente (best-effort: si falla el IO, no rompe la
/// apertura del vault, que es lo importante).
fn remember_recent(path: &str) {
    if let Ok(conn) = open_app_store() {
        let _ = app_store::remember(&conn, path, app_store::now_millis());
    }
}

/// Lista los vaults recientes, purgando de paso los que ya no existen en disco.
#[tauri::command]
fn recents_list() -> AppResult<Vec<RecentVault>> {
    let conn = open_app_store()?;
    app_store::prune_missing(&conn, |p| std::path::Path::new(p).exists())?;
    app_store::list_recents(&conn)
}

/// Elimina un vault de la lista de recientes; devuelve la lista actualizada.
#[tauri::command]
fn recents_forget(path: String) -> AppResult<Vec<RecentVault>> {
    let conn = open_app_store()?;
    app_store::forget(&conn, &path)?;
    app_store::list_recents(&conn)
}

// --- Plantillas de conexión ---
// Biblioteca a nivel de app (app_store) + ligado por-vault (launch_templates).

/// Lista la biblioteca de plantillas (defaults + del usuario).
#[tauri::command]
fn template_list() -> AppResult<Vec<app_store::Template>> {
    let conn = open_app_store()?;
    app_store::list_templates(&conn)
}

/// Crea o actualiza una plantilla en la biblioteca. `id = None` crea una nueva.
#[tauri::command]
fn template_upsert(
    id: Option<String>,
    name: String,
    connection: String,
    command: String,
) -> AppResult<()> {
    let conn = open_app_store()?;
    app_store::upsert_template(&conn, id.as_deref(), &name, &connection, &command)
}

/// Elimina una plantilla de la biblioteca.
#[tauri::command]
fn template_delete(id: String) -> AppResult<()> {
    let conn = open_app_store()?;
    app_store::delete_template(&conn, &id)
}

/// Liga una plantilla al vault abierto: **copia** su comando al `launch_templates`
/// del vault (portable; viaja con el `.karto`). Sustituye el override anterior de
/// ese tipo de conexión si lo había.
#[tauri::command]
fn template_link_to_vault(vault: tauri::State<Vault>, id: String) -> AppResult<()> {
    let store = open_app_store()?;
    let tpl = app_store::get_template(&store, &id)?
        .ok_or_else(|| AppError::Other("plantilla no encontrada".into()))?;
    vault.with_conn(|c| usecases::templates::set_vault_override(c, &tpl.connection, &tpl.command))?;
    // Ligar una plantilla es una acción local deliberada del usuario → confía en las
    // plantillas de este vault en esta máquina (así el connect no pide confirmación).
    if let Some(path) = vault.status().path {
        let _ = app_store::trust_vault_templates(&store, &path);
    }
    Ok(())
}

/// Lista los overrides de plantilla ligados en el vault abierto.
#[tauri::command]
fn template_vault_list(vault: tauri::State<Vault>) -> AppResult<Vec<usecases::templates::VaultTemplate>> {
    vault.with_conn(usecases::templates::list_vault_overrides)
}

/// Desliga (borra) el override del vault para un tipo de conexión.
#[tauri::command]
fn template_vault_unlink(vault: tauri::State<Vault>, connection: String) -> AppResult<()> {
    vault.with_conn(|c| usecases::templates::delete_vault_override(c, &connection))
}

// --- Scripts remotos ---
// Biblioteca a nivel de app (app_store) + ejecución por SSH sobre los equipos de
// un diagrama, con streaming de salida por `tauri::ipc::Channel`.

/// Lista la biblioteca de scripts (semillas + del usuario).
#[tauri::command]
fn script_list() -> AppResult<Vec<app_store::Script>> {
    let conn = open_app_store()?;
    app_store::list_scripts(&conn)
}

/// Crea o actualiza un script en la biblioteca. `id = None` crea uno nuevo.
#[tauri::command]
fn script_upsert(
    id: Option<String>,
    name: String,
    body: String,
    interpreter: String,
) -> AppResult<()> {
    let conn = open_app_store()?;
    app_store::upsert_script(&conn, id.as_deref(), &name, &body, &interpreter)
}

/// Elimina un script de la biblioteca.
#[tauri::command]
fn script_delete(id: String) -> AppResult<()> {
    let conn = open_app_store()?;
    app_store::delete_script(&conn, &id)
}

/// Lista las carpetas de scripts.
#[tauri::command]
fn script_folder_list() -> AppResult<Vec<app_store::ScriptFolder>> {
    let conn = open_app_store()?;
    app_store::list_script_folders(&conn)
}

/// Crea una carpeta de scripts y devuelve su id.
#[tauri::command]
fn script_folder_create(name: String) -> AppResult<String> {
    let conn = open_app_store()?;
    app_store::create_script_folder(&conn, &name)
}

/// Renombra una carpeta de scripts.
#[tauri::command]
fn script_folder_rename(id: String, name: String) -> AppResult<()> {
    let conn = open_app_store()?;
    app_store::rename_script_folder(&conn, &id, &name)
}

/// Borra una carpeta de scripts (sus scripts quedan sueltos, no se borran).
#[tauri::command]
fn script_folder_delete(id: String) -> AppResult<()> {
    let conn = open_app_store()?;
    app_store::delete_script_folder(&conn, &id)
}

/// Mueve un script a una carpeta (`folderId = None` lo deja suelto).
#[tauri::command]
fn script_set_folder(id: String, folder_id: Option<String>) -> AppResult<()> {
    let conn = open_app_store()?;
    app_store::set_script_folder(&conn, &id, folder_id.as_deref())
}

/// Equipos de un diagrama con su elegibilidad para ejecutar scripts.
#[tauri::command]
fn script_targets(
    vault: tauri::State<Vault>,
    map_id: String,
) -> AppResult<Vec<usecases::scripts::ScriptTarget>> {
    vault.with_conn(|c| usecases::scripts::list_targets(c, &map_id))
}

/// Ejecuta un script sobre los equipos indicados, transmitiendo la salida por el
/// canal `on_event`. Resuelve todas las conexiones con el lock del vault una sola
/// vez y luego ejecuta sin tocar la DB (secuencial o en paralelo por equipo).
#[tauri::command]
fn scripts_run(
    vault: tauri::State<Vault>,
    node_ids: Vec<String>,
    body: String,
    interpreter: String,
    mode: String,
    context_id: Option<String>,
    on_event: tauri::ipc::Channel<usecases::scripts::RunEvent>,
) -> AppResult<()> {
    // Cada nodo resuelto: petición SSH (bash/python) o conexión de BD (Modelo B).
    enum Job {
        Ssh(AppResult<domain::ConnectionRequest>),
        Db(AppResult<usecases::scripts::DbConn>),
    }

    // 1. Resolución (necesita el vault): una pasada bajo el lock. El intérprete es
    // el mismo para todo el run, así que decide el pipeline.
    let is_db = usecases::scripts::is_db_engine(&interpreter);
    let resolved: Vec<(String, Job)> = vault.with_conn(|c| {
        Ok(node_ids
            .iter()
            .map(|id| {
                let job = if is_db {
                    Job::Db(usecases::scripts::resolve_db_target(
                        c,
                        id,
                        context_id.as_deref(),
                        &interpreter,
                    ))
                } else {
                    Job::Ssh(usecases::scripts::resolve_target(c, id, context_id.as_deref()))
                };
                (id.clone(), job)
            })
            .collect())
    })?;

    // 2. Ejecución (sin DB) en un **hilo de fondo**: el comando devuelve al
    // instante para no bloquear el hilo principal (la UI). La salida sigue
    // llegando por el `Channel` (Clone + Send). El frontend cierra el estado
    // "ejecutando" al recibir todos los eventos `done`.
    fn run_one(
        node_id: &str,
        job: Job,
        body: &str,
        interpreter: &str,
        ch: tauri::ipc::Channel<usecases::scripts::RunEvent>,
    ) {
        let emit = move |ev| {
            let _ = ch.send(ev);
        };
        match job {
            Job::Ssh(Ok(req)) => {
                usecases::scripts::run_target(&req, node_id, body, interpreter, &emit)
            }
            Job::Db(Ok(conn)) => {
                usecases::scripts::run_db_target(interpreter, &conn, node_id, body, &emit)
            }
            Job::Ssh(Err(e)) | Job::Db(Err(e)) => {
                usecases::scripts::emit_error(node_id, &e.to_string(), &emit)
            }
        }
    }

    std::thread::spawn(move || {
        if mode == "parallel" {
            let mut handles = Vec::new();
            for (node_id, job) in resolved {
                let ch = on_event.clone();
                let body = body.clone();
                let interpreter = interpreter.clone();
                handles.push(std::thread::spawn(move || {
                    run_one(&node_id, job, &body, &interpreter, ch)
                }));
            }
            for h in handles {
                let _ = h.join();
            }
        } else {
            for (node_id, job) in resolved {
                run_one(&node_id, job, &body, &interpreter, on_event.clone());
            }
        }
    });
    Ok(())
}

// --- Diagnóstico (log de soporte) ---
// Config a nivel de máquina (app_store), independiente del vault: el log lo
// escribe el backend en `~/.local/share/app.karto.desktop/karto.log`.

/// Clave del nivel de log en la config de app.
const LOG_LEVEL_KEY: &str = "logLevel";

/// Nivel actual del log de diagnóstico (`info`|`warning`|`error`). Si no está
/// fijado, cae al default (Warning).
#[tauri::command]
fn log_level_get() -> AppResult<String> {
    let conn = open_app_store()?;
    let level = app_store::get_config(&conn, LOG_LEVEL_KEY)?
        .as_deref()
        .and_then(usecases::diagnostics::LogLevel::from_str)
        .unwrap_or_else(usecases::diagnostics::level);
    Ok(level.as_str().to_string())
}

/// Cambia el nivel del log: lo persiste (machine-level) y lo aplica en caliente.
#[tauri::command]
fn log_level_set(level: String) -> AppResult<()> {
    let parsed = usecases::diagnostics::LogLevel::from_str(&level)
        .ok_or_else(|| AppError::Other(format!("nivel de log inválido: {level}")))?;
    let conn = open_app_store()?;
    app_store::set_config(&conn, LOG_LEVEL_KEY, parsed.as_str())?;
    usecases::diagnostics::set_level(parsed);
    Ok(())
}

/// Ruta absoluta del archivo de log (para mostrarla/copiarla en la UI).
#[tauri::command]
fn log_path_get() -> AppResult<String> {
    usecases::diagnostics::log_path()
        .map(|p| p.display().to_string())
        .ok_or_else(|| AppError::Other("no se pudo resolver la ruta del log".into()))
}

/// Abre en el gestor de archivos del SO la carpeta que contiene el log.
#[tauri::command]
fn open_log_dir() -> AppResult<()> {
    usecases::diagnostics::open_log_dir()
}

/// Directorio sugerido por defecto al crear un vault (el `home` del usuario).
/// El diálogo nativo lo pre-rellena, pero el usuario puede cambiarlo: la ruta
/// no es un control de seguridad (lo es el cifrado), así que se prioriza que sea
/// visible y fácil de respaldar.
#[tauri::command]
fn default_vault_dir(app: tauri::AppHandle) -> AppResult<String> {
    let home = app
        .path()
        .home_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(home.display().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            app.manage(VaultService::new(SqlcipherStore::new()));
            // Aplica el nivel de log guardado (machine-level) antes de operar; si
            // no hay app_store o valor, se queda el default (Warning).
            if let Ok(conn) = open_app_store() {
                if let Ok(Some(v)) = app_store::get_config(&conn, LOG_LEVEL_KEY) {
                    if let Some(level) = usecases::diagnostics::LogLevel::from_str(&v) {
                        usecases::diagnostics::set_level(level);
                    }
                }
            }
            // Encabezado de sesión con datos del equipo (siempre, sin filtrar nivel).
            usecases::diagnostics::record_startup();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            vault_status,
            vault_create,
            vault_unlock,
            vault_lock,
            vault_close,
            vault_rekey,
            vault_export,
            vault_export_subset,
            recents_list,
            recents_forget,
            template_list,
            template_upsert,
            template_delete,
            template_link_to_vault,
            template_vault_list,
            template_vault_unlink,
            script_list,
            script_upsert,
            script_delete,
            script_folder_list,
            script_folder_create,
            script_folder_rename,
            script_folder_delete,
            script_set_folder,
            script_targets,
            scripts_run,
            log_level_get,
            log_level_set,
            log_path_get,
            open_log_dir,
            default_vault_dir,
            export_write,
            settings_get,
            settings_set,
            ssh_import_preview,
            ssh_import_candidates,
            ssh_import_parse_file,
            ssh_import_hosts,
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
            node_search,
            facts_poll,
            node_health,
            node_create,
            node_set_position,
            node_set_parent,
            node_rename,
            node_set_properties,
            node_set_endpoints,
            node_delete,
            context_list,
            context_create,
            context_rename,
            context_delete,
            edge_create,
            edge_set_label,
            edge_set_style,
            edge_delete,
            credential_list,
            credential_upsert,
            credential_reveal,
            credential_delete,
            connect_node,
            vault_trust_templates,
            open_node_url,
            open_external_url,
            ssh_provision_key,
            ssh_provision_poll
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar la aplicación Karto");
}
