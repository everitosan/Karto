//! Caso de uso: importar hosts desde `~/.ssh/config` para acelerar la adopción.
//! El parseo recibe un lector para testearlo sin el archivo real del usuario.

use crate::domain::{CandidateFile, ImportedHost, Node};
use crate::error::{AppError, AppResult};
use crate::usecases::workspace::{self, CredentialInput};
use rusqlite::Connection;
use ssh2_config::{ParseRule, SshConfig};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

/// Tamaño máximo (bytes) de un archivo a inspeccionar como candidato: evita
/// leer llaves/binarios grandes que casualmente vivan bajo `~/.ssh`.
const MAX_CANDIDATE_BYTES: u64 = 512 * 1024;

/// Parsea la configuración SSH desde cualquier lector con buffer.
pub fn parse_hosts(reader: &mut impl BufRead) -> AppResult<Vec<ImportedHost>> {
    let config = SshConfig::default()
        .parse(reader, ParseRule::ALLOW_UNKNOWN_FIELDS)
        .map_err(|e| AppError::Other(format!("no se pudo parsear ssh config: {e}")))?;

    let hosts = config
        .get_hosts()
        .iter()
        .filter_map(|host| {
            // Ignorar el bloque comodín "*".
            let alias = host
                .pattern
                .iter()
                .map(|p| p.pattern.clone())
                .find(|p| p != "*")?;
            let params = &host.params;
            Some(ImportedHost {
                alias,
                hostname: params.host_name.clone(),
                user: params.user.clone(),
                port: params.port,
                identity_file: params
                    .identity_file
                    .as_ref()
                    .and_then(|files| files.first())
                    .map(|p| p.display().to_string()),
            })
        })
        .collect();

    Ok(hosts)
}

/// El host destino de un import: `HostName` si existe, si no el propio alias
/// (que a menudo ya es la IP/dominio con el que uno se conecta).
pub fn host_target(host: &ImportedHost) -> &str {
    host.hostname.as_deref().unwrap_or(&host.alias)
}

/// Posiciones en rejilla para el auto-layout de los nodos importados. Función
/// pura (sin DB) para poder probar la disposición de forma aislada.
pub fn grid_layout(count: usize, cols: usize, dx: f64, dy: f64) -> Vec<(f64, f64)> {
    let cols = cols.max(1);
    (0..count)
        .map(|i| ((i % cols) as f64 * dx, (i / cols) as f64 * dy))
        .collect()
}

/// Crea un nodo `server` por cada host importado en el diagrama destino,
/// dispuestos en rejilla, con su credencial SSH predeterminada (usuario, puerto
/// y ruta de llave si venían en el `~/.ssh/config`). El secreto queda vacío: la
/// contraseña se teclea al conectar o se añade luego desde el panel. Devuelve
/// los nodos creados para que el frontend refresque el grafo.
pub fn import_hosts(
    conn: &Connection,
    map_id: &str,
    hosts: &[ImportedHost],
) -> AppResult<Vec<Node>> {
    let positions = grid_layout(hosts.len(), 4, 220.0, 140.0);
    let mut created = Vec::with_capacity(hosts.len());
    for (host, (x, y)) in hosts.iter().zip(positions) {
        let node = workspace::node_create(conn, map_id, "server", &host.alias, x, y)?;

        let mut props = HashMap::new();
        props.insert("hostname".to_string(), host_target(host).to_string());
        workspace::node_set_properties(conn, &node.id, &props)?;

        workspace::credential_upsert(
            conn,
            CredentialInput {
                id: None,
                node_id: &node.id,
                kind: "ssh",
                username: host.user.as_deref(),
                secret: None,
                port: host.port,
                key_path: host.identity_file.as_deref(),
                is_default: true,
                options: None,
            },
        )?;

        created.push(node);
    }
    Ok(created)
}

/// Ruta por defecto del archivo de configuración SSH del usuario actual.
pub fn default_config_path() -> Option<PathBuf> {
    ssh_dir().map(|dir| dir.join("config"))
}

/// Directorio `~/.ssh` del usuario actual.
pub fn ssh_dir() -> Option<PathBuf> {
    dirs_home().map(|home| home.join(".ssh"))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Parsea un archivo de configuración SSH concreto (sugerencia o soltado).
pub fn parse_file(path: &Path) -> AppResult<Vec<ImportedHost>> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    parse_hosts(&mut reader)
}

/// Heurística de contenido: ¿parece un archivo de config SSH? Basta una línea
/// (fuera de comentarios) que empiece por `Host`, `HostName` o `Match`.
pub fn looks_like_ssh_config(content: &str) -> bool {
    content.lines().any(|line| {
        let t = line.trim_start();
        if t.starts_with('#') {
            return false;
        }
        let lower = t.to_ascii_lowercase();
        lower.starts_with("host ")
            || lower.starts_with("hostname ")
            || lower.starts_with("match ")
    })
}

/// Descarte rápido por nombre: llaves, hosts conocidos, sockets, etc. El filtro
/// de contenido ya excluye llaves (empiezan por `-----BEGIN`), pero esto evita
/// leerlas siquiera.
pub fn is_probably_not_config(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    name.ends_with(".pub")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.ends_with(".ppk")
        || name.starts_with("id_")
        || name.starts_with("known_hosts")
        || name.starts_with("authorized_keys")
        || name == "agent"
        || name.ends_with(".sock")
}

/// Recorre `dir` de forma recursiva y devuelve los archivos que parecen config
/// SSH y contienen al menos un host. Separado de `discover_candidates` para
/// poder probarlo con un directorio temporal.
pub fn discover_in(dir: &Path) -> Vec<CandidateFile> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                stack.push(path);
                continue;
            }
            if !ft.is_file() || is_probably_not_config(&path) {
                continue;
            }
            let too_big = std::fs::metadata(&path)
                .map(|m| m.len() > MAX_CANDIDATE_BYTES)
                .unwrap_or(true);
            if too_big {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue; // no es UTF-8 → seguramente binario/llave
            };
            if !looks_like_ssh_config(&content) {
                continue;
            }
            let mut reader = std::io::BufReader::new(content.as_bytes());
            let hosts = parse_hosts(&mut reader).unwrap_or_default();
            if hosts.is_empty() {
                continue;
            }
            out.push(CandidateFile {
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string(),
                path: path.display().to_string(),
                host_count: hosts.len(),
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// Candidatos a importar bajo `~/.ssh` (incluye `config.d/**` recursivamente).
pub fn discover_candidates() -> Vec<CandidateFile> {
    match ssh_dir() {
        Some(dir) => discover_in(&dir),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn parses_a_basic_host_block() {
        let cfg = "Host web1\n  HostName 10.0.0.10\n  User deploy\n  Port 2200\n";
        let mut reader = BufReader::new(cfg.as_bytes());
        let hosts = parse_hosts(&mut reader).unwrap();
        let web1 = hosts.iter().find(|h| h.alias == "web1").unwrap();
        assert_eq!(web1.hostname.as_deref(), Some("10.0.0.10"));
        assert_eq!(web1.user.as_deref(), Some("deploy"));
        assert_eq!(web1.port, Some(2200));
    }

    #[test]
    fn grid_layout_wraps_by_columns() {
        let pos = grid_layout(5, 4, 100.0, 50.0);
        assert_eq!(pos.len(), 5);
        assert_eq!(pos[0], (0.0, 0.0));
        assert_eq!(pos[3], (300.0, 0.0)); // última de la primera fila
        assert_eq!(pos[4], (0.0, 50.0)); // envuelve a la segunda fila
    }

    #[test]
    fn host_target_prefers_hostname_then_alias() {
        let with = ImportedHost {
            alias: "web1".into(),
            hostname: Some("10.0.0.10".into()),
            user: None,
            port: None,
            identity_file: None,
        };
        let without = ImportedHost {
            hostname: None,
            ..with.clone()
        };
        assert_eq!(host_target(&with), "10.0.0.10");
        assert_eq!(host_target(&without), "web1");
    }

    #[test]
    fn import_hosts_creates_nodes_with_default_ssh_credential() {
        use crate::infra::migrations;
        use crate::usecases::workspace;

        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        let map = workspace::map_create(&conn, "Red", None).unwrap();

        let hosts = vec![
            ImportedHost {
                alias: "web1".into(),
                hostname: Some("10.0.0.10".into()),
                user: Some("deploy".into()),
                port: Some(2200),
                identity_file: Some("~/.ssh/id_ed25519".into()),
            },
            ImportedHost {
                alias: "db1".into(),
                hostname: None,
                user: None,
                port: None,
                identity_file: None,
            },
        ];

        let created = import_hosts(&conn, &map.id, &hosts).unwrap();
        assert_eq!(created.len(), 2);

        let graph = workspace::graph_load(&conn, &map.id).unwrap();
        assert_eq!(graph.nodes.len(), 2);

        let web = graph.nodes.iter().find(|n| n.label == "web1").unwrap();
        assert_eq!(web.kind, "server");
        assert_eq!(web.properties.get("hostname").map(String::as_str), Some("10.0.0.10"));

        let creds = workspace::credential_list(&conn, &web.id).unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].kind, "ssh");
        assert_eq!(creds[0].username.as_deref(), Some("deploy"));
        assert_eq!(creds[0].port, Some(2200));
        assert_eq!(creds[0].key_path.as_deref(), Some("~/.ssh/id_ed25519"));
        assert!(creds[0].is_default);

        // El host sin HostName usa el alias como destino.
        let db = graph.nodes.iter().find(|n| n.label == "db1").unwrap();
        assert_eq!(db.properties.get("hostname").map(String::as_str), Some("db1"));
    }

    #[test]
    fn looks_like_ssh_config_detects_host_blocks() {
        assert!(looks_like_ssh_config("Host web1\n  HostName 10.0.0.1\n"));
        assert!(looks_like_ssh_config("  hostname foo\n"));
        assert!(!looks_like_ssh_config("# Host commented out\n"));
        assert!(!looks_like_ssh_config("-----BEGIN OPENSSH PRIVATE KEY-----\n"));
        assert!(!looks_like_ssh_config("just some text\n"));
    }

    #[test]
    fn is_probably_not_config_skips_keys_and_known_hosts() {
        assert!(is_probably_not_config(Path::new("/home/u/.ssh/id_ed25519")));
        assert!(is_probably_not_config(Path::new("/home/u/.ssh/id_rsa.pub")));
        assert!(is_probably_not_config(Path::new("/home/u/.ssh/known_hosts")));
        assert!(!is_probably_not_config(Path::new("/home/u/.ssh/config")));
        assert!(!is_probably_not_config(Path::new("/home/u/.ssh/config.d/work")));
    }

    #[test]
    fn discover_in_finds_configs_recursively_and_skips_keys() {
        use std::fs;
        // Directorio temporal único que simula ~/.ssh.
        let base = std::env::temp_dir().join(format!("karto-ssh-test-{}", std::process::id()));
        let confd = base.join("config.d");
        fs::create_dir_all(&confd).unwrap();
        fs::write(base.join("config"), "Host main\n  HostName 10.0.0.1\n").unwrap();
        fs::write(confd.join("work"), "Host w1\n  HostName 10.1.0.1\nHost w2\n").unwrap();
        // Ruido que debe ignorarse.
        fs::write(base.join("id_ed25519"), "Host fake\n").unwrap(); // por nombre
        fs::write(base.join("known_hosts"), "example.com ssh-rsa AAAA\n").unwrap();
        fs::write(base.join("notes.txt"), "solo texto sin hosts\n").unwrap();

        let mut found = discover_in(&base);
        found.sort_by(|a, b| a.name.cmp(&b.name));
        let names: Vec<_> = found.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["config", "work"]);
        let work = found.iter().find(|c| c.name == "work").unwrap();
        assert_eq!(work.host_count, 2);

        fs::remove_dir_all(&base).ok();
    }
}
