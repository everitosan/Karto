//! Registro de diagnóstico para soporte. Escribe un `karto.log` en el directorio
//! de datos de la app (junto a `app.db`) con líneas de **warning** cuando algo
//! falla de forma capturable: no se pudo lanzar una conexión, falta el cliente de
//! BD, un script no pudo arrancar o terminó con error, etc. El objetivo es que un
//! usuario pueda **compartir ese archivo** y el problema se identifique sin pedirle
//! más datos.
//!
//! Privacidad: nunca se escriben secretos (contraseñas/llaves) ni direcciones
//! (host/IP). Los llamadores pasan solo campos no sensibles (id de nodo, tipo de
//! conexión, programa, mensaje de error) y, como red de seguridad, `sanitize`
//! redacta IPs y tokens `usuario@host` que pudieran colarse en un mensaje.
//!
//! El núcleo de formateo (`format_line`, `format_timestamp`, `sanitize`) es puro y
//! testeable; `warn` es el borde con efecto (append al archivo, best-effort: si el
//! IO falla nunca rompe el flujo que lo invoca).

use crate::error::{AppError, AppResult};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Tamaño máximo del log antes de rotar (~1 MB). Al superarlo, el archivo actual
/// se mueve a `karto.log.1` (sobrescribiendo el anterior) y se empieza uno nuevo.
/// Un solo respaldo basta: es un log de soporte, no una auditoría histórica.
const MAX_BYTES: u64 = 1_000_000;

/// Nivel de detalle del log. Ordenados por **severidad ascendente**: un nivel
/// activo deja pasar los eventos de su severidad **o mayor** (Info = todo;
/// Warning = warnings y errores; Error = solo errores).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

impl LogLevel {
    fn severity(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Error => 2,
        }
    }

    /// Clave textual estable (persistencia + puente al frontend).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    /// Etiqueta que se escribe en la línea de log.
    fn label(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// Umbral de severidad activo. Global de proceso porque el log se escribe desde
/// varios puntos e hilos del backend; un atómico evita enhebrar el nivel por toda
/// la pila de llamadas. Por defecto **Warning** (severidad 1).
static THRESHOLD: AtomicU8 = AtomicU8::new(1);

/// Fija el nivel activo (lo aplica en caliente; la persistencia la hace el caller).
pub fn set_level(level: LogLevel) {
    THRESHOLD.store(level.severity(), Ordering::Relaxed);
}

/// Nivel activo actual.
pub fn level() -> LogLevel {
    match THRESHOLD.load(Ordering::Relaxed) {
        0 => LogLevel::Info,
        2 => LogLevel::Error,
        _ => LogLevel::Warning,
    }
}

/// Ruta del log de diagnóstico (`~/.local/share/app.karto.desktop/karto.log` en
/// Linux). `None` si no se puede resolver el directorio de datos.
pub fn log_path() -> Option<PathBuf> {
    crate::usecases::app_store::data_dir().map(|d| d.join("karto.log"))
}

/// Abre en el gestor de archivos del SO la carpeta que contiene el log, para que
/// el usuario lo localice y comparta. Crea el directorio si aún no existe (primer
/// arranque sin logs) para que el gestor no falle. Linux-first (`xdg-open`).
pub fn open_log_dir() -> AppResult<()> {
    let path =
        log_path().ok_or_else(|| AppError::Other("no se pudo resolver la ruta del log".into()))?;
    let dir = path
        .parent()
        .ok_or_else(|| AppError::Other("la ruta del log no tiene directorio".into()))?;
    std::fs::create_dir_all(dir)?;
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program)
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::Other(format!("no se pudo abrir la carpeta del log: {e}")))
}

/// Segundos epoch actuales (0 si el reloj está antes de 1970, improbable).
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Registra un evento con nivel explícito y campos `clave=valor`. Filtra por el
/// umbral activo y es best-effort: cualquier fallo de IO se ignora (el diagnóstico
/// nunca debe tumbar la operación que lo genera).
pub fn log(level: LogLevel, component: &str, event: &str, fields: &[(&str, &str)]) {
    if level.severity() < THRESHOLD.load(Ordering::Relaxed) {
        return;
    }
    let line = format_line(now_secs(), level.label(), component, event, fields);
    let _ = append(&line);
}

/// Escribe la línea de **inicio de sesión** con datos del equipo (SO, arquitectura,
/// versión de la app, escritorio, locale, nivel de log activo y clientes/terminales
/// detectados en el PATH). Se escribe **siempre**, sin filtrar por nivel: es el
/// encabezado que da contexto a todo lo demás en un log que el usuario comparte.
/// Nada sensible: ni host, ni IP, ni usuario, ni secretos.
pub fn record_startup() {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let locale = std::env::var("LANG").unwrap_or_default();
    let tools = detect_tools().join(",");
    let line = format_line(
        now_secs(),
        LogLevel::Info.label(),
        "app",
        "startup",
        &[
            ("version", env!("CARGO_PKG_VERSION")),
            ("os", std::env::consts::OS),
            ("arch", std::env::consts::ARCH),
            ("desktop", &desktop),
            ("locale", &locale),
            ("logLevel", level().as_str()),
            ("tools", &tools),
        ],
    );
    let _ = append(&line);
}

/// Clientes de conexión y terminales soportadas presentes en el PATH. Saber cuáles
/// faltan explica de un vistazo muchos fallos de "no se pudo lanzar/conectar".
fn detect_tools() -> Vec<String> {
    use crate::domain::Os;
    use crate::usecases::connections::{program_in_path, terminals_for};
    const CLIENTS: &[&str] = &[
        "ssh",
        "ssh-copy-id",
        "xdg-open",
        "psql",
        "mysql",
        "mongosh",
        "redis-cli",
    ];
    let mut found: Vec<String> = CLIENTS
        .iter()
        .filter(|t| program_in_path(t))
        .map(|t| t.to_string())
        .collect();
    // Terminales del SO en el que corre: en Windows listar las de Linux no
    // decía nada (y con el bug de PATHEXT ni siquiera se detectaban).
    for term in terminals_for(Os::current()) {
        if program_in_path(term.program) {
            found.push(term.program.to_string());
        }
    }
    found
}

/// Atajo para eventos de severidad Warning (el caso habitual: algo falló pero es
/// recuperable / esperado). Para otras severidades usar `log` directamente.
pub fn warn(component: &str, event: &str, fields: &[(&str, &str)]) {
    log(LogLevel::Warning, component, event, fields);
}

/// Añade la línea al log, creando el directorio y rotando si hace falta.
fn append(line: &str) -> std::io::Result<()> {
    let Some(path) = log_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
        // Rota: conserva un único respaldo. Si el rename falla (p. ej. permisos),
        // se sigue escribiendo en el actual (no perdemos el mensaje por rotar).
        let _ = std::fs::rename(&path, path.with_extension("log.1"));
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    f.write_all(line.as_bytes())
}

/// Formatea una línea de log (núcleo puro). Forma:
/// `2026-08-23T14:05:09Z WARN [component] event key="valor" ...`.
/// Los valores se sanean y se citan con `{:?}` para que caracteres raros o saltos
/// de línea no rompan el formato de una línea por evento.
pub fn format_line(
    ts_secs: u64,
    level: &str,
    component: &str,
    event: &str,
    fields: &[(&str, &str)],
) -> String {
    let mut s = format!(
        "{} {} [{}] {}",
        format_timestamp(ts_secs),
        level,
        component,
        event
    );
    for (k, v) in fields {
        s.push_str(&format!(" {}={:?}", k, sanitize(v)));
    }
    s.push('\n');
    s
}

/// Red de seguridad de privacidad: redacta direcciones IPv4 y tokens con `@`
/// (`usuario@host`) que pudieran aparecer en un mensaje de error. Trabaja por
/// tokens separados por espacios para no destrozar el resto del texto.
pub fn sanitize(value: &str) -> String {
    value
        .split(' ')
        .map(|tok| {
            if tok.is_empty() {
                tok.to_string()
            } else if let Some((user, _host)) = tok.split_once('@') {
                // usuario@host → conserva el usuario, oculta el host/dirección.
                format!("{user}@<redacted>")
            } else if is_ipv4(tok) {
                "<ip>".to_string()
            } else {
                tok.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// ¿El token es exactamente una dirección IPv4 (cuatro grupos 0-255)? Tolera
/// puntuación final (`,` `.` `:` `)`), común al final de una frase o `host:port`.
fn is_ipv4(tok: &str) -> bool {
    let core = tok.trim_end_matches([',', '.', ':', ')', ';']);
    let core = core.split(':').next().unwrap_or(core); // descarta un `:puerto`
    let mut groups = core.split('.');
    let mut count = 0;
    for g in &mut groups {
        count += 1;
        if count > 4 || g.is_empty() || g.len() > 3 || !g.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        if g.parse::<u16>().map(|n| n > 255).unwrap_or(true) {
            return false;
        }
    }
    count == 4
}

/// Marca de tiempo UTC en formato ISO-8601 (`YYYY-MM-DDTHH:MM:SSZ`) a partir de
/// segundos epoch, sin dependencias externas.
pub fn format_timestamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3_600, (rem % 3_600) / 60, rem % 60);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Convierte días desde 1970-01-01 a `(año, mes, día)` (algoritmo de H. Hinnant).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (y + if m <= 2 { 1 } else { 0 }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_formats_known_epoch() {
        // 2026-08-23T00:00:00Z = 1_787_443_200 (día exacto).
        assert_eq!(format_timestamp(1_787_443_200), "2026-08-23T00:00:00Z");
        // Epoch cero.
        assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
        // Con hora/minuto/segundo.
        assert_eq!(format_timestamp(1_787_443_200 + 3_661), "2026-08-23T01:01:01Z");
    }

    #[test]
    fn sanitize_redacts_ip_and_userhost() {
        assert_eq!(sanitize("fallo en root@10.0.0.5 ahora"), "fallo en root@<redacted> ahora");
        assert_eq!(sanitize("conexión a 192.168.1.20 rechazada"), "conexión a <ip> rechazada");
        // IP con puntuación final.
        assert_eq!(sanitize("destino 10.1.2.3:22, sin ruta"), "destino <ip> sin ruta");
    }

    #[test]
    fn sanitize_leaves_safe_text_untouched() {
        let msg = "no se pudo lanzar la conexión: No such file or directory (os error 2)";
        assert_eq!(sanitize(msg), msg);
    }

    #[test]
    fn is_ipv4_rejects_non_addresses() {
        assert!(is_ipv4("10.0.0.5"));
        assert!(!is_ipv4("10.0.0")); // faltan grupos
        assert!(!is_ipv4("999.0.0.1")); // fuera de rango
        assert!(!is_ipv4("web.local")); // no numérico
        assert!(!is_ipv4("1.2.3.4.5")); // sobran grupos
    }

    #[test]
    fn format_line_has_timestamp_component_and_quoted_fields() {
        let line = format_line(
            0,
            "WARN",
            "connection",
            "launch_failed",
            &[("nodeId", "n1"), ("program", "ssh")],
        );
        assert_eq!(
            line,
            "1970-01-01T00:00:00Z WARN [connection] launch_failed nodeId=\"n1\" program=\"ssh\"\n"
        );
    }

    #[test]
    fn format_line_keeps_multiline_error_on_one_line() {
        let line = format_line(0, "ERROR", "script", "target_error", &[("error", "a\nb")]);
        assert!(line.matches('\n').count() == 1); // solo el salto final
        assert!(line.contains("error=\"a\\nb\""));
    }

    #[test]
    fn record_startup_writes_environment_header() {
        // Apunta el dir de datos a un temporal propio (ningún otro test lee
        // `data_dir`, así que fijar la env aquí es seguro).
        let dir = std::env::temp_dir().join(format!("karto-diag-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("XDG_DATA_HOME", &dir);

        record_startup();

        let log = log_path().expect("ruta de log");
        let content = std::fs::read_to_string(&log).expect("log escrito");
        assert!(content.contains("[app] startup"), "contenido: {content}");
        assert!(content.contains("os=") && content.contains("arch="));
        assert!(content.contains("version=") && content.contains("logLevel="));
        // Nunca datos sensibles en el encabezado.
        assert!(!content.to_lowercase().contains("password"));

        std::env::remove_var("XDG_DATA_HOME");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn log_level_roundtrips_and_orders_by_severity() {
        assert_eq!(LogLevel::from_str("warning"), Some(LogLevel::Warning));
        assert_eq!(LogLevel::from_str("nope"), None);
        assert_eq!(LogLevel::Error.as_str(), "error");
        // Severidad: Info < Warning < Error.
        assert!(LogLevel::Info.severity() < LogLevel::Warning.severity());
        assert!(LogLevel::Warning.severity() < LogLevel::Error.severity());
    }
}
