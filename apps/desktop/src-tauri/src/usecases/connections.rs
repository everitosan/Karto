//! Casos de uso de conexión (SSH/VNC/RDP/web).
//!
//! Tres responsabilidades separadas para poder probar sin tocar el sistema:
//!   1. **Armado del comando** (funciones puras): dado un `ConnectionRequest` y
//!      el SO, devuelven el `LaunchSpec` (programa + argumentos) a ejecutar.
//!   2. **Resolución desde el vault** (`resolve`): lee las propiedades del nodo y
//!      los datos de la credencial elegida (host, usuario, puerto, llave, opciones).
//!   3. **Lanzamiento** (`connect_node`, con efecto): arranca el proceso. Con SSH
//!      por contraseña, `ssh` la pide de forma interactiva en la terminal; Karto
//!      no intermedia el secreto al conectar (se guarda solo para copiar/documentar).

use crate::domain::{ConnectionKind, ConnectionRequest, Os};
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection, OptionalExtension};

/// Comando listo para lanzar: programa + argumentos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl LaunchSpec {
    fn plain(program: &str, args: Vec<String>) -> Self {
        Self {
            program: program.to_string(),
            args,
        }
    }
}

// --- Detección de terminal (Linux) ------------------------------------------

/// Terminal soportada: `program` es el ejecutable y `exec` el/los flag(s) que
/// preceden al comando a correr dentro de ella (p. ej. `gnome-terminal --`,
/// `konsole -e`, `kitty` sin flag).
pub struct TerminalDef {
    pub program: &'static str,
    pub exec: &'static [&'static str],
}

/// Terminales de Linux en orden de preferencia.
pub const LINUX_TERMINALS: &[TerminalDef] = &[
    TerminalDef { program: "x-terminal-emulator", exec: &["-e"] },
    TerminalDef { program: "gnome-terminal", exec: &["--"] },
    TerminalDef { program: "konsole", exec: &["-e"] },
    TerminalDef { program: "kitty", exec: &[] },
    TerminalDef { program: "alacritty", exec: &["-e"] },
    TerminalDef { program: "xterm", exec: &["-e"] },
];

/// Primera terminal disponible según el predicado `exists` (inyectable en test).
pub fn detect_terminal(
    terminals: &[TerminalDef],
    exists: impl Fn(&str) -> bool,
) -> Option<&TerminalDef> {
    terminals.iter().find(|t| exists(t.program))
}

/// ¿Hay un ejecutable con ese nombre en el `PATH`? (sin dependencias externas).
pub fn program_in_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join(name);
        std::fs::metadata(&candidate).map(|m| m.is_file()).unwrap_or(false)
    })
}

/// Envuelve un comando interno en la terminal detectada.
/// `konsole -e ssh user@host` → `LaunchSpec{ program: "konsole", args: ["-e","ssh","user@host"] }`.
pub fn wrap_in_terminal(term: &TerminalDef, inner: &[String]) -> LaunchSpec {
    let mut args: Vec<String> = term.exec.iter().map(|s| s.to_string()).collect();
    args.extend_from_slice(inner);
    LaunchSpec::plain(term.program, args)
}

/// Escapa un argumento para incrustarlo con seguridad en una línea de shell POSIX.
fn shell_quote(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }
    let safe = arg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c));
    if safe {
        arg.to_string()
    } else {
        // Comillas simples; se cierran, se escapa la ' literal y se reabren.
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

/// Une un argv en una línea de shell segura (para pasarla a `bash -c`).
fn to_shell_line(argv: &[String]) -> String {
    argv.iter()
        .map(|a| shell_quote(a))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Envuelve el comando interno en `bash -c "<cmd>; pausa"` para que la terminal
/// **no se cierre** al terminar o fallar (así se ven los errores de `ssh`) y para
/// no depender de que el ejecutable exista: `bash` siempre está, lo que evita que
/// emuladores como konsole crasheen al no poder crear la sesión (error Qt de
/// "null widget").
pub fn hold_wrapper(inner: &[String]) -> Vec<String> {
    let line = to_shell_line(inner);
    let script = format!(
        "{line}; status=$?; echo; \
         read -n1 -s -r -p \"Conexión finalizada (código $status). Pulsa una tecla para cerrar…\"; echo"
    );
    vec!["bash".to_string(), "-c".to_string(), script]
}

// --- Armado de comandos (puro) ----------------------------------------------

/// Destino `user@host` o `host` a secas si no hay usuario.
fn ssh_destination(req: &ConnectionRequest) -> String {
    match &req.user {
        Some(user) => format!("{user}@{}", req.host),
        None => req.host.clone(),
    }
}

/// Comando SSH interno (el que corre dentro de la terminal). Con llave usa `-i`;
/// con contraseña, `ssh` la pide de forma interactiva en la terminal (el usuario
/// la teclea). Karto no intermedia la contraseña al conectar.
pub fn ssh_inner_command(req: &ConnectionRequest) -> Vec<String> {
    let mut cmd = vec!["ssh".to_string()];
    if let Some(key) = &req.key_path {
        cmd.push("-i".into());
        cmd.push(key.clone());
    }
    if let Some(port) = req.port {
        cmd.push("-p".into());
        cmd.push(port.to_string());
    }
    // Opciones extra del usuario: cada una entra como `-o <opción>` (un solo
    // argv por opción, así valores con espacios —p. ej. ProxyCommand— no rompen).
    for opt in &req.ssh_options {
        cmd.push("-o".into());
        cmd.push(opt.clone());
    }
    cmd.push(ssh_destination(req));
    cmd
}

/// Normaliza el texto libre de opciones SSH: una opción por línea, recortando
/// espacios y descartando líneas vacías o comentarios (`#`).
pub fn parse_ssh_options(raw: Option<&str>) -> Vec<String> {
    raw.map(|text| {
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

/// Comando final de SSH: envuelve el comando interno en la terminal del SO.
/// En Linux requiere una terminal detectada; en mac/Windows se delega en la
/// terminal por defecto del sistema (Fase 3 posterior afinará mac/Windows).
pub fn build_ssh(req: &ConnectionRequest, terminal: Option<&TerminalDef>) -> AppResult<LaunchSpec> {
    let held = hold_wrapper(&ssh_inner_command(req));
    let term = terminal.ok_or_else(|| {
        AppError::Other(
            "no se encontró una terminal soportada (instala gnome-terminal, konsole, kitty…)"
                .into(),
        )
    })?;
    Ok(wrap_in_terminal(term, &held))
}

/// Abre una URL en el navegador por defecto según el SO.
pub fn build_open_url(os: Os, url: &str) -> LaunchSpec {
    match os {
        Os::Linux => LaunchSpec::plain("xdg-open", vec![url.to_string()]),
        Os::Macos => LaunchSpec::plain("open", vec![url.to_string()]),
        // `start` necesita un primer argumento de título (vacío) por su sintaxis.
        Os::Windows => LaunchSpec::plain(
            "cmd",
            vec!["/C".into(), "start".into(), String::new(), url.to_string()],
        ),
    }
}

/// Comando de cliente VNC (Linux, `vncviewer`). La contraseña VNC automática
/// requiere un archivo de formato propio del cliente y se abordará después;
/// por ahora abre el visor apuntando a `host:puerto`.
pub fn build_vnc(req: &ConnectionRequest) -> LaunchSpec {
    let port = req.port.unwrap_or(5900);
    LaunchSpec::plain("vncviewer", vec![format!("{}:{}", req.host, port)])
}

/// Arma el `LaunchSpec` completo para una petición, según su tipo y el SO.
pub fn plan(req: &ConnectionRequest, os: Os) -> AppResult<LaunchSpec> {
    match req.kind {
        ConnectionKind::Ssh => {
            let terminal = match os {
                Os::Linux => detect_terminal(LINUX_TERMINALS, program_in_path),
                // En mac/Windows aún no envolvemos en terminal explícita.
                _ => None,
            };
            build_ssh(req, terminal)
        }
        ConnectionKind::Web => {
            let url = req
                .url
                .as_deref()
                .ok_or_else(|| AppError::Other("el nodo no tiene URL de administración".into()))?;
            Ok(build_open_url(os, url))
        }
        ConnectionKind::Vnc => Ok(build_vnc(req)),
        ConnectionKind::Rdp => Err(AppError::Other(
            "las conexiones RDP aún no están soportadas".into(),
        )),
    }
}

// --- Resolución desde el vault ----------------------------------------------

/// Deriva el host de las propiedades del nodo: prioriza `ip`, luego `hostname`.
fn node_host(conn: &Connection, node_id: &str) -> AppResult<Option<String>> {
    let host = conn
        .query_row(
            "SELECT value FROM node_properties \
             WHERE node_id = ?1 AND key IN ('ip', 'hostname') \
             ORDER BY CASE key WHEN 'ip' THEN 0 ELSE 1 END LIMIT 1",
            params![node_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(host)
}

/// URL de administración del nodo (`url_admin` o `url`).
fn node_url(conn: &Connection, node_id: &str) -> AppResult<Option<String>> {
    let url = conn
        .query_row(
            "SELECT value FROM node_properties \
             WHERE node_id = ?1 AND key IN ('url_admin', 'url') \
             ORDER BY CASE key WHEN 'url_admin' THEN 0 ELSE 1 END LIMIT 1",
            params![node_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(url)
}

struct CredRow {
    kind: String,
    username: Option<String>,
    port: Option<u16>,
    key_path: Option<String>,
    options: Option<String>,
}

/// Lee la credencial elegida (o la predeterminada del nodo si `credential_id` es
/// `None`). No trae el secreto: al conectar no se usa.
fn load_credential(
    conn: &Connection,
    node_id: &str,
    credential_id: Option<&str>,
) -> AppResult<CredRow> {
    let row = match credential_id {
        Some(id) => conn
            .query_row(
                "SELECT kind, username, port, key_path, options FROM credentials \
                 WHERE id = ?1 AND node_id = ?2",
                params![id, node_id],
                map_cred_row,
            )
            .optional()?,
        None => conn
            .query_row(
                "SELECT kind, username, port, key_path, options FROM credentials \
                 WHERE node_id = ?1 ORDER BY is_default DESC, kind LIMIT 1",
                params![node_id],
                map_cred_row,
            )
            .optional()?,
    };
    row.ok_or_else(|| AppError::Other("el nodo no tiene credenciales para conectar".into()))
}

fn map_cred_row(r: &rusqlite::Row) -> rusqlite::Result<CredRow> {
    Ok(CredRow {
        kind: r.get(0)?,
        username: r.get(1)?,
        port: r.get::<_, Option<i64>>(2)?.map(|p| p as u16),
        key_path: r.get(3)?,
        options: r.get(4)?,
    })
}

/// Construye la petición de conexión leyendo el nodo y la credencial del vault.
pub fn resolve(
    conn: &Connection,
    node_id: &str,
    credential_id: Option<&str>,
) -> AppResult<ConnectionRequest> {
    let cred = load_credential(conn, node_id, credential_id)?;
    let kind = ConnectionKind::from_str(&cred.kind)
        .ok_or_else(|| AppError::Other(format!("tipo de conexión desconocido: {}", cred.kind)))?;

    match kind {
        ConnectionKind::Web => {
            let url = node_url(conn, node_id)?
                .ok_or_else(|| AppError::Other("el nodo no tiene URL de administración".into()))?;
            Ok(ConnectionRequest {
                kind,
                user: cred.username,
                host: String::new(),
                port: cred.port,
                key_path: None,
                url: Some(url),
                ssh_options: Vec::new(),
            })
        }
        _ => {
            let host = node_host(conn, node_id)?.ok_or_else(|| {
                AppError::Other("el nodo no tiene IP ni hostname para conectar".into())
            })?;
            Ok(ConnectionRequest {
                kind,
                user: cred.username,
                host,
                port: cred.port,
                key_path: cred.key_path,
                url: None,
                ssh_options: parse_ssh_options(cred.options.as_deref()),
            })
        }
    }
}

// --- Lanzamiento (efecto) ---------------------------------------------------

/// Resuelve la conexión y lanza el proceso correspondiente (terminal con `ssh`,
/// navegador con la URL, o visor VNC). Con SSH por contraseña, `ssh` la pide de
/// forma interactiva en la terminal; Karto no maneja el secreto al conectar.
pub fn connect_node(
    conn: &Connection,
    node_id: &str,
    credential_id: Option<&str>,
) -> AppResult<()> {
    let req = resolve(conn, node_id, credential_id)?;
    let spec = plan(&req, Os::current())?;
    std::process::Command::new(&spec.program)
        .args(&spec.args)
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::Other(format!("no se pudo lanzar la conexión: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::migrations;

    fn req_ssh_key() -> ConnectionRequest {
        ConnectionRequest {
            kind: ConnectionKind::Ssh,
            user: Some("root".into()),
            host: "10.0.0.5".into(),
            port: Some(2222),
            key_path: Some("/home/me/.ssh/id_ed25519".into()),
            url: None,
            ssh_options: Vec::new(),
        }
    }

    #[test]
    fn ssh_key_inner_command_has_key_port_and_dest() {
        assert_eq!(
            ssh_inner_command(&req_ssh_key()),
            vec!["ssh", "-i", "/home/me/.ssh/id_ed25519", "-p", "2222", "root@10.0.0.5"]
        );
    }

    #[test]
    fn ssh_options_are_injected_as_dash_o_before_destination() {
        let req = ConnectionRequest {
            ssh_options: vec![
                "ServerAliveInterval=60".into(),
                "ConnectTimeout=10".into(),
            ],
            ..req_ssh_key()
        };
        assert_eq!(
            ssh_inner_command(&req),
            vec![
                "ssh",
                "-i",
                "/home/me/.ssh/id_ed25519",
                "-p",
                "2222",
                "-o",
                "ServerAliveInterval=60",
                "-o",
                "ConnectTimeout=10",
                "root@10.0.0.5"
            ]
        );
    }

    #[test]
    fn parse_ssh_options_trims_and_skips_blank_and_comments() {
        let raw = "  ServerAliveInterval=60 \n\n# comentario\nProxyJump bastion\n";
        assert_eq!(
            parse_ssh_options(Some(raw)),
            vec!["ServerAliveInterval=60", "ProxyJump bastion"]
        );
        assert!(parse_ssh_options(None).is_empty());
        assert!(parse_ssh_options(Some("   \n # x")).is_empty());
    }

    #[test]
    fn ssh_without_user_uses_bare_host() {
        let req = ConnectionRequest {
            user: None,
            host: "server.local".into(),
            port: None,
            key_path: None,
            ..req_ssh_key()
        };
        assert_eq!(ssh_inner_command(&req), vec!["ssh", "server.local"]);
    }

    #[test]
    fn ssh_password_is_interactive_no_sshpass_wrapper() {
        // Sin llave: `ssh` pide la contraseña en la terminal; nunca anteponemos sshpass.
        let req = ConnectionRequest { key_path: None, ..req_ssh_key() };
        assert_eq!(ssh_inner_command(&req), vec!["ssh", "-p", "2222", "root@10.0.0.5"]);
    }

    #[test]
    fn terminal_detection_picks_first_available() {
        let term = detect_terminal(LINUX_TERMINALS, |p| p == "konsole").unwrap();
        assert_eq!(term.program, "konsole");
        assert_eq!(term.exec, &["-e"]);
    }

    #[test]
    fn terminal_detection_none_when_missing() {
        assert!(detect_terminal(LINUX_TERMINALS, |_| false).is_none());
    }

    #[test]
    fn wrap_in_terminal_prefixes_exec_flag() {
        let term = TerminalDef { program: "konsole", exec: &["-e"] };
        let spec = wrap_in_terminal(&term, &["ssh".into(), "host".into()]);
        assert_eq!(spec.program, "konsole");
        assert_eq!(spec.args, vec!["-e", "ssh", "host"]);
    }

    #[test]
    fn build_ssh_requires_terminal_on_linux() {
        assert!(build_ssh(&req_ssh_key(), None).is_err());
    }

    #[test]
    fn build_ssh_wraps_in_bash_hold() {
        let term = TerminalDef { program: "kitty", exec: &[] };
        let spec = build_ssh(&req_ssh_key(), Some(&term)).unwrap();
        // kitty no lleva flag de exec: argv = ["bash", "-c", "<script>"].
        assert_eq!(spec.args[0], "bash");
        assert_eq!(spec.args[1], "-c");
        let script = &spec.args[2];
        assert!(script.contains("ssh -i /home/me/.ssh/id_ed25519 -p 2222 root@10.0.0.5"));
        assert!(script.contains("Pulsa una tecla")); // se mantiene abierta
    }

    #[test]
    fn build_ssh_with_terminal_exec_flag_prefixes_it() {
        let term = TerminalDef { program: "konsole", exec: &["-e"] };
        let spec = build_ssh(&req_ssh_key(), Some(&term)).unwrap();
        assert_eq!(spec.program, "konsole");
        assert_eq!(spec.args[0], "-e");
        assert_eq!(spec.args[1], "bash");
    }

    #[test]
    fn hold_wrapper_quotes_and_pauses() {
        let held = hold_wrapper(&["ssh".into(), "user@host name".into()]);
        assert_eq!(held[0], "bash");
        assert_eq!(held[1], "-c");
        // El argumento con espacio va entre comillas simples.
        assert!(held[2].contains("ssh 'user@host name'"));
        assert!(held[2].contains("read -n1"));
    }

    #[test]
    fn shell_quote_leaves_safe_and_wraps_unsafe() {
        assert_eq!(shell_quote("root@10.0.0.5"), "root@10.0.0.5");
        assert_eq!(shell_quote("/home/me/.ssh/id_ed25519"), "/home/me/.ssh/id_ed25519");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn open_url_per_os() {
        assert_eq!(build_open_url(Os::Linux, "http://x").program, "xdg-open");
        assert_eq!(build_open_url(Os::Macos, "http://x").program, "open");
        assert_eq!(build_open_url(Os::Windows, "http://x").program, "cmd");
    }

    #[test]
    fn plan_web_uses_url() {
        let req = ConnectionRequest {
            kind: ConnectionKind::Web,
            user: None,
            host: String::new(),
            port: None,
            key_path: None,
            url: Some("https://panel.local".into()),
            ssh_options: Vec::new(),
        };
        assert_eq!(plan(&req, Os::Linux).unwrap().args, vec!["https://panel.local"]);
    }

    #[test]
    fn plan_rdp_not_supported_yet() {
        let req = ConnectionRequest { kind: ConnectionKind::Rdp, ..req_ssh_key() };
        assert!(plan(&req, Os::Linux).is_err());
    }

    // --- Resolución desde el vault ---

    fn seed() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO maps (id, name) VALUES ('m1', 'Mapa')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO nodes (id, map_id, kind, label) VALUES ('n1', 'm1', 'server', 'Web')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn resolve_ssh_uses_ip_and_default_credential() {
        let conn = seed();
        conn.execute(
            "INSERT INTO node_properties (node_id, key, value) VALUES ('n1','ip','10.0.0.9'),('n1','hostname','web.local')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO credentials (id, node_id, kind, username, secret, port, is_default, options) \
             VALUES ('c1','n1','ssh','root','pw',22,1,'ServerAliveInterval=60\nProxyJump bastion')",
            [],
        )
        .unwrap();

        let req = resolve(&conn, "n1", None).unwrap();
        assert_eq!(req.kind, ConnectionKind::Ssh);
        assert_eq!(req.host, "10.0.0.9"); // ip gana a hostname
        assert_eq!(req.user.as_deref(), Some("root"));
        assert_eq!(req.port, Some(22));
        assert_eq!(req.ssh_options, vec!["ServerAliveInterval=60", "ProxyJump bastion"]);
    }

    #[test]
    fn resolve_falls_back_to_hostname_when_no_ip() {
        let conn = seed();
        conn.execute(
            "INSERT INTO node_properties (node_id, key, value) VALUES ('n1','hostname','web.local')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO credentials (id, node_id, kind, is_default) VALUES ('c1','n1','ssh',1)",
            [],
        )
        .unwrap();
        assert_eq!(resolve(&conn, "n1", None).unwrap().host, "web.local");
    }

    #[test]
    fn resolve_web_reads_url_and_ignores_host() {
        let conn = seed();
        conn.execute(
            "INSERT INTO node_properties (node_id, key, value) VALUES ('n1','url_admin','https://p.local')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO credentials (id, node_id, kind, is_default) VALUES ('c1','n1','web',1)",
            [],
        )
        .unwrap();
        let req = resolve(&conn, "n1", None).unwrap();
        assert_eq!(req.kind, ConnectionKind::Web);
        assert_eq!(req.url.as_deref(), Some("https://p.local"));
        assert!(req.ssh_options.is_empty());
    }

    #[test]
    fn resolve_specific_credential_id() {
        let conn = seed();
        conn.execute(
            "INSERT INTO node_properties (node_id, key, value) VALUES ('n1','ip','1.2.3.4')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO credentials (id, node_id, kind, username, is_default) \
             VALUES ('c1','n1','ssh','root',1),('c2','n1','ssh','deploy',0)",
            [],
        )
        .unwrap();
        assert_eq!(resolve(&conn, "n1", Some("c2")).unwrap().user.as_deref(), Some("deploy"));
    }

    #[test]
    fn resolve_errors_without_credentials() {
        let conn = seed();
        assert!(resolve(&conn, "n1", None).is_err());
    }

    #[test]
    fn resolve_ssh_errors_without_host() {
        let conn = seed();
        conn.execute(
            "INSERT INTO credentials (id, node_id, kind, is_default) VALUES ('c1','n1','ssh',1)",
            [],
        )
        .unwrap();
        assert!(resolve(&conn, "n1", None).is_err());
    }
}
