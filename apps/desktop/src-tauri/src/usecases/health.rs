//! Health check de nodos: comprueba si el equipo responde en su puerto de
//! servicio con una **conexión TCP** (no ICMP, así no requiere privilegios). El
//! host se deriva del contexto activo (endpoint) o del hostname, igual que al
//! conectar; el puerto, de la credencial predeterminada o del tipo. La elección
//! de puerto es un núcleo puro y testeable; la sonda TCP es el efecto.

use crate::error::AppResult;
use crate::usecases::connections;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(3);

/// Estado de salud de un nodo tras la sonda.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    /// `reachable` | `unreachable` | `unresolved` | `noTarget`.
    pub state: String,
    /// Puerto sondeado (0 si no había objetivo).
    pub port: u16,
    /// Latencia del connect en ms (solo si `reachable`).
    pub latency_ms: Option<u64>,
}

impl HealthStatus {
    pub fn simple(state: &str, port: u16) -> Self {
        Self { state: state.to_string(), port, latency_ms: None }
    }

    /// ¿Es un fallo de sondeo digno del log de soporte? `reachable` es éxito y
    /// `noTarget` es "nada que sondear" (no hay dirección configurada, no un fallo);
    /// solo `unreachable`/`unresolved` son comprobaciones que salieron mal.
    pub fn is_probe_failure(&self) -> bool {
        matches!(self.state.as_str(), "unreachable" | "unresolved")
    }
}

/// Deja rastro en el log de soporte cuando una comprobación de estado **no** salió
/// exitosa. Best-effort y sin datos sensibles: solo id de nodo, estado y puerto
/// (nunca host ni IP). Los éxitos y los `noTarget` no se registran para no ensuciar.
pub fn log_probe_result(node_id: &str, status: &HealthStatus) {
    if !status.is_probe_failure() {
        return;
    }
    crate::usecases::diagnostics::warn(
        "health",
        "probe_failed",
        &[
            ("nodeId", node_id),
            ("state", &status.state),
            ("port", &status.port.to_string()),
        ],
    );
}

/// Puerto de servicio por defecto según el tipo de credencial.
fn default_port(kind: &str) -> u16 {
    match kind {
        "rdp" => 3389,
        "vnc" => 5900,
        "web" => 443,
        _ => 22, // ssh y cualquier otro
    }
}

/// Elige el puerto a sondear: el de la credencial **predeterminada** (o, si no
/// hay, la primera); su `port` explícito si lo tiene, si no el del tipo; y 22
/// como último recurso cuando el nodo no tiene credenciales.
pub fn pick_port(creds: &[(String, Option<u16>, bool)]) -> u16 {
    let chosen = creds
        .iter()
        .find(|(_, _, is_default)| *is_default)
        .or_else(|| creds.first());
    match chosen {
        Some((kind, port, _)) => port.unwrap_or_else(|| default_port(kind)),
        None => 22,
    }
}

/// Sonda TCP con timeout. Distingue no-resolución de no-alcanzable. Es la parte
/// **lenta** (bloquea hasta `TIMEOUT`): el adaptador la corre fuera del hilo
/// principal para no congelar la UI.
pub fn tcp_probe(host: &str, port: u16) -> HealthStatus {
    let Ok(mut addrs) = (host, port).to_socket_addrs() else {
        return HealthStatus::simple("unresolved", port);
    };
    let Some(addr) = addrs.next() else {
        return HealthStatus::simple("unresolved", port);
    };
    let started = Instant::now();
    match TcpStream::connect_timeout(&addr, TIMEOUT) {
        Ok(_) => HealthStatus {
            state: "reachable".to_string(),
            port,
            latency_ms: Some(started.elapsed().as_millis() as u64),
        },
        Err(_) => HealthStatus::simple("unreachable", port),
    }
}

/// Deriva `(host, puerto)` de una URL para sondearla por TCP. Soporta esquema
/// http/https (puerto por defecto 80/443), puerto explícito `host:port`, y
/// descarta userinfo y ruta. Sin esquema asume https (443). `None` si no hay host.
pub fn parse_url_target(url: &str) -> Option<(String, u16)> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    // Esquema → puerto por defecto.
    let (rest, default_port) = if let Some(r) = url.strip_prefix("https://") {
        (r, 443)
    } else if let Some(r) = url.strip_prefix("http://") {
        (r, 80)
    } else if let Some(idx) = url.find("://") {
        (&url[idx + 3..], 443) // esquema desconocido: asume TLS
    } else {
        (url, 443) // sin esquema: asume https
    };
    // Corta la autoridad en la primera '/', '?' o '#'; descarta userinfo (…@host).
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    if host_port.is_empty() {
        return None;
    }
    // `host:port` explícito (solo si el puerto es numérico; ignora IPv6 sin []).
    if let Some((h, p)) = host_port.rsplit_once(':') {
        if let Ok(port) = p.parse::<u16>() {
            if !h.is_empty() {
                return Some((h.to_string(), port));
            }
        }
    }
    Some((host_port.to_string(), default_port))
}

/// Si el nodo es una BD cuyo cliente da mejor señal que una sonda TCP, arma su
/// comando de ping headless. Devuelve `None` (→ el caller usa la sonda TCP) cuando
/// el motor no tiene cliente de ping, o falta la credencial de BD. El caller decide
/// además si el cliente está instalado: si no lo está, también cae a TCP.
/// - **MongoDB**: `mongosh` entiende SRV/Atlas, TLS y auth, cosa que una sonda TCP
///   no puede resolver.
/// - **Redis**: `redis-cli PING` confirma que el servidor responde de verdad (no
///   solo que el puerto acepta conexiones) y respeta la auth.
pub fn client_ping_command(
    conn: &Connection,
    node_id: &str,
    context_id: Option<&str>,
) -> AppResult<Option<crate::usecases::scripts::DbCommand>> {
    use crate::usecases::scripts;
    let engine: Option<String> = conn
        .query_row(
            "SELECT value FROM node_properties WHERE node_id = ?1 AND key = 'gestor'",
            params![node_id],
            |r| r.get(0),
        )
        .optional()?;
    // Sin credencial de BD no hay ping posible → `resolve_db_target` falla y caemos
    // a TCP (Ok(None)).
    match engine.as_deref() {
        Some("mongodb") => Ok(scripts::resolve_db_target(conn, node_id, context_id, "mongodb")
            .ok()
            .map(|c| scripts::build_mongo_ping(&c))),
        Some("redis") => Ok(scripts::resolve_db_target(conn, node_id, context_id, "redis")
            .ok()
            .map(|c| scripts::build_redis_ping(&c))),
        _ => Ok(None),
    }
}

/// Ejecuta un comando de ping de cliente (p. ej. `mongosh`) de forma headless con
/// timeout. Éxito (exit 0) → `reachable` (con latencia); fallo/timeout →
/// `unreachable`. La parte **lenta**: la corre el adaptador fuera del hilo main.
pub fn run_client_ping(cmd: crate::usecases::scripts::DbCommand) -> HealthStatus {
    use std::process::{Command, Stdio};
    let started = Instant::now();
    let mut child = match Command::new(&cmd.program)
        .args(&cmd.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return HealthStatus::simple("unreachable", 0),
    };
    // El handshake SRV+TLS+auth tarda más que un TCP: margen de ~8s.
    let deadline = Duration::from_secs(8);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    HealthStatus {
                        state: "reachable".to_string(),
                        port: 0,
                        latency_ms: Some(started.elapsed().as_millis() as u64),
                    }
                } else {
                    HealthStatus::simple("unreachable", 0)
                };
            }
            Ok(None) => {
                if started.elapsed() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return HealthStatus::simple("unreachable", 0);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return HealthStatus::simple("unreachable", 0),
        }
    }
}

/// Resuelve el objetivo `(host, puerto)` a sondear leyendo el vault. Prioriza la
/// dirección real (endpoint del contexto o `hostname`); si no hay, cae a la URL
/// del nodo (apps web: deriva host+puerto del enlace). `None` si no hay ninguna
/// forma de sondearlo (→ `noTarget`). Es la parte **rápida** (solo DB).
pub fn resolve_probe(
    conn: &Connection,
    node_id: &str,
    context_id: Option<&str>,
) -> AppResult<Option<(String, u16)>> {
    if let Some(host) = connections::node_host(conn, node_id, context_id)? {
        let mut stmt =
            conn.prepare("SELECT kind, port, is_default FROM credentials WHERE node_id = ?1")?;
        let creds: Vec<(String, Option<u16>, bool)> = stmt
            .query_map(params![node_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?;
        return Ok(Some((host, pick_port(&creds))));
    }
    // Sin dirección real: usa la URL como objetivo (aplicaciones web).
    if let Some(url) = connections::node_url(conn, node_id)? {
        if let Some(target) = parse_url_target(&url) {
            return Ok(Some(target));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_port_prefers_default_credential() {
        let creds = vec![
            ("ssh".to_string(), Some(22), false),
            ("web".to_string(), Some(8443), true),
        ];
        assert_eq!(pick_port(&creds), 8443);
    }

    #[test]
    fn pick_port_infers_from_kind_when_no_explicit_port() {
        let creds = vec![("rdp".to_string(), None, true)];
        assert_eq!(pick_port(&creds), 3389);
    }

    #[test]
    fn pick_port_falls_back_to_22_without_credentials() {
        assert_eq!(pick_port(&[]), 22);
    }

    #[test]
    fn parse_url_https_default_port() {
        assert_eq!(
            parse_url_target("https://app.acme.io/panel"),
            Some(("app.acme.io".to_string(), 443))
        );
    }

    #[test]
    fn parse_url_http_default_port() {
        assert_eq!(
            parse_url_target("http://box.local"),
            Some(("box.local".to_string(), 80))
        );
    }

    #[test]
    fn parse_url_explicit_port_and_userinfo() {
        assert_eq!(
            parse_url_target("https://user@app.acme.io:8443/x?y=1"),
            Some(("app.acme.io".to_string(), 8443))
        );
    }

    #[test]
    fn parse_url_without_scheme_assumes_https() {
        assert_eq!(
            parse_url_target("acme.io"),
            Some(("acme.io".to_string(), 443))
        );
    }

    #[test]
    fn parse_url_empty_is_none() {
        assert_eq!(parse_url_target("   "), None);
    }

    #[test]
    fn is_probe_failure_only_for_unreachable_and_unresolved() {
        assert!(HealthStatus::simple("unreachable", 6379).is_probe_failure());
        assert!(HealthStatus::simple("unresolved", 22).is_probe_failure());
        // Éxito y "nada que sondear" no son fallos → no se registran.
        assert!(!HealthStatus::simple("reachable", 443).is_probe_failure());
        assert!(!HealthStatus::simple("noTarget", 0).is_probe_failure());
    }

    #[test]
    fn probe_unresolved_host_is_not_a_panic() {
        // Un host inválido no debe romper: se reporta como no resuelto.
        let s = tcp_probe("host.invalido.karto.local", 22);
        assert_eq!(s.state, "unresolved");
    }

    #[test]
    fn resolve_probe_without_host_is_none() {
        use crate::infra::migrations;
        use crate::usecases::workspace;
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        let map = workspace::map_create(&conn, "Red", None).unwrap();
        let node = workspace::node_create(&conn, &map.id, "server", "X", 0.0, 0.0).unwrap();
        // Sin dirección ni hostname → no hay objetivo que sondear (→ noTarget).
        assert!(resolve_probe(&conn, &node.id, None).unwrap().is_none());
    }
}
