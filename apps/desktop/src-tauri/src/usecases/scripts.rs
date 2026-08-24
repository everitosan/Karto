//! Ejecución de scripts remotos sobre los equipos de un diagrama.
//!
//! La **biblioteca** de scripts vive a nivel de app/máquina (`app_store::scripts`);
//! aquí está la ejecución: dado un `ConnectionRequest` ya resuelto desde el vault,
//! se corre el script por SSH **capturando** su salida (no-interactivo) y se
//! transmite **línea a línea** vía un callback (`emit`), para que la UI la muestre
//! en vivo por equipo.
//!
//! Captura sin secretos: solo se admite auth por **llave** (`BatchMode=yes` hace
//! que `ssh` falle rápido en vez de pedir contraseña). Un equipo solo-contraseña
//! produce un error "requiere llave" (se resuelve aprovisionando una con
//! `ssh_provision`). El cuerpo del script se pasa por **stdin** a `bash -s`, así
//! no hay que citar nada del contenido.
//!
//! `build_capture_argv` es puro (testeable); `run_target` es el borde con efecto.

use crate::domain::{ConnectionKind, ConnectionRequest, Credential};
use crate::error::{AppError, AppResult};
use rusqlite::{Connection, OptionalExtension};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Evento de progreso de una ejecución en un equipo. Se serializa hacia el
/// frontend (por `tauri::ipc::Channel`): `rename_all` renombra las **variantes**
/// (`Status`→`status`) y `rename_all_fields` los **campos** (`node_id`→`nodeId`);
/// sin este último los campos irían en snake_case y el frontend no los leería.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RunEvent {
    /// Transición de estado del equipo: `running` | `ok` | `error`.
    Status { node_id: String, status: String },
    /// Una línea de salida (stdout o stderr) del equipo.
    Line { node_id: String, line: String },
    /// Fin de la ejecución en el equipo. `error` si ni siquiera pudo arrancar.
    Done {
        node_id: String,
        exit_code: Option<i32>,
        error: Option<String>,
    },
}

/// Un equipo candidato (nodo de un diagrama) con sus **capacidades crudas**. El
/// frontend decide compatibilidad y selectabilidad según el intérprete del script
/// (shell → `ssh_key`; motor de BD → `kind==database` + `gestor` + `db_cred`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptTarget {
    pub node_id: String,
    pub label: String,
    /// Tipo de nodo del catálogo (server, vm, database…).
    pub kind: String,
    /// Gestor de BD (`postgresql`, `mysql`…) si el nodo lo declara.
    pub gestor: Option<String>,
    /// Tiene credencial SSH con llave (para bash/python).
    pub ssh_key: bool,
    /// Tiene credencial de BD (`kind='db'`) para los motores de BD.
    pub db_cred: bool,
}

/// Elige una credencial del nodo de un `kind` dado (prefiere la marcada por
/// defecto). `None` si el nodo no tiene ninguna de ese tipo.
fn pick_credential(conn: &Connection, node_id: &str, kind: &str) -> AppResult<Option<Credential>> {
    let creds = crate::usecases::workspace::credential_list(conn, node_id)?;
    Ok(creds
        .iter()
        .find(|c| c.kind == kind && c.is_default)
        .or_else(|| creds.iter().find(|c| c.kind == kind))
        .cloned())
}

/// Elige la credencial SSH del nodo (prefiere la marcada por defecto).
pub fn pick_ssh_credential(conn: &Connection, node_id: &str) -> AppResult<Option<Credential>> {
    pick_credential(conn, node_id, "ssh")
}

/// Lista los equipos de un diagrama con sus capacidades crudas (el frontend
/// decide compatibilidad/selectabilidad según el intérprete del script).
pub fn list_targets(conn: &Connection, map_id: &str) -> AppResult<Vec<ScriptTarget>> {
    let graph = crate::usecases::workspace::graph_load(conn, map_id)?;
    let mut targets = Vec::with_capacity(graph.nodes.len());
    for node in graph.nodes {
        let ssh_key = pick_ssh_credential(conn, &node.id)?
            .and_then(|c| c.key_path)
            .is_some_and(|k| !k.is_empty());
        let db_cred = pick_credential(conn, &node.id, "db")?.is_some();
        targets.push(ScriptTarget {
            node_id: node.id,
            label: node.label,
            gestor: node.properties.get("gestor").cloned(),
            kind: node.kind,
            ssh_key,
            db_cred,
        });
    }
    Ok(targets)
}

/// Resuelve la petición de conexión para ejecutar un script en un nodo: elige su
/// credencial SSH y delega en `connections::resolve` (endpoint del contexto,
/// materialización de llave, etc.). Error si el nodo no tiene credencial SSH.
pub fn resolve_target(
    conn: &Connection,
    node_id: &str,
    context_id: Option<&str>,
) -> AppResult<ConnectionRequest> {
    let cred = pick_ssh_credential(conn, node_id)?
        .ok_or_else(|| AppError::Other("el equipo no tiene credencial SSH".into()))?;
    crate::usecases::connections::resolve(conn, node_id, Some(&cred.id), context_id)
}

// --- Base de datos (Modelo B: cliente local + red) --------------------------

/// ¿El intérprete es un motor de BD (se ejecuta con cliente local, no por SSH)?
pub fn is_db_engine(interpreter: &str) -> bool {
    matches!(
        interpreter,
        "postgresql" | "mysql" | "mariadb" | "mongodb" | "redis"
    )
}

/// Puerto por defecto del motor.
fn default_db_port(engine: &str) -> u16 {
    match engine {
        "postgresql" => 5432,
        "mysql" | "mariadb" => 3306,
        "mongodb" => 27017,
        "redis" => 6379,
        _ => 0,
    }
}

/// Datos de conexión a una BD ya resueltos desde el vault.
#[derive(Debug, Clone, PartialEq)]
pub struct DbConn {
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub password: Option<String>,
    pub dbname: Option<String>,
    /// Solo mongo: usar `mongodb+srv://` (clúster con descubrimiento por DNS SRV,
    /// p. ej. Atlas). En ese caso no se añade puerto a la URI.
    pub srv: bool,
}

/// Comando local para un cliente de BD (programa + args + variable de entorno
/// del secreto, si aplica). Puro y testeable.
#[derive(Debug, Clone, PartialEq)]
pub struct DbCommand {
    pub program: String,
    pub args: Vec<String>,
    /// `(clave, valor)` de entorno para el secreto (fuera de argv), si aplica.
    pub env: Option<(String, String)>,
}

/// Arma el comando del cliente local según el motor. Con `interactive=false` el
/// cuerpo del script se pasa por stdin (psql en modo `-f -`); con `interactive=true`
/// se abre el cliente para uso manual (sin `-f -`). El secreto va por variable de
/// entorno (PGPASSWORD/MYSQL_PWD/REDISCLI_AUTH); en mongo va en la URI (visible en
/// `ps` local, decisión asumida).
pub fn build_db_command(engine: &str, c: &DbConn, interactive: bool) -> AppResult<DbCommand> {
    let port = c.port.to_string();
    let cmd = match engine {
        "postgresql" => {
            let mut args = vec!["-h".into(), c.host.clone(), "-p".into(), port];
            if let Some(u) = &c.user {
                args.push("-U".into());
                args.push(u.clone());
            }
            if let Some(db) = &c.dbname {
                args.push("-d".into());
                args.push(db.clone());
            }
            if !interactive {
                args.push("-f".into());
                args.push("-".into());
            }
            DbCommand {
                program: "psql".into(),
                args,
                env: c.password.clone().map(|p| ("PGPASSWORD".into(), p)),
            }
        }
        "mysql" | "mariadb" => {
            let mut args = vec!["-h".into(), c.host.clone(), "-P".into(), port];
            if let Some(u) = &c.user {
                args.push("-u".into());
                args.push(u.clone());
            }
            if let Some(db) = &c.dbname {
                args.push(db.clone());
            }
            DbCommand {
                program: "mysql".into(),
                args,
                env: c.password.clone().map(|p| ("MYSQL_PWD".into(), p)),
            }
        }
        "mongodb" => DbCommand {
            program: "mongosh".into(),
            args: vec![mongo_uri(c)],
            env: None,
        },
        "redis" => {
            let mut args = vec!["-h".into(), c.host.clone(), "-p".into(), port];
            // `instancia` numérica ⇒ índice de BD (`-n`).
            if let Some(n) = c.dbname.as_deref().and_then(|d| d.parse::<u8>().ok()) {
                args.push("-n".into());
                args.push(n.to_string());
            }
            DbCommand {
                program: "redis-cli".into(),
                args,
                env: c.password.clone().map(|p| ("REDISCLI_AUTH".into(), p)),
            }
        }
        other => {
            return Err(AppError::Other(format!("motor de BD no soportado: {other}")));
        }
    };
    Ok(cmd)
}

/// Lee una propiedad del nodo.
fn node_property(conn: &Connection, node_id: &str, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM node_properties WHERE node_id = ?1 AND key = ?2",
            rusqlite::params![node_id, key],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
}

/// Arma la URI de conexión de Mongo:
/// `mongodb://[user[:pass]@]host:port[/db]` o, para clúster/Atlas (SRV),
/// `mongodb+srv://[user[:pass]@]host[/db]` (los nodos y el puerto los resuelve el
/// driver por registros DNS SRV, así que no se pone puerto). Usuario/contraseña
/// se codifican en porcentaje: si traen `@ : / ? #` (comunes en Atlas) romperían
/// la URI.
pub fn mongo_uri(c: &DbConn) -> String {
    let auth = match (&c.user, &c.password) {
        (Some(u), Some(p)) => format!("{}:{}@", percent_encode(u), percent_encode(p)),
        (Some(u), None) => format!("{}@", percent_encode(u)),
        _ => String::new(),
    };
    let db = c.dbname.as_deref().map(|d| format!("/{d}")).unwrap_or_default();
    if c.srv {
        format!("mongodb+srv://{auth}{}{db}", c.host)
    } else {
        format!("mongodb://{auth}{}:{}{db}", c.host, c.port)
    }
}

/// Comando `mongosh` headless para comprobar salud: hace `ping` al servidor y sale
/// con código 0 si responde. Fija `serverSelectionTimeoutMS` para no colgarse si
/// el clúster no contesta. Reusa la URI (SRV/TLS/auth los maneja mongosh).
pub fn build_mongo_ping(c: &DbConn) -> DbCommand {
    let base = mongo_uri(c);
    // La query necesita un path: si no hay `/db`, añade `/` antes de `?`.
    let uri = if c.dbname.is_some() {
        format!("{base}?serverSelectionTimeoutMS=4000")
    } else {
        format!("{base}/?serverSelectionTimeoutMS=4000")
    };
    DbCommand {
        program: "mongosh".into(),
        args: vec![
            uri,
            "--quiet".into(),
            "--norc".into(),
            "--eval".into(),
            "db.adminCommand({ ping: 1 })".into(),
        ],
        env: None,
    }
}

/// Comando `redis-cli` headless para comprobar salud: envía `PING` y sale con
/// código 0 si el servidor responde `PONG`. Reusa host/puerto/índice y el secreto
/// por `REDISCLI_AUTH` (igual que el cliente interactivo). No fija timeout propio:
/// el caller acota la espera matando el proceso si tarda demasiado.
pub fn build_redis_ping(c: &DbConn) -> DbCommand {
    let mut args = vec!["-h".into(), c.host.clone(), "-p".into(), c.port.to_string()];
    // `instancia` numérica ⇒ índice de BD (`-n`), igual que en el cliente.
    if let Some(n) = c.dbname.as_deref().and_then(|d| d.parse::<u8>().ok()) {
        args.push("-n".into());
        args.push(n.to_string());
    }
    args.push("PING".into());
    DbCommand {
        program: "redis-cli".into(),
        args,
        env: c.password.clone().map(|p| ("REDISCLI_AUTH".into(), p)),
    }
}

/// Codifica en porcentaje para el userinfo de una URI (RFC 3986): deja intactos
/// los caracteres no reservados (`A-Z a-z 0-9 - . _ ~`) y codifica el resto. Así
/// usuarios/contraseñas con `@ : / ? # %` no rompen la URI de mongo.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Quita un `:puerto` final si lo hay (deja el host limpio). No toca IPv6/otros.
fn strip_port(addr: &str) -> String {
    match addr.rsplit_once(':') {
        Some((h, p)) if p.parse::<u16>().is_ok() => h.to_string(),
        _ => addr.to_string(),
    }
}

/// Normaliza el host de Mongo y decide si la conexión es por SRV. Devuelve
/// `(host, srv)`:
/// - Si el valor trae esquema explícito (`mongodb+srv://` / `mongodb://`), manda.
/// - Sin esquema, autodetecta SRV para clústeres Atlas (`*.mongodb.net`).
/// - Descarta userinfo (`user@`), ruta y query; en modo no-SRV también el puerto.
fn parse_mongo_host(raw: &str) -> (String, bool) {
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix("mongodb+srv://") {
        return (mongo_authority(rest, true), true);
    }
    if let Some(rest) = raw.strip_prefix("mongodb://") {
        return (mongo_authority(rest, false), false);
    }
    let host = mongo_authority(raw, false);
    let srv = host.ends_with(".mongodb.net");
    (host, srv)
}

/// Extrae el host de una autoridad `[user@]host[:port][/db][?...]`. Con
/// `srv=false` quita también el `:puerto` (va aparte); con `srv=true` no hay.
fn mongo_authority(rest: &str, srv: bool) -> String {
    let authority = rest.split(['/', '?']).next().unwrap_or(rest);
    let host = authority.rsplit('@').next().unwrap_or(authority);
    if srv {
        host.to_string()
    } else {
        strip_port(host)
    }
}

/// Resuelve los datos de conexión a la BD del nodo (para el motor dado). host =
/// endpoint del contexto → propiedad `host` → hostname; port = cred → default;
/// dbname = propiedad `instancia`. Error si falta la credencial de BD.
pub fn resolve_db_target(
    conn: &Connection,
    node_id: &str,
    context_id: Option<&str>,
    engine: &str,
) -> AppResult<DbConn> {
    let cred = pick_credential(conn, node_id, "db")?
        .ok_or_else(|| AppError::Other("el equipo no tiene credencial de BD".into()))?;

    // Dirección cruda del vault (endpoint del contexto → hostname → prop `host`
    // heredada). El saneado del puerto/esquema depende del motor.
    let raw = crate::usecases::connections::node_host(conn, node_id, context_id)?
        .or(node_property(conn, node_id, "host")?)
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .ok_or_else(|| AppError::Other("el equipo no tiene dirección para la BD".into()))?;

    // Mongo puede ser clúster/Atlas (SRV, sin puerto); el resto usa host:puerto.
    let (host, srv) = if engine == "mongodb" {
        parse_mongo_host(&raw)
    } else {
        (strip_port(&raw), false)
    };

    let password = crate::usecases::workspace::credential_reveal(conn, &cred.id)?;
    Ok(DbConn {
        host,
        port: cred.port.unwrap_or_else(|| default_db_port(engine)),
        user: cred.username,
        password,
        dbname: node_property(conn, node_id, "instancia")?,
        srv,
    })
}

/// Comando remoto que recibe el cuerpo del script por stdin, según el intérprete.
/// `bash` → `bash -s`; `python` → `python3 -`. Cualquier otro cae a `bash`.
fn interpreter_cmd(interpreter: &str) -> [&'static str; 2] {
    match interpreter {
        "python" => ["python3", "-"],
        _ => ["bash", "-s"],
    }
}

/// Arma el argv no-interactivo de `ssh` para capturar la salida del script. El
/// cuerpo va por stdin al intérprete (sin quoting). Precondición: SSH + llave.
pub fn build_capture_argv(req: &ConnectionRequest, interpreter: &str) -> AppResult<Vec<String>> {
    if req.kind != ConnectionKind::Ssh {
        return Err(AppError::Other(
            "solo se pueden ejecutar scripts por SSH".into(),
        ));
    }
    let key = req
        .key_path
        .as_deref()
        .filter(|k| !k.is_empty())
        .ok_or_else(|| {
            AppError::Other("requiere llave: el equipo no tiene llave SSH configurada".into())
        })?;

    let mut argv = vec![
        "ssh".to_string(),
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "ConnectTimeout=10".into(),
    ];
    for opt in &req.ssh_options {
        argv.push("-o".into());
        argv.push(opt.clone());
    }
    argv.push("-i".into());
    argv.push(key.to_string());
    if let Some(port) = req.port {
        argv.push("-p".into());
        argv.push(port.to_string());
    }
    argv.push(match &req.user {
        Some(u) => format!("{u}@{}", req.host),
        None => req.host.clone(),
    });
    for tok in interpreter_cmd(interpreter) {
        argv.push(tok.into());
    }
    Ok(argv)
}

/// Ejecuta el script en un equipo, emitiendo `RunEvent`s (status/líneas/done).
/// `emit` puede llamarse desde varios hilos, así que debe ser `Sync`. Bloquea
/// hasta que el proceso remoto termina (el caller decide secuencial/paralelo).
pub fn run_target(
    req: &ConnectionRequest,
    node_id: &str,
    body: &str,
    interpreter: &str,
    emit: &(dyn Fn(RunEvent) + Sync),
) {
    let argv = match build_capture_argv(req, interpreter) {
        Ok(a) => a,
        Err(e) => return emit_error(node_id, &e.to_string(), emit),
    };
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    run_streamed(cmd, "ssh", node_id, body, emit);
}

/// Ejecuta un `Command` ya armado capturando su salida **línea a línea** por
/// `emit`: escribe `body` en stdin y transmite stdout+stderr (hilos scoped).
/// `what` describe el binario para los mensajes de error. Bloquea hasta terminar.
fn run_streamed(
    mut cmd: Command,
    what: &str,
    node_id: &str,
    body: &str,
    emit: &(dyn Fn(RunEvent) + Sync),
) {
    emit(RunEvent::Status {
        node_id: node_id.to_string(),
        status: "running".into(),
    });

    let mut child = match cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return emit_error(node_id, &format!("no se pudo lanzar {what}: {e}"), emit),
    };

    // Manda el cuerpo por stdin y ciérralo (EOF) para que el intérprete lo ejecute.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(body.as_bytes());
        // `stdin` se dropea al salir del bloque → cierra el pipe.
    }

    // Envuelve stdout/stderr como lectores de líneas homogéneos (trait objects).
    let readers: Vec<Box<dyn BufRead + Send>> = [
        child.stdout.take().map(|r| Box::new(BufReader::new(r)) as Box<dyn BufRead + Send>),
        child.stderr.take().map(|r| Box::new(BufReader::new(r)) as Box<dyn BufRead + Send>),
    ]
    .into_iter()
    .flatten()
    .collect();

    // Lee ambos streams en paralelo (scoped: sin 'static, `emit` prestado).
    std::thread::scope(|scope| {
        for reader in readers {
            scope.spawn(move || {
                for line in reader.lines().map_while(Result::ok) {
                    emit(RunEvent::Line {
                        node_id: node_id.to_string(),
                        line,
                    });
                }
            });
        }
    });

    let (exit_code, status) = match child.wait() {
        Ok(st) => (st.code(), if st.success() { "ok" } else { "error" }),
        Err(_) => (None, "error"),
    };
    if status == "error" {
        // Terminó con fallo (script inválido, SSH no pudo conectar, permisos…).
        // No registramos la salida (stderr) para no filtrar host/datos; solo el
        // binario y el código de salida bastan para orientar el diagnóstico.
        let code = exit_code.map(|c| c.to_string()).unwrap_or_else(|| "n/a".into());
        crate::usecases::diagnostics::warn(
            "script",
            "target_failed",
            &[("nodeId", node_id), ("program", what), ("exitCode", &code)],
        );
    }
    emit(RunEvent::Status {
        node_id: node_id.to_string(),
        status: status.into(),
    });
    emit(RunEvent::Done {
        node_id: node_id.to_string(),
        exit_code,
        error: None,
    });
}

/// Ejecuta el script contra una BD con el cliente local del motor. Detecta el
/// binario en el PATH y pasa el secreto por env (o URI en mongo).
pub fn run_db_target(
    engine: &str,
    conn: &DbConn,
    node_id: &str,
    body: &str,
    emit: &(dyn Fn(RunEvent) + Sync),
) {
    let spec = match build_db_command(engine, conn, false) {
        Ok(s) => s,
        Err(e) => return emit_error(node_id, &e.to_string(), emit),
    };
    if !crate::usecases::connections::program_in_path(&spec.program) {
        return emit_error(
            node_id,
            &format!("no se encontró el cliente '{}' en el PATH; instálalo", spec.program),
            emit,
        );
    }
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args);
    if let Some((k, v)) = &spec.env {
        cmd.env(k, v);
    }
    run_streamed(cmd, &spec.program, node_id, body, emit);
}

/// Emite el par status=error + done{error} para un equipo que no pudo ejecutar
/// (fallo de precondición, de resolución o de arranque del proceso).
pub fn emit_error(node_id: &str, message: &str, emit: &(dyn Fn(RunEvent) + Sync)) {
    // El script no pudo ni arrancar (precondición: falta llave/credencial, no se
    // pudo lanzar el proceso, motor no soportado…). Lo dejamos en el log de soporte.
    crate::usecases::diagnostics::warn(
        "script",
        "target_error",
        &[("nodeId", node_id), ("error", message)],
    );
    emit(RunEvent::Status {
        node_id: node_id.to_string(),
        status: "error".into(),
    });
    emit(RunEvent::Done {
        node_id: node_id.to_string(),
        exit_code: None,
        error: Some(message.to_string()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ssh_req(key: Option<&str>) -> ConnectionRequest {
        ConnectionRequest {
            kind: ConnectionKind::Ssh,
            user: Some("root".into()),
            host: "10.0.0.5".into(),
            port: Some(2222),
            key_path: key.map(str::to_string),
            url: None,
            ssh_options: vec!["ServerAliveInterval=30".into()],
        }
    }

    #[test]
    fn build_argv_includes_batchmode_key_port_and_options() {
        let argv = build_capture_argv(&ssh_req(Some("/home/me/.ssh/id_ed25519")), "bash").unwrap();
        assert_eq!(
            argv,
            vec![
                "ssh",
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "ServerAliveInterval=30",
                "-i",
                "/home/me/.ssh/id_ed25519",
                "-p",
                "2222",
                "root@10.0.0.5",
                "bash",
                "-s",
            ]
        );
    }

    #[test]
    fn build_argv_python_uses_python3_stdin() {
        let argv = build_capture_argv(&ssh_req(Some("/k")), "python").unwrap();
        assert!(argv.ends_with(&["python3".to_string(), "-".to_string()]));
        assert!(!argv.contains(&"bash".to_string()));
    }

    #[test]
    fn build_argv_unknown_interpreter_falls_back_to_bash() {
        let argv = build_capture_argv(&ssh_req(Some("/k")), "cobol").unwrap();
        assert!(argv.ends_with(&["bash".to_string(), "-s".to_string()]));
    }

    #[test]
    fn build_argv_without_user_or_port() {
        let req = ConnectionRequest {
            user: None,
            port: None,
            ssh_options: Vec::new(),
            ..ssh_req(Some("/k"))
        };
        let argv = build_capture_argv(&req, "bash").unwrap();
        assert!(argv.contains(&"10.0.0.5".to_string()));
        assert!(!argv.contains(&"-p".to_string()));
        assert!(argv.ends_with(&["bash".to_string(), "-s".to_string()]));
    }

    #[test]
    fn build_argv_errors_without_key() {
        let err = build_capture_argv(&ssh_req(None), "bash").unwrap_err().to_string();
        assert!(err.contains("requiere llave"), "mensaje: {err}");
        // Llave vacía cuenta como sin llave.
        assert!(build_capture_argv(&ssh_req(Some("")), "bash").is_err());
    }

    #[test]
    fn run_event_serializes_with_camelcase_fields() {
        // El frontend lee `type`/`nodeId`/`exitCode`; si los campos salieran en
        // snake_case, la salida se quedaría en "pending". Blindamos la forma JSON.
        let line = serde_json::to_value(RunEvent::Line {
            node_id: "n1".into(),
            line: "hola".into(),
        })
        .unwrap();
        assert_eq!(line["type"], "line");
        assert_eq!(line["nodeId"], "n1");

        let done = serde_json::to_value(RunEvent::Done {
            node_id: "n1".into(),
            exit_code: Some(0),
            error: None,
        })
        .unwrap();
        assert_eq!(done["type"], "done");
        assert_eq!(done["nodeId"], "n1");
        assert_eq!(done["exitCode"], 0);
    }

    #[test]
    fn build_argv_errors_for_non_ssh() {
        let req = ConnectionRequest {
            kind: ConnectionKind::Web,
            ..ssh_req(Some("/k"))
        };
        assert!(build_capture_argv(&req, "bash").is_err());
    }

    fn dbconn(dbname: Option<&str>) -> DbConn {
        DbConn {
            host: "db.internal".into(),
            port: 5432,
            user: Some("app".into()),
            password: Some("s3cr3t".into()),
            dbname: dbname.map(str::to_string),
            srv: false,
        }
    }

    #[test]
    fn db_command_postgresql_uses_pgpassword_and_stdin() {
        let c = build_db_command("postgresql", &dbconn(Some("shop")), false).unwrap();
        assert_eq!(c.program, "psql");
        assert_eq!(
            c.args,
            vec!["-h", "db.internal", "-p", "5432", "-U", "app", "-d", "shop", "-f", "-"]
        );
        assert_eq!(c.env, Some(("PGPASSWORD".into(), "s3cr3t".into())));
    }

    #[test]
    fn db_command_postgresql_interactive_omits_stdin_flag() {
        let c = build_db_command("postgresql", &dbconn(Some("shop")), true).unwrap();
        assert!(!c.args.contains(&"-f".to_string()));
    }

    #[test]
    fn db_command_mysql_omits_db_when_absent() {
        let mut conn = dbconn(None);
        conn.port = 3306;
        let c = build_db_command("mysql", &conn, false).unwrap();
        assert_eq!(c.program, "mysql");
        assert_eq!(c.args, vec!["-h", "db.internal", "-P", "3306", "-u", "app"]);
        assert_eq!(c.env, Some(("MYSQL_PWD".into(), "s3cr3t".into())));
    }

    #[test]
    fn db_command_mongodb_puts_credentials_in_uri() {
        let mut conn = dbconn(Some("shop"));
        conn.port = 27017;
        let c = build_db_command("mongodb", &conn, false).unwrap();
        assert_eq!(c.program, "mongosh");
        assert_eq!(c.args, vec!["mongodb://app:s3cr3t@db.internal:27017/shop"]);
        assert!(c.env.is_none());
    }

    #[test]
    fn db_command_mongodb_srv_omits_port_and_uses_srv_scheme() {
        let mut conn = dbconn(Some("dziro"));
        conn.host = "dziro.zktuu.mongodb.net".into();
        conn.srv = true;
        let c = build_db_command("mongodb", &conn, true).unwrap();
        assert_eq!(
            c.args,
            vec!["mongodb+srv://app:s3cr3t@dziro.zktuu.mongodb.net/dziro"]
        );
    }

    #[test]
    fn db_command_mongodb_percent_encodes_credentials() {
        let mut conn = dbconn(Some("dziro"));
        conn.host = "clu.mongodb.net".into();
        conn.srv = true;
        conn.user = Some("dziro-app".into());
        conn.password = Some("p@ss:w/rd#1".into());
        let c = build_db_command("mongodb", &conn, true).unwrap();
        assert_eq!(
            c.args,
            vec!["mongodb+srv://dziro-app:p%40ss%3Aw%2Frd%231@clu.mongodb.net/dziro"]
        );
    }

    #[test]
    fn build_mongo_ping_srv_headless_with_timeout() {
        let mut conn = dbconn(Some("dziro"));
        conn.host = "clu.mongodb.net".into();
        conn.srv = true;
        let c = build_mongo_ping(&conn);
        assert_eq!(c.program, "mongosh");
        assert_eq!(
            c.args,
            vec![
                "mongodb+srv://app:s3cr3t@clu.mongodb.net/dziro?serverSelectionTimeoutMS=4000",
                "--quiet",
                "--norc",
                "--eval",
                "db.adminCommand({ ping: 1 })",
            ]
        );
    }

    #[test]
    fn build_mongo_ping_without_db_inserts_path_before_query() {
        let mut conn = dbconn(None);
        conn.host = "clu.mongodb.net".into();
        conn.srv = true;
        let c = build_mongo_ping(&conn);
        assert_eq!(
            c.args[0],
            "mongodb+srv://app:s3cr3t@clu.mongodb.net/?serverSelectionTimeoutMS=4000"
        );
    }

    #[test]
    fn parse_mongo_host_detects_atlas_srv() {
        assert_eq!(
            parse_mongo_host("dziro.zktuu.mongodb.net"),
            ("dziro.zktuu.mongodb.net".to_string(), true)
        );
    }

    #[test]
    fn parse_mongo_host_respects_explicit_scheme() {
        assert_eq!(
            parse_mongo_host("mongodb+srv://user@clu.example.com/db"),
            ("clu.example.com".to_string(), true)
        );
        assert_eq!(
            parse_mongo_host("mongodb://box.local:27017"),
            ("box.local".to_string(), false)
        );
    }

    #[test]
    fn parse_mongo_host_plain_strips_port() {
        assert_eq!(
            parse_mongo_host("10.0.0.5:27017"),
            ("10.0.0.5".to_string(), false)
        );
    }

    #[test]
    fn db_command_redis_adds_index_only_when_numeric() {
        let mut conn = dbconn(Some("2"));
        conn.port = 6379;
        let c = build_db_command("redis", &conn, false).unwrap();
        assert_eq!(c.program, "redis-cli");
        assert_eq!(c.args, vec!["-h", "db.internal", "-p", "6379", "-n", "2"]);
        assert_eq!(c.env, Some(("REDISCLI_AUTH".into(), "s3cr3t".into())));

        // `instancia` no numérica ⇒ sin `-n`.
        let c2 = build_db_command("redis", &dbconn(Some("cache")), false).unwrap();
        assert!(!c2.args.contains(&"-n".to_string()));
    }

    #[test]
    fn build_redis_ping_appends_ping_with_index_and_auth() {
        let mut conn = dbconn(Some("2"));
        conn.port = 6379;
        let c = build_redis_ping(&conn);
        assert_eq!(c.program, "redis-cli");
        assert_eq!(c.args, vec!["-h", "db.internal", "-p", "6379", "-n", "2", "PING"]);
        assert_eq!(c.env, Some(("REDISCLI_AUTH".into(), "s3cr3t".into())));

        // Sin índice numérico ni contraseña: solo host/puerto + PING, sin env.
        let mut plain = dbconn(None);
        plain.password = None;
        let c2 = build_redis_ping(&plain);
        assert_eq!(c2.args.last().map(String::as_str), Some("PING"));
        assert!(!c2.args.contains(&"-n".to_string()));
        assert_eq!(c2.env, None);
    }

    #[test]
    fn db_command_rejects_unknown_engine() {
        assert!(build_db_command("oracle", &dbconn(None), false).is_err());
    }

    #[test]
    fn is_db_engine_classifies() {
        assert!(is_db_engine("postgresql") && is_db_engine("redis"));
        assert!(!is_db_engine("bash") && !is_db_engine("python"));
    }
}
