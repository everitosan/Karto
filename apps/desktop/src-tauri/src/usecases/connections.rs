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
use std::path::{Path, PathBuf};

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

/// Terminales de Windows en orden de preferencia. **Solo PowerShell**, y es una
/// decisión deliberada: se lanza con `CREATE_NEW_CONSOLE` (ver `terminal_command`)
/// y, si el usuario tiene Windows Terminal como terminal predeterminada del
/// sistema, Windows enruta esa consola nueva a `wt` por su cuenta. Así se obtiene
/// la UX de `wt` **sin** pasar el comando por su parser, que trocea la línea en
/// cada `;` (comprobado: un script con `;` sin escapar no llega a ejecutarse).
/// Apilar el escapado de `wt` sobre el de PowerShell sería una segunda capa de
/// quoting sobre datos del vault: más superficie de inyección a cambio de nada.
pub const WINDOWS_TERMINALS: &[TerminalDef] = &[TerminalDef {
    program: "powershell.exe",
    exec: &["-NoProfile", "-Command"],
}];

/// Terminales candidatas para un SO. macOS sigue sin soporte (su Fase 7 propia):
/// la lista vacía hace que `require_terminal` dé el error explicativo.
pub fn terminals_for(os: Os) -> &'static [TerminalDef] {
    match os {
        Os::Linux => LINUX_TERMINALS,
        Os::Windows => WINDOWS_TERMINALS,
        Os::Macos => &[],
    }
}

/// Primera terminal disponible según el predicado `exists` (inyectable en test).
pub fn detect_terminal(
    terminals: &[TerminalDef],
    exists: impl Fn(&str) -> bool,
) -> Option<&TerminalDef> {
    terminals.iter().find(|t| exists(t.program))
}

/// Extensiones con las que probar un nombre de ejecutable. En Unix el nombre va
/// tal cual; en Windows el binario lleva sufijo (`ssh` → `ssh.exe`), así que se
/// prueban además las de `PATHEXT`. La cadena vacía va **siempre primero**: cubre
/// el nombre que ya trae extensión (`wt.exe`) y mantiene Unix sin cambios.
///
/// Sin esto, en Windows `dir.join("ssh")` no existe nunca y la detección de
/// binarios devuelve `false` para todo: clientes de BD, terminales y el
/// encabezado de diagnóstico se quedan en blanco sin explicación.
pub fn executable_exts() -> Vec<String> {
    let mut exts = vec![String::new()];
    if cfg!(windows) {
        // Valor por defecto de Windows cuando `PATHEXT` no está definida.
        let raw = std::env::var_os("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
        exts.extend(
            raw.to_string_lossy()
                .split(';')
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .map(|e| e.to_ascii_lowercase()),
        );
    }
    exts
}

/// Núcleo puro de la búsqueda: ¿hay un archivo `name` + alguna de `exts` en
/// alguno de `dirs`? `is_file` se inyecta para probarlo sin tocar disco y para
/// poder verificar el comportamiento de Windows compilando en Linux.
pub fn find_program(
    name: &str,
    dirs: &[PathBuf],
    exts: &[String],
    is_file: impl Fn(&Path) -> bool,
) -> bool {
    dirs.iter()
        .any(|dir| exts.iter().any(|ext| is_file(&dir.join(format!("{name}{ext}")))))
}

/// ¿Hay un ejecutable con ese nombre en el `PATH`? (sin dependencias externas).
pub fn program_in_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    let dirs: Vec<PathBuf> = std::env::split_paths(&path).collect();
    find_program(name, &dirs, &executable_exts(), |p| {
        std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
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
pub fn to_shell_line(argv: &[String]) -> String {
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
pub fn hold_wrapper(inner: &[String], os: Os) -> Vec<String> {
    match os {
        Os::Windows => hold_line(&to_pwsh_line(inner), os),
        _ => hold_line(&to_shell_line(inner), os),
    }
}

/// Igual que `hold_wrapper` pero sobre una línea de shell ya construida (permite
/// encadenar comandos, p. ej. `ssh-copy-id … && ssh -i …`).
pub fn hold_line(line: &str, os: Os) -> Vec<String> {
    match os {
        // PowerShell: `$LASTEXITCODE` trae el código del binario nativo. Devuelve
        // **un solo elemento** porque la terminal ya aporta `-NoProfile -Command`
        // (ver `WINDOWS_TERMINALS`), igual que en Linux `bash -c` aporta los suyos.
        // `Read-Host` exige Enter: PowerShell no tiene equivalente de `read -n1`.
        Os::Windows => vec![format!(
            "{line}; $code = $LASTEXITCODE; Write-Host ''; \
             Read-Host \"Conexión finalizada (código $code). Pulsa Enter para cerrar\""
        )],
        _ => {
            let script = format!(
                "{line}; status=$?; echo; \
                 read -n1 -s -r -p \"Conexión finalizada (código $status). Pulsa una tecla para cerrar…\"; echo"
            );
            vec!["bash".to_string(), "-c".to_string(), script]
        }
    }
}

/// Escapa un argumento como **literal de PowerShell**: comillas simples, con la
/// comilla interna duplicada. Dentro de comillas simples PowerShell no expande
/// nada (`$`, backtick, `;`, `|`), así que un valor traído del vault —host,
/// usuario, ruta de llave, opción SSH— no puede convertirse en código.
fn pwsh_quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "''"))
}

/// Une un argv en una línea de PowerShell. Antepone el operador de llamada `&`
/// porque un programa entrecomillado se interpretaría como una **cadena** (se
/// imprimiría) en vez de ejecutarse.
pub fn to_pwsh_line(argv: &[String]) -> String {
    if argv.is_empty() {
        return String::new();
    }
    let mut parts = Vec::with_capacity(argv.len() + 1);
    parts.push("&".to_string());
    parts.extend(argv.iter().map(|a| pwsh_quote(a)));
    parts.join(" ")
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
/// Argumentos comunes de la credencial SSH (llave, puerto y opciones extra), sin
/// el ejecutable `ssh` ni el destino. Se reutiliza tanto en la conexión normal
/// como en el sondeo de datos del equipo.
fn ssh_base_args(req: &ConnectionRequest) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(key) = &req.key_path {
        args.push("-i".into());
        args.push(key.clone());
    }
    if let Some(port) = req.port {
        args.push("-p".into());
        args.push(port.to_string());
    }
    // Opciones extra del usuario: cada una entra como `-o <opción>` (un solo
    // argv por opción, así valores con espacios —p. ej. ProxyCommand— no rompen).
    for opt in &req.ssh_options {
        args.push("-o".into());
        args.push(opt.clone());
    }
    args
}

pub fn ssh_inner_command(req: &ConnectionRequest) -> Vec<String> {
    let mut cmd = vec!["ssh".to_string()];
    cmd.extend(ssh_base_args(req));
    cmd.push(ssh_destination(req));
    cmd
}

/// Línea de shell que **sondea el equipo y luego abre la sesión interactiva**,
/// con una sola autenticación. Usa multiplexado SSH (`ControlMaster`): la primera
/// conexión corre el script de facts y vuelca su salida a `facts_file` (local);
/// la segunda, interactiva y **sin modificar**, reutiliza el socket sin volver a
/// autenticar. Sirve igual con llave o con contraseña (el usuario la teclea una
/// vez en la primera). Si el multiplexado no está disponible, la interactiva
/// simplemente vuelve a pedir credenciales (degradado, no roto).
pub fn ssh_facts_line(req: &ConnectionRequest, control_path: &str, facts_file: &str) -> String {
    let base = ssh_base_args(req);
    let dest = ssh_destination(req);

    // 1) Conexión de sondeo: establece el maestro, corre el script, sale.
    let mut probe = vec!["ssh".to_string()];
    for o in [
        "ControlMaster=auto",
        &format!("ControlPath={control_path}"),
        "ControlPersist=30",
        "ConnectTimeout=8",
        "BatchMode=no",
    ] {
        probe.push("-o".into());
        probe.push(o.to_string());
    }
    probe.extend(base.clone());
    probe.push(dest.clone());
    probe.push(crate::usecases::facts::remote_script());

    // 2) Conexión interactiva: reutiliza el maestro (sin re-autenticar).
    let mut inter = vec!["ssh".to_string()];
    inter.push("-o".into());
    inter.push(format!("ControlPath={control_path}"));
    inter.extend(base);
    inter.push(dest);

    format!(
        "{} > {} 2>/dev/null; {}",
        to_shell_line(&probe),
        shell_quote(facts_file),
        to_shell_line(&inter),
    )
}

/// Directivas SSH que pueden ejecutar comandos locales al conectar. Se rechazan
/// porque un vault compartido/importado podría traerlas en las opciones de una
/// credencial y lograr ejecución de código en la máquina de quien conecta.
const DANGEROUS_SSH_DIRECTIVES: &[&str] = &[
    "proxycommand",
    "localcommand",
    "permitlocalcommand",
    "knownhostscommand",
    "match", // `Match exec <cmd>` ejecuta un comando local.
];

/// ¿La línea de opción SSH invoca una directiva capaz de lanzar procesos locales?
/// Normaliza el nombre de la directiva (primer token antes de `=` o espacio, en
/// minúsculas) y lo compara con la lista negra. Blocklist en vez de allowlist para
/// no romper opciones legítimas comunes (`ServerAliveInterval`, `ProxyJump`…).
pub fn is_dangerous_ssh_option(opt: &str) -> bool {
    let directive = opt
        .trim()
        .split(|c: char| c == '=' || c.is_whitespace())
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    DANGEROUS_SSH_DIRECTIVES.contains(&directive.as_str())
}

/// Normaliza el texto libre de opciones SSH: una opción por línea, recortando
/// espacios y descartando líneas vacías o comentarios (`#`). Además **filtra** las
/// directivas peligrosas (`is_dangerous_ssh_option`), registrando cada descarte en
/// el log de soporte para que el usuario entienda por qué su opción no se aplicó.
pub fn parse_ssh_options(raw: Option<&str>) -> Vec<String> {
    raw.map(|text| {
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter(|l| {
                if is_dangerous_ssh_option(l) {
                    crate::usecases::diagnostics::warn(
                        "connection",
                        "ssh_option_blocked",
                        &[("directive", l.split(|c: char| c == '=' || c.is_whitespace()).next().unwrap_or(""))],
                    );
                    false
                } else {
                    true
                }
            })
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

/// Comando final de SSH: envuelve el comando interno en la terminal del SO.
/// En Linux requiere una terminal detectada; en mac/Windows se delega en la
/// terminal por defecto del sistema (Fase 3 posterior afinará mac/Windows).
pub fn build_ssh(
    req: &ConnectionRequest,
    terminal: Option<&TerminalDef>,
    os: Os,
) -> AppResult<LaunchSpec> {
    let held = hold_wrapper(&ssh_inner_command(req), os);
    Ok(wrap_in_terminal(require_terminal(terminal)?, &held))
}

/// Como `build_ssh` pero sondea el equipo antes de la sesión interactiva
/// (ver `ssh_facts_line`). `facts_file` recibe los datos para que Karto los lea.
pub fn build_ssh_with_facts(
    req: &ConnectionRequest,
    terminal: Option<&TerminalDef>,
    control_path: &str,
    facts_file: &str,
) -> AppResult<LaunchSpec> {
    let held = hold_line(&ssh_facts_line(req, control_path, facts_file), Os::Linux);
    Ok(wrap_in_terminal(require_terminal(terminal)?, &held))
}

fn require_terminal(terminal: Option<&TerminalDef>) -> AppResult<&TerminalDef> {
    terminal.ok_or_else(|| {
        AppError::Other(match Os::current() {
            Os::Linux => "no se encontró una terminal soportada (instala gnome-terminal, konsole, kitty…)".into(),
            Os::Windows => "no se encontró PowerShell en el PATH".to_string(),
            Os::Macos => "las conexiones en terminal aún no están soportadas en macOS".to_string(),
        })
    })
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

/// Abre el **cliente VNC por defecto del equipo** apuntando a `vnc://host:puerto`,
/// delegando en el abridor del SO (`xdg-open`/`open`/`start`, igual que Web). Así
/// no dependemos de un binario concreto ni de empaquetar uno: el sistema lanza el
/// visor VNC registrado para el esquema `vnc://` y el usuario teclea la contraseña.
/// La inyección automática de la contraseña (sidecar TigerVNC + `VNC_PASSWORD`)
/// queda pendiente para una fase posterior.
pub fn build_vnc(req: &ConnectionRequest, os: Os) -> LaunchSpec {
    let port = req.port.unwrap_or(5900);
    let authority = match &req.user {
        Some(user) => format!("{user}@{}:{}", req.host, port),
        None => format!("{}:{}", req.host, port),
    };
    build_open_url(os, &format!("vnc://{authority}"))
}

/// Arma el `LaunchSpec` completo para una petición, según su tipo y el SO.
pub fn plan(req: &ConnectionRequest, os: Os) -> AppResult<LaunchSpec> {
    match req.kind {
        ConnectionKind::Ssh => {
            let terminal = detect_terminal(terminals_for(os), program_in_path);
            build_ssh(req, terminal, os)
        }
        ConnectionKind::Web => {
            let url = req
                .url
                .as_deref()
                .ok_or_else(|| AppError::Other("el nodo no tiene URL de administración".into()))?;
            Ok(build_open_url(os, url))
        }
        ConnectionKind::Vnc => Ok(build_vnc(req, os)),
        ConnectionKind::Rdp => Err(AppError::Other(
            "las conexiones RDP aún no están soportadas".into(),
        )),
    }
}

// --- Resolución desde el vault ----------------------------------------------

/// Deriva la dirección del nodo para el contexto activo: primero el endpoint del
/// nodo en ese contexto (la IP/host con la que se alcanza *desde ahí*); si no hay
/// (o no se pasó contexto), cae al `hostname` de las propiedades como respaldo
/// estable. Así, al cambiar de contexto (sitio/VPN), la conexión usa la dirección
/// correcta sin tocar nodo por nodo.
pub(crate) fn node_host(
    conn: &Connection,
    node_id: &str,
    context_id: Option<&str>,
) -> AppResult<Option<String>> {
    if let Some(ctx) = context_id {
        let endpoint = conn
            .query_row(
                "SELECT address FROM node_endpoints WHERE node_id = ?1 AND context_id = ?2",
                params![node_id, ctx],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        if endpoint.is_some() {
            return Ok(endpoint);
        }
    }
    // Respaldo: hostname/FQDN (identidad estable, no depende del contexto).
    let host = conn
        .query_row(
            "SELECT value FROM node_properties WHERE node_id = ?1 AND key = 'hostname'",
            params![node_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(host)
}

/// URL de administración del nodo (`url_admin` o `url`).
pub(crate) fn node_url(conn: &Connection, node_id: &str) -> AppResult<Option<String>> {
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
    /// Material de la llave privada guardado en el vault (si el usuario lo eligió).
    private_key: Option<String>,
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
                "SELECT kind, username, port, key_path, options, private_key FROM credentials \
                 WHERE id = ?1 AND node_id = ?2",
                params![id, node_id],
                map_cred_row,
            )
            .optional()?,
        None => conn
            .query_row(
                "SELECT kind, username, port, key_path, options, private_key FROM credentials \
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
        private_key: r.get(5)?,
    })
}

/// Escribe la llave privada a disco con permisos 0600 si el archivo aún no
/// existe. Se usa para materializar una llave guardada en el vault al abrir el
/// vault en otro equipo. Crea el directorio padre (0700 en Unix) si hace falta.
///
/// Si el archivo destino **ya existe**, no lo sobrescribe, pero en Unix verifica
/// que sea seguro reusarlo: modo exactamente 0600 (sin permisos para grupo/otros).
/// Si es más permisivo, aborta con un error en vez de conectar con una llave que
/// podría haber sido leída o manipulada por otro usuario.
pub fn materialize_key(key_path: &str, private_key: &str) -> AppResult<()> {
    use crate::infra::file_perms::{self, Audit, Kind};

    let path = Path::new(key_path);
    if path.exists() {
        return match file_perms::audit(path)? {
            Audit::Private => Ok(()),
            Audit::Exposed { detail } => Err(AppError::Other(format!(
                "la llave existente {key_path} es accesible por otras cuentas ({detail}); \
                 corrígelo o bórrala para regenerarla"
            ))),
            // Windows: leer la DACL exigiría una dependencia nueva de la API del
            // SO. No se toca el archivo —es del usuario, y apretarle los permisos
            // en silencio no desharía una exposición previa, sólo la ocultaría—;
            // queda constancia en el log y, si de verdad está abierta, `ssh` la
            // rechaza él mismo con "UNPROTECTED PRIVATE KEY FILE".
            Audit::Unknown => {
                crate::usecases::diagnostics::warn(
                    "connection",
                    "key_perms_unverified",
                    &[("reason", "la plataforma no permite auditar los permisos")],
                );
                Ok(())
            }
        };
    }
    if let Some(parent) = path.parent() {
        // Sólo se restringe el directorio si lo creamos nosotros: si ya existía
        // es del usuario (p. ej. `~/.ssh`, con sus propios permisos o ACEs), y
        // reescribírselos sería un efecto colateral que nadie pidió.
        let created = !parent.exists();
        std::fs::create_dir_all(parent)?;
        if created {
            file_perms::restrict_to_owner(parent, Kind::Dir)?;
        }
    }
    std::fs::write(path, private_key)?;
    // Debe ir sí o sí: una llave en claro con permisos abiertos no sólo se puede
    // filtrar, es que `ssh` se niega a usarla.
    file_perms::restrict_to_owner(path, Kind::File)?;
    Ok(())
}

/// Construye la petición de conexión leyendo el nodo y la credencial del vault.
pub fn resolve(
    conn: &Connection,
    node_id: &str,
    credential_id: Option<&str>,
    context_id: Option<&str>,
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
            let host = node_host(conn, node_id, context_id)?.ok_or_else(|| {
                AppError::Other(
                    "el nodo no tiene dirección en el contexto activo ni hostname para conectar"
                        .into(),
                )
            })?;
            // Si la credencial trae la llave guardada en el vault y su archivo no
            // existe en disco (p. ej. vault movido a otro equipo), se materializa.
            if let (Some(kp), Some(pk)) = (&cred.key_path, &cred.private_key) {
                materialize_key(kp, pk)?;
            }
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

/// Cuando Karto corre empaquetado como **AppImage**, su `AppRun` inyecta rutas a
/// las librerías del bundle (`LD_LIBRARY_PATH`, `GTK_PATH`, `GIO_MODULE_DIR`…).
/// Cualquier proceso externo que lancemos (la terminal, `xdg-open`) las **hereda**
/// e intenta cargar esas libs incompatibles con las del sistema → no arranca (la
/// terminal "no abre"). Antes de spawnear devolvemos al hijo un entorno de sistema:
/// de las variables tipo-lista quitamos solo las entradas bajo `$APPDIR` y
/// eliminamos por completo las que apuntan a un fichero/dir del bundle. Fuera del
/// AppImage (deb, binario suelto, dev) `APPDIR` no existe y no tocamos nada.
fn strip_appimage_env(cmd: &mut std::process::Command) {
    let Some(appdir) = std::env::var_os("APPDIR") else {
        return;
    };
    let appdir = std::path::PathBuf::from(appdir);

    // Variables tipo-PATH (rutas separadas por ':'): conservamos las del sistema.
    for var in [
        "LD_LIBRARY_PATH",
        "XDG_DATA_DIRS",
        "GTK_PATH",
        "GST_PLUGIN_SYSTEM_PATH",
        "GST_PLUGIN_SYSTEM_PATH_1_0",
    ] {
        let Some(val) = std::env::var_os(var) else { continue };
        let kept: Vec<_> = std::env::split_paths(&val)
            .filter(|p| !p.starts_with(&appdir))
            .collect();
        match std::env::join_paths(kept) {
            Ok(joined) if !joined.is_empty() => {
                cmd.env(var, joined);
            }
            _ => {
                cmd.env_remove(var);
            }
        }
    }

    // Variables que apuntan a un único fichero/dir dentro del bundle: se quitan.
    for var in [
        "GDK_PIXBUF_MODULE_FILE",
        "GDK_PIXBUF_MODULEDIR",
        "GIO_MODULE_DIR",
        "GSETTINGS_SCHEMA_DIR",
        "GST_PLUGIN_SCANNER",
    ] {
        cmd.env_remove(var);
    }
}

/// `Command` para un proceso **externo** (terminal, navegador, visor VNC) con el
/// entorno del AppImage ya saneado (ver [`strip_appimage_env`]).
fn external_command(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    strip_appimage_env(&mut cmd);
    // Windows: el abridor de URL/VNC pasa por `cmd /C start`, que sin esto
    // parpadea una consola negra un instante antes de ceder al navegador.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// `Command` para un proceso que **debe verse en una consola**: la terminal con
/// `ssh` o el cliente de BD.
///
/// En Windows es imprescindible `CREATE_NEW_CONSOLE`: Karto es una app GUI y no
/// tiene consola propia, así que sin la bandera el proceso no tendría dónde
/// dibujarse. Con ella Windows crea una consola nueva — y si el usuario tiene
/// Windows Terminal como predeterminada del sistema, la enruta ahí sola. En
/// Linux/macOS el emulador de terminal ya abre su propia ventana.
fn terminal_command(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    strip_appimage_env(&mut cmd);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        cmd.creation_flags(CREATE_NEW_CONSOLE);
    }
    cmd
}

/// Resuelve la conexión y lanza el proceso correspondiente (terminal con `ssh`,
/// navegador con la URL, o visor VNC). Con SSH por contraseña, `ssh` la pide de
/// forma interactiva en la terminal; Karto no maneja el secreto al conectar.
/// Abre el cliente interactivo de BD (psql/mysql/mongosh/redis-cli) en una
/// terminal. El motor viene de la propiedad `gestor` del nodo; el secreto va por
/// variable de entorno (o en la URI de mongo). Solo Linux por ahora (mac/Windows
/// en Fase 7). Reutiliza el pipeline de resolución/armado de `usecases::scripts`.
/// Lanza el proceso de una conexión ya armada, registrando un warning de
/// diagnóstico si no arranca (binario ausente, permisos…). El error del SO no
/// incluye host ni secreto, así que es seguro registrarlo.
fn spawn_connection(spec: &LaunchSpec, node_id: &str, kind: &str) -> AppResult<()> {
    // SSH abre una terminal interactiva; web/VNC delegan en el abridor del SO.
    let mut command = if kind == ConnectionKind::Ssh.as_str() {
        terminal_command(&spec.program)
    } else {
        external_command(&spec.program)
    };
    command
        .args(&spec.args)
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            crate::usecases::diagnostics::warn(
                "connection",
                "launch_failed",
                &[
                    ("nodeId", node_id),
                    ("kind", kind),
                    ("program", &spec.program),
                    ("error", &e.to_string()),
                ],
            );
            AppError::Other(format!("no se pudo lanzar la conexión: {e}"))
        })
}

fn connect_db(conn: &Connection, node_id: &str, context_id: Option<&str>) -> AppResult<()> {
    use crate::usecases::scripts;
    if Os::current() != Os::Linux {
        return Err(AppError::Other(
            "la conexión a BD solo está disponible en Linux por ahora".into(),
        ));
    }
    let engine: String = conn
        .query_row(
            "SELECT value FROM node_properties WHERE node_id = ?1 AND key = 'gestor'",
            params![node_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::Other("el nodo de BD no tiene 'gestor' definido".into()))?;
    if !scripts::is_db_engine(&engine) {
        return Err(AppError::Other(format!(
            "motor de BD no soportado para conectar: {engine}"
        )));
    }

    let dbconn = scripts::resolve_db_target(conn, node_id, context_id, &engine)?;
    let spec = scripts::build_db_command(&engine, &dbconn, true)?; // interactivo
    if !program_in_path(&spec.program) {
        crate::usecases::diagnostics::warn(
            "connection",
            "db_client_missing",
            &[
                ("nodeId", node_id),
                ("engine", &engine),
                ("program", &spec.program),
            ],
        );
        return Err(AppError::Other(format!(
            "no se encontró el cliente '{}' en el PATH; instálalo",
            spec.program
        )));
    }

    // Cliente (programa + args) envuelto para mantener la terminal abierta al salir.
    let mut inner = Vec::with_capacity(spec.args.len() + 1);
    inner.push(spec.program.clone());
    inner.extend(spec.args.clone());
    let os = Os::current();
    let terminal = require_terminal(detect_terminal(terminals_for(os), program_in_path))?;
    let launch = wrap_in_terminal(terminal, &hold_wrapper(&inner, os));

    let mut command = terminal_command(&launch.program);
    command.args(&launch.args);
    // El secreto viaja por env (heredado por la terminal → cliente), no en argv.
    if let Some((k, v)) = &spec.env {
        command.env(k, v);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            crate::usecases::diagnostics::warn(
                "connection",
                "db_launch_failed",
                &[
                    ("nodeId", node_id),
                    ("engine", &engine),
                    ("program", &spec.program),
                    ("error", &e.to_string()),
                ],
            );
            AppError::Other(format!("no se pudo lanzar la conexión: {e}"))
        })
}

pub fn connect_node(
    conn: &Connection,
    node_id: &str,
    credential_id: Option<&str>,
    context_id: Option<&str>,
    templates_trusted: bool,
) -> AppResult<()> {
    // Credenciales de BD: abren el cliente interactivo (otra tubería, no SSH/URL).
    if load_credential(conn, node_id, credential_id)?.kind == "db" {
        return connect_db(conn, node_id, context_id);
    }
    let req = resolve(conn, node_id, credential_id, context_id)?;
    let os = Os::current();

    // Plantilla del vault (override del comando interno). Si existe para SSH en
    // Linux, **gana** y se salta el sondeo de facts (una plantilla custom manda:
    // podría reestructurar el comando de formas incompatibles con el multiplexado).
    if req.kind == ConnectionKind::Ssh && os == Os::Linux {
        if let Some(cmd) = crate::usecases::templates::vault_override(conn, "ssh", os.as_key())? {
            // Una plantilla embebida ejecuta shell arbitrario. Si el vault no está
            // marcado como de confianza en esta máquina (p. ej. importado de un
            // tercero), no la ejecutamos en silencio: pedimos confirmación al usuario.
            if !templates_trusted {
                return Err(AppError::TemplateConfirmationRequired(cmd));
            }
            let inner = crate::usecases::templates::render(&cmd, &req);
            if inner.is_empty() {
                return Err(AppError::Other(
                    "la plantilla de conexión quedó vacía tras sustituir los datos".into(),
                ));
            }
            let terminal = detect_terminal(terminals_for(os), program_in_path);
            let spec = wrap_in_terminal(require_terminal(terminal)?, &hold_wrapper(&inner, os));
            return spawn_connection(&spec, node_id, "ssh");
        }
    }

    // SSH en Linux: además de abrir la terminal, sondea el equipo (hostname, SO,
    // kernel…) y vuelca los datos a un archivo que el frontend lee luego.
    let spec = if req.kind == ConnectionKind::Ssh && os == Os::Linux {
        let facts_file = crate::usecases::facts::facts_file_path(node_id);
        let control = crate::usecases::facts::control_path(node_id);
        // Limpia un sondeo anterior para no leer datos rancios.
        let _ = std::fs::remove_file(&facts_file);
        let terminal = detect_terminal(terminals_for(os), program_in_path);
        build_ssh_with_facts(
            &req,
            terminal,
            &control.to_string_lossy(),
            &facts_file.to_string_lossy(),
        )?
    } else {
        plan(&req, os)?
    };
    spawn_connection(&spec, node_id, req.kind.as_str())
}

/// Abre la URL del nodo (`url_admin`/`url`) en el navegador del sistema, sin
/// necesidad de credencial: abrir un enlace no requiere el secreto de acceso.
pub fn open_node_url(conn: &Connection, node_id: &str) -> AppResult<()> {
    let url = node_url(conn, node_id)?
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| AppError::Other("el nodo no tiene URL configurada".into()))?;
    let spec = build_open_url(Os::current(), url.trim());
    external_command(&spec.program)
        .args(&spec.args)
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            crate::usecases::diagnostics::warn(
                "connection",
                "url_open_failed",
                &[
                    ("nodeId", node_id),
                    ("program", &spec.program),
                    ("error", &e.to_string()),
                ],
            );
            AppError::Other(format!("no se pudo abrir la URL: {e}"))
        })
}

/// Abre una URL externa (http/https) en el navegador del sistema. A diferencia
/// de `open_node_url`, no toca el vault: se usa para enlaces fijos de la app
/// (p. ej. la sección "Acerca de"). Se rechaza cualquier esquema que no sea
/// http/https para no delegar en el SO la apertura de esquemas arbitrarios.
pub fn open_external_url(url: &str) -> AppResult<()> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError::Other("solo se permiten enlaces http/https".into()));
    }
    let spec = build_open_url(Os::current(), url);
    external_command(&spec.program)
        .args(&spec.args)
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            crate::usecases::diagnostics::warn(
                "connection",
                "external_url_open_failed",
                &[("program", &spec.program), ("error", &e.to_string())],
            );
            AppError::Other(format!("no se pudo abrir la URL: {e}"))
        })
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
    fn facts_line_probes_then_reuses_master() {
        let line = ssh_facts_line(&req_ssh_key(), "/tmp/cm.sock", "/tmp/facts.txt");
        // Dos invocaciones de ssh separadas por `;`, la primera redirige al archivo.
        let (probe, inter) = line.split_once("; ssh ").unwrap();
        // Sondeo: establece el maestro, corre el script remoto y vuelca al archivo.
        assert!(probe.contains("ControlMaster=auto"));
        assert!(probe.contains("ControlPath=/tmp/cm.sock"));
        assert!(probe.contains("-i /home/me/.ssh/id_ed25519") && probe.contains("-p 2222"));
        assert!(probe.contains("root@10.0.0.5"));
        assert!(probe.contains(crate::usecases::facts::BEGIN));
        assert!(probe.trim_end().ends_with("> /tmp/facts.txt 2>/dev/null"));
        // Interactiva: reutiliza el socket, sin ControlMaster/script.
        assert!(inter.contains("ControlPath=/tmp/cm.sock"));
        assert!(!inter.contains("ControlMaster"));
        assert!(inter.trim_end().ends_with("root@10.0.0.5"));
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

    #[cfg(unix)]
    #[test]
    fn materialize_key_writes_0600_in_0700_dir_and_rejects_insecure_existing() {
        use std::os::unix::fs::PermissionsExt;
        let base = std::env::temp_dir().join(format!("karto-matkey-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("keys");
        let path = dir.join("id_ed25519");
        let path_str = path.to_string_lossy().to_string();

        // Materializa: crea dir 0700 y archivo 0600.
        materialize_key(&path_str, "PRIVATE").unwrap();
        assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        // Reusar un archivo ya seguro no falla.
        materialize_key(&path_str, "PRIVATE").unwrap();

        // Un archivo existente con permisos laxos se rechaza.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(materialize_key(&path_str, "PRIVATE").is_err());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn dangerous_ssh_options_are_detected() {
        // Directivas que ejecutan comandos locales (mayúsculas/espacios/`=` mezclados).
        assert!(is_dangerous_ssh_option("ProxyCommand=sh -c evil"));
        assert!(is_dangerous_ssh_option("proxycommand nc %h %p"));
        assert!(is_dangerous_ssh_option("LocalCommand touch /tmp/pwned"));
        assert!(is_dangerous_ssh_option("PermitLocalCommand=yes"));
        assert!(is_dangerous_ssh_option("KnownHostsCommand /x"));
        assert!(is_dangerous_ssh_option("Match exec \"/x\""));
        // Opciones legítimas: no se marcan.
        assert!(!is_dangerous_ssh_option("ServerAliveInterval=60"));
        assert!(!is_dangerous_ssh_option("ProxyJump bastion"));
        assert!(!is_dangerous_ssh_option("StrictHostKeyChecking=accept-new"));
        assert!(!is_dangerous_ssh_option("ConnectTimeout 10"));
    }

    #[test]
    fn parse_ssh_options_filters_dangerous_directives() {
        let raw = "ServerAliveInterval=60\nProxyCommand=sh -c evil\nProxyJump bastion\nLocalCommand x";
        assert_eq!(
            parse_ssh_options(Some(raw)),
            vec!["ServerAliveInterval=60", "ProxyJump bastion"]
        );
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
        assert!(build_ssh(&req_ssh_key(), None, Os::Linux).is_err());
    }

    #[test]
    fn build_ssh_wraps_in_bash_hold() {
        let term = TerminalDef { program: "kitty", exec: &[] };
        let spec = build_ssh(&req_ssh_key(), Some(&term), Os::Linux).unwrap();
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
        let spec = build_ssh(&req_ssh_key(), Some(&term), Os::Linux).unwrap();
        assert_eq!(spec.program, "konsole");
        assert_eq!(spec.args[0], "-e");
        assert_eq!(spec.args[1], "bash");
    }

    #[test]
    fn hold_wrapper_quotes_and_pauses() {
        let held = hold_wrapper(&["ssh".into(), "user@host name".into()], Os::Linux);
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
    fn external_url_rejects_non_http_schemes() {
        assert!(open_external_url("file:///etc/passwd").is_err());
        assert!(open_external_url("javascript:alert(1)").is_err());
        assert!(open_external_url("evesan.rocks").is_err());
    }

    #[test]
    fn vnc_opens_scheme_url_with_default_client_per_os() {
        let req = ConnectionRequest {
            kind: ConnectionKind::Vnc,
            user: None,
            host: "10.0.0.7".into(),
            port: Some(5901),
            key_path: None,
            url: None,
            ssh_options: Vec::new(),
        };
        // Linux delega en xdg-open con la URL vnc://host:puerto.
        let linux = build_vnc(&req, Os::Linux);
        assert_eq!(linux.program, "xdg-open");
        assert_eq!(linux.args, vec!["vnc://10.0.0.7:5901"]);
        // macOS usa `open` (lanza Screen Sharing).
        assert_eq!(build_vnc(&req, Os::Macos).program, "open");
        // Puerto por defecto 5900 y usuario incrustado en la autoridad si lo hay.
        let with_user = ConnectionRequest {
            user: Some("admin".into()),
            port: None,
            ..req
        };
        assert_eq!(build_vnc(&with_user, Os::Linux).args, vec!["vnc://admin@10.0.0.7:5900"]);
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
    fn resolve_ssh_uses_context_endpoint_and_default_credential() {
        let conn = seed();
        // Dirección por contexto: 'default' existe tras la migración.
        conn.execute(
            "INSERT INTO node_endpoints (node_id, context_id, address) VALUES ('n1','default','10.0.0.9')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO node_properties (node_id, key, value) VALUES ('n1','hostname','web.local')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO credentials (id, node_id, kind, username, secret, port, is_default, options) \
             VALUES ('c1','n1','ssh','root','pw',22,1,'ServerAliveInterval=60\nProxyJump bastion')",
            [],
        )
        .unwrap();

        let req = resolve(&conn, "n1", None, Some("default")).unwrap();
        assert_eq!(req.kind, ConnectionKind::Ssh);
        assert_eq!(req.host, "10.0.0.9"); // endpoint del contexto gana a hostname
        assert_eq!(req.user.as_deref(), Some("root"));
        assert_eq!(req.port, Some(22));
        assert_eq!(req.ssh_options, vec!["ServerAliveInterval=60", "ProxyJump bastion"]);
    }

    #[test]
    fn resolve_switches_address_per_context() {
        let conn = seed();
        conn.execute(
            "INSERT INTO access_contexts (id, name, position) VALUES ('vpn','VPN',1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO node_endpoints (node_id, context_id, address) \
             VALUES ('n1','default','10.0.0.9'),('n1','vpn','172.16.0.9')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO credentials (id, node_id, kind, is_default) VALUES ('c1','n1','ssh',1)",
            [],
        )
        .unwrap();
        assert_eq!(resolve(&conn, "n1", None, Some("default")).unwrap().host, "10.0.0.9");
        assert_eq!(resolve(&conn, "n1", None, Some("vpn")).unwrap().host, "172.16.0.9");
    }

    #[test]
    fn resolve_falls_back_to_hostname_when_no_endpoint() {
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
        // Sin endpoint en el contexto activo → respaldo al hostname.
        assert_eq!(resolve(&conn, "n1", None, Some("default")).unwrap().host, "web.local");
        // Sin contexto → también respaldo al hostname.
        assert_eq!(resolve(&conn, "n1", None, None).unwrap().host, "web.local");
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
        let req = resolve(&conn, "n1", None, None).unwrap();
        assert_eq!(req.kind, ConnectionKind::Web);
        assert_eq!(req.url.as_deref(), Some("https://p.local"));
        assert!(req.ssh_options.is_empty());
    }

    #[test]
    fn resolve_specific_credential_id() {
        let conn = seed();
        conn.execute(
            "INSERT INTO node_endpoints (node_id, context_id, address) VALUES ('n1','default','1.2.3.4')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO credentials (id, node_id, kind, username, is_default) \
             VALUES ('c1','n1','ssh','root',1),('c2','n1','ssh','deploy',0)",
            [],
        )
        .unwrap();
        assert_eq!(
            resolve(&conn, "n1", Some("c2"), Some("default")).unwrap().user.as_deref(),
            Some("deploy")
        );
    }

    #[test]
    fn resolve_errors_without_credentials() {
        let conn = seed();
        assert!(resolve(&conn, "n1", None, Some("default")).is_err());
    }

    #[test]
    fn resolve_ssh_errors_without_host() {
        let conn = seed();
        conn.execute(
            "INSERT INTO credentials (id, node_id, kind, is_default) VALUES ('c1','n1','ssh',1)",
            [],
        )
        .unwrap();
        assert!(resolve(&conn, "n1", None, Some("default")).is_err());
    }

    // --- Detección de ejecutables en el PATH (Fase 1 Windows) ---

    /// Predicado de "archivo existente" sobre una lista fija de rutas, para
    /// probar la búsqueda sin tocar disco ni depender del SO anfitrión.
    fn fake_fs<'a>(present: &'a [&'a str]) -> impl Fn(&Path) -> bool + 'a {
        move |p: &Path| present.iter().any(|f| Path::new(f) == p)
    }

    fn dirs(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn find_program_matches_bare_name_on_unix_exts() {
        let exts = vec![String::new()];
        assert!(find_program(
            "ssh",
            &dirs(&["/usr/bin"]),
            &exts,
            fake_fs(&["/usr/bin/ssh"])
        ));
    }

    // El caso que rompía Windows: el binario es `ssh.exe`, no `ssh`.
    #[test]
    fn find_program_appends_windows_extensions() {
        let bare = vec![String::new()];
        let windows = vec![String::new(), ".exe".into(), ".cmd".into()];
        let present = ["C:/Windows/System32/OpenSSH/ssh.exe"];
        let path = dirs(&["C:/Windows/System32/OpenSSH"]);

        assert!(!find_program("ssh", &path, &bare, fake_fs(&present)));
        assert!(find_program("ssh", &path, &windows, fake_fs(&present)));
    }

    // Un nombre que ya trae extensión se encuentra con la extensión vacía.
    #[test]
    fn find_program_accepts_name_with_extension() {
        let exts = vec![String::new(), ".exe".into()];
        assert!(find_program(
            "wt.exe",
            &dirs(&["C:/apps"]),
            &exts,
            fake_fs(&["C:/apps/wt.exe"])
        ));
    }

    #[test]
    fn find_program_scans_every_dir_and_reports_absence() {
        let exts = vec![String::new(), ".exe".into()];
        let path = dirs(&["/opt/bin", "/usr/bin"]);
        assert!(find_program("psql", &path, &exts, fake_fs(&["/usr/bin/psql"])));
        assert!(!find_program("mongosh", &path, &exts, fake_fs(&["/usr/bin/psql"])));
    }

    // Prueba de integración contra el PATH real: el intérprete de comandos del
    // SO siempre está. En Windows es `cmd.exe`, así que este test falla si se
    // vuelve a perder el manejo de `PATHEXT` (era el bug original).
    #[test]
    fn program_in_path_finds_the_system_shell() {
        let shell = if cfg!(windows) { "cmd" } else { "sh" };
        assert!(program_in_path(shell), "no se encontró '{shell}' en el PATH");
    }

    #[test]
    fn executable_exts_always_tries_the_bare_name_first() {
        let exts = executable_exts();
        assert_eq!(exts.first().map(String::as_str), Some(""));
        if cfg!(windows) {
            assert!(exts.iter().any(|e| e == ".exe"), "faltan las de PATHEXT: {exts:?}");
        } else {
            assert_eq!(exts.len(), 1, "en Unix no se añaden extensiones: {exts:?}");
        }
    }

    // --- Windows: terminal, quoting de PowerShell y hold (Fase 2) ---

    fn win_term() -> &'static TerminalDef {
        &WINDOWS_TERMINALS[0]
    }

    #[test]
    fn terminals_for_maps_each_os() {
        assert_eq!(terminals_for(Os::Linux).len(), LINUX_TERMINALS.len());
        assert_eq!(terminals_for(Os::Windows)[0].program, "powershell.exe");
        // macOS sigue sin soporte: lista vacía -> `require_terminal` explica.
        assert!(terminals_for(Os::Macos).is_empty());
        assert!(detect_terminal(terminals_for(Os::Macos), |_| true).is_none());
    }

    #[test]
    fn to_pwsh_line_uses_call_operator_and_quotes_every_token() {
        let argv: Vec<String> = vec!["ssh".into(), "-i".into(), "C:/mis llaves/id".into()];
        // Sin `&`, PowerShell trataría 'ssh' como cadena y la imprimiría.
        assert_eq!(to_pwsh_line(&argv), "& 'ssh' '-i' 'C:/mis llaves/id'");
        assert_eq!(to_pwsh_line(&[]), "");
    }

    // La comilla simple es el único carácter que puede cerrar el literal: se
    // duplica. Todo lo demás queda inerte dentro de comillas simples.
    #[test]
    fn pwsh_quote_neutralizes_injection_from_the_vault() {
        let hostile: Vec<String> = vec![
            "ssh".into(),
            "root@host'; calc.exe; #".into(),
            "$(rm -rf /)".into(),
            "a`b|c;d".into(),
        ];
        let line = to_pwsh_line(&hostile);
        assert_eq!(
            line,
            "& 'ssh' 'root@host''; calc.exe; #' '$(rm -rf /)' 'a`b|c;d'"
        );
        // Ninguna comilla simple queda suelta: siempre en pares.
        assert_eq!(line.matches('\'').count() % 2, 0);
    }

    #[test]
    fn hold_wrapper_on_windows_is_a_single_powershell_script() {
        let held = hold_wrapper(&["ssh".into(), "user@host name".into()], Os::Windows);
        // Un solo elemento: la terminal ya aporta `-NoProfile -Command`.
        assert_eq!(held.len(), 1, "{held:?}");
        assert!(held[0].starts_with("& 'ssh' 'user@host name'"));
        assert!(held[0].contains("$LASTEXITCODE"));
        assert!(held[0].contains("Read-Host"));
        // Nada de sintaxis POSIX filtrada.
        assert!(!held[0].contains("read -n1"));
        assert!(!held[0].contains("bash"));
    }

    #[test]
    fn build_ssh_on_windows_targets_powershell() {
        let spec = build_ssh(&req_ssh_key(), Some(win_term()), Os::Windows).unwrap();
        assert_eq!(spec.program, "powershell.exe");
        assert_eq!(spec.args[0], "-NoProfile");
        assert_eq!(spec.args[1], "-Command");
        assert_eq!(spec.args.len(), 3, "el script va en un solo argumento");
        let script = &spec.args[2];
        assert!(script.contains("& 'ssh' '-i' '/home/me/.ssh/id_ed25519' '-p' '2222' 'root@10.0.0.5'"));
    }

    // La regresión que motiva la fase: `plan` devolvía error para todo SSH en
    // Windows porque solo Linux buscaba terminal.
    #[test]
    #[cfg(windows)]
    fn plan_no_longer_fails_for_ssh_on_windows() {
        let spec = plan(&req_ssh_key(), Os::Windows).expect("SSH debe planificarse en Windows");
        assert_eq!(spec.program, "powershell.exe");
    }
}

/// Verificación end-to-end del lanzamiento en Windows. Abre una consola de
/// verdad, así que está marcada `#[ignore]`: se corre a mano con
/// `cargo test --lib windows_launch -- --ignored --nocapture`.
///
/// Comprueba de una pasada lo que los tests puros no pueden: que
/// `CREATE_NEW_CONSOLE` da ventana a una app GUI, que PowerShell acepta la línea
/// que arma `to_pwsh_line`, y que un argumento con comilla simple llega como
/// **dato** y no como código.
#[cfg(all(test, windows))]
mod windows_launch {
    use super::*;

    #[test]
    #[ignore = "abre una consola real"]
    fn spawns_a_visible_console_and_runs_the_command() {
        let out = std::env::temp_dir().join("karto-e2e.txt");
        let _ = std::fs::remove_file(&out);
        let inner: Vec<String> = vec![
            "cmd.exe".into(),
            "/c".into(),
            format!("echo hola d'a mundo> {}", out.display()),
        ];
        let term = detect_terminal(terminals_for(Os::Windows), program_in_path)
            .expect("powershell en el PATH");
        let spec = wrap_in_terminal(term, &hold_wrapper(&inner, Os::Windows));
        let mut child = terminal_command(&spec.program).args(&spec.args).spawn().unwrap();

        let mut got = None;
        for _ in 0..24 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            if let Ok(c) = std::fs::read_to_string(&out) {
                if !c.trim().is_empty() {
                    got = Some(c);
                    break;
                }
            }
        }
        // La consola queda esperando el Enter del hold: se cierra aquí.
        let _ = child.kill();
        assert_eq!(got.as_deref().map(str::trim), Some("hola d'a mundo"));
    }
}

/// Verificación end-to-end de los permisos de llave en Windows: comprueba que
/// una llave materializada por Karto en un directorio **hostil** (con `Everyone`
/// con acceso, heredado) resulta aceptable para el `ssh` del sistema.
///
/// Marcada `#[ignore]` porque depende de `ssh-keygen`/`ssh` instalados:
/// `cargo test --lib windows_key_perms -- --ignored --nocapture`.
#[cfg(all(test, windows))]
mod windows_key_perms {
    use super::*;

    /// `ssh-keygen` **del sistema**, no el que salga del `PATH`. Importa: si hay
    /// Git para Windows instalado, su `ssh-keygen` es un build MSYS que no hace la
    /// comprobación de ACL de Windows y aceptaría una llave que el OpenSSH del
    /// sistema rechaza. La app GUI hereda el PATH del sistema y usa este; el test
    /// debe medir el mismo binario.
    fn system_ssh_keygen() -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("SystemRoot").unwrap())
            .join(r"System32\OpenSSH\ssh-keygen.exe")
    }

    /// ¿`ssh` aceptaría esta llave? `ssh-keygen -y` hace la misma comprobación de
    /// permisos que `ssh` al cargar la identidad, sin necesitar servidor.
    fn ssh_accepts(key: &Path) -> (bool, String) {
        let out = std::process::Command::new(system_ssh_keygen())
            .args(["-y", "-f"])
            .arg(key)
            .output()
            .expect("ssh-keygen en el PATH");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (!combined.contains("UNPROTECTED"), combined)
    }

    #[test]
    #[ignore = "necesita ssh-keygen y toca ACLs reales"]
    fn key_written_into_a_hostile_dir_is_accepted_by_ssh() {
        let base = std::env::temp_dir().join(format!("karto-keyperm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        // Directorio hostil: Everyone con lectura, que se hereda a los hijos.
        let icacls = std::path::PathBuf::from(std::env::var("SystemRoot").unwrap())
            .join(r"System32\icacls.exe");
        let st = std::process::Command::new(&icacls)
            .arg(&base)
            .args(["/grant", "*S-1-1-0:(OI)(CI)(R)"])
            .output()
            .unwrap();
        assert!(st.status.success(), "no se pudo preparar el directorio hostil");

        // Material de llave real, generado aparte y leído como haría el vault.
        let seed = base.join("seed");
        std::process::Command::new(system_ssh_keygen())
            .args(["-t", "ed25519", "-N", "", "-C", "karto-test", "-q", "-f"])
            .arg(&seed)
            .status()
            .unwrap();
        let material = std::fs::read_to_string(&seed).unwrap();

        // Control: escrito "a pelo", hereda el Everyone del directorio.
        let ingenua = base.join("ingenua");
        std::fs::write(&ingenua, &material).unwrap();
        let (ok_ingenua, detalle) = ssh_accepts(&ingenua);
        assert!(!ok_ingenua, "el directorio hostil no lo fue: {detalle}");

        // Lo que hace Karto ahora.
        let gestionada = base.join("gestionada");
        materialize_key(&gestionada.to_string_lossy(), &material).unwrap();
        let (ok_gestionada, detalle) = ssh_accepts(&gestionada);
        assert!(ok_gestionada, "ssh rechazó la llave materializada: {detalle}");

        let _ = std::fs::remove_dir_all(&base);
    }
}
