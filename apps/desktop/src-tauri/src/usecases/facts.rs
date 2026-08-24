//! Sondeo de datos del equipo al conectar por SSH. La conexión interactiva
//! (`connections`) corre un pequeño script remoto **antes** de abrir la sesión y
//! vuelca su salida a un archivo local; aquí se parsea y se fusiona en las
//! propiedades del nodo. Núcleos puros (script + parseo) testeables sin red.

use crate::error::AppResult;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;

pub const BEGIN: &str = "__KARTO_FACTS_BEGIN__";
pub const END: &str = "__KARTO_FACTS_END__";

/// Script remoto (una sola línea) que imprime `clave=valor` entre marcadores.
/// Sin comillas simples a propósito, para que el quoting al incrustarlo como un
/// único argumento de `ssh` sea trivial. Cada dato tolera su ausencia (`2>/dev/null`).
pub fn remote_script() -> String {
    [
        format!("echo {BEGIN}"),
        "echo hostname=$(hostname 2>/dev/null)".into(),
        "echo os=$(. /etc/os-release 2>/dev/null; echo $PRETTY_NAME)".into(),
        "echo kernel=$(uname -r 2>/dev/null)".into(),
        "echo arch=$(uname -m 2>/dev/null)".into(),
        "echo cpus=$(nproc 2>/dev/null)".into(),
        "echo ram_kb=$(grep MemTotal /proc/meminfo 2>/dev/null | tr -dc 0-9)".into(),
        "echo disk_gb=$(df -BG --output=size / 2>/dev/null | tail -1 | tr -dc 0-9)".into(),
        "echo uptime=$(uptime -p 2>/dev/null)".into(),
        "echo virt=$(systemd-detect-virt 2>/dev/null)".into(),
        format!("echo {END}"),
    ]
    .join("; ")
}

/// Parsea la salida del sondeo. Devuelve `None` si aún no está el marcador de
/// fin (el archivo se escribe progresivamente ⇒ el caller sigue sondeando).
/// Normaliza `ram_kb` a un `ram` legible y descarta valores vacíos.
pub fn parse_facts(output: &str) -> Option<HashMap<String, String>> {
    let start = output.find(BEGIN)?;
    let rest = &output[start + BEGIN.len()..];
    let end = rest.find(END)?; // sin marcador de fin: aún incompleto.
    let block = &rest[..end];

    // Primero recogemos los valores crudos, luego derivamos las propiedades que
    // casan con el catálogo (p. ej. `recursos` = "CPU / RAM / disco").
    let mut raw: HashMap<&str, String> = HashMap::new();
    for line in block.lines() {
        let line = line.trim().trim_end_matches('\r');
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        if !value.is_empty() {
            raw.insert(
                match key {
                    "hostname" | "os" | "kernel" | "arch" | "cpus" | "ram_kb" | "disk_gb"
                    | "uptime" | "virt" => key,
                    _ => continue,
                },
                value.to_string(),
            );
        }
    }

    let mut facts = HashMap::new();
    // Propiedades sugeridas del catálogo (caen en su campo, no en "otras").
    for key in ["hostname", "os"] {
        if let Some(v) = raw.get(key) {
            facts.insert(key.to_string(), v.clone());
        }
    }
    // `recursos` = CPU · RAM · disco, con las piezas que haya (campo del catálogo).
    let mut recursos = Vec::new();
    if let Some(c) = raw.get("cpus") {
        recursos.push(format!("{c} CPU"));
    }
    if let Some(r) = raw.get("ram_kb").and_then(|kb| human_ram(kb)) {
        recursos.push(format!("{r} RAM"));
    }
    if let Some(d) = raw.get("disk_gb") {
        recursos.push(format!("{d} GiB disco"));
    }
    if !recursos.is_empty() {
        facts.insert("recursos".to_string(), recursos.join(" · "));
    }
    // Datos extra (útiles aunque no tengan campo dedicado en el catálogo).
    for key in ["kernel", "arch", "uptime"] {
        if let Some(v) = raw.get(key) {
            facts.insert(key.to_string(), v.clone());
        }
    }
    if let Some(v) = raw.get("virt") {
        let v = if v == "none" { "bare-metal" } else { v.as_str() };
        facts.insert("virt".to_string(), v.to_string());
    }
    Some(facts)
}

/// KiB (entero) → "X.Y GiB". Devuelve `None` si no es numérico.
fn human_ram(kb: &str) -> Option<String> {
    let kb: f64 = kb.parse().ok()?;
    Some(format!("{:.1} GiB", kb / 1_048_576.0))
}

/// Ruta del archivo local donde la terminal vuelca el sondeo de un nodo.
pub fn facts_file_path(node_id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("karto-facts-{node_id}.txt"))
}

/// Ruta del socket de multiplexado SSH (`ControlPath`) para un nodo.
pub fn control_path(node_id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("karto-cm-{node_id}.sock"))
}

/// Lee el archivo de sondeo del nodo; si está completo, fusiona los datos en las
/// propiedades del nodo (upsert, sin tocar otras claves), borra el archivo y
/// devuelve los datos. Si aún no hay archivo o está incompleto, devuelve `None`.
pub fn poll(conn: &Connection, node_id: &str) -> AppResult<Option<HashMap<String, String>>> {
    let path = facts_file_path(node_id);
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    let Some(facts) = parse_facts(&content) else {
        return Ok(None);
    };

    {
        let mut stmt = conn.prepare(
            "INSERT INTO node_properties (node_id, key, value) VALUES (?1, ?2, ?3) \
             ON CONFLICT(node_id, key) DO UPDATE SET value = excluded.value",
        )?;
        for (key, value) in &facts {
            stmt.execute(params![node_id, key, value])?;
        }
    }
    let _ = std::fs::remove_file(&path);
    Ok(Some(facts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_script_has_markers_and_no_single_quotes() {
        let s = remote_script();
        assert!(s.contains(BEGIN) && s.contains(END));
        assert!(!s.contains('\''), "el script no debe usar comillas simples");
    }

    #[test]
    fn parse_extracts_block_and_normalizes() {
        let out = format!(
            "garbage before\n{BEGIN}\r\nhostname=web01\nos=Debian GNU/Linux 12\nkernel=6.1.0-18-amd64\n\
             arch=x86_64\ncpus=4\nram_kb=8192000\ndisk_gb=40\nuptime=up 3 days\nvirt=kvm\nempty=\n{END}\ntrailing"
        );
        let f = parse_facts(&out).unwrap();
        assert_eq!(f.get("hostname").unwrap(), "web01");
        assert_eq!(f.get("os").unwrap(), "Debian GNU/Linux 12");
        // CPU/RAM/disco se combinan en el campo `recursos` del catálogo, no sueltos.
        assert_eq!(f.get("recursos").unwrap(), "4 CPU · 7.8 GiB RAM · 40 GiB disco");
        assert!(!f.contains_key("cpus") && !f.contains_key("ram") && !f.contains_key("ram_kb"));
        assert_eq!(f.get("kernel").unwrap(), "6.1.0-18-amd64");
        assert!(!f.contains_key("empty"), "los valores vacíos se descartan");
    }

    #[test]
    fn recursos_omits_missing_pieces() {
        // Solo CPU disponible ⇒ recursos sin RAM ni disco.
        let out = format!("{BEGIN}\ncpus=2\n{END}");
        assert_eq!(parse_facts(&out).unwrap().get("recursos").unwrap(), "2 CPU");
    }

    #[test]
    fn virt_none_becomes_bare_metal() {
        let out = format!("{BEGIN}\nvirt=none\n{END}");
        assert_eq!(parse_facts(&out).unwrap().get("virt").unwrap(), "bare-metal");
    }

    #[test]
    fn incomplete_without_end_marker_is_none() {
        let out = format!("{BEGIN}\nhostname=web01\n");
        assert!(parse_facts(&out).is_none());
    }

    #[test]
    fn poll_upserts_into_node_properties() {
        use crate::infra::migrations;
        use crate::usecases::workspace;
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        let map = workspace::map_create(&conn, "Red", None).unwrap();
        let node = workspace::node_create(&conn, &map.id, "server", "Web", 0.0, 0.0).unwrap();

        // Propiedad previa que NO debe perderse al fusionar.
        let mut existing = HashMap::new();
        existing.insert("notas".to_string(), "prod".to_string());
        workspace::node_set_properties(&conn, &node.id, &existing).unwrap();

        let path = facts_file_path(&node.id);
        std::fs::write(&path, format!("{BEGIN}\nhostname=web01\nkernel=6.1\n{END}")).unwrap();

        let facts = poll(&conn, &node.id).unwrap().unwrap();
        assert_eq!(facts.get("hostname").unwrap(), "web01");
        assert!(!path.exists(), "el archivo se borra tras leerlo");

        // El grafo refleja la fusión: facts + la propiedad previa.
        let graph = workspace::graph_load(&conn, &map.id).unwrap();
        let props = &graph.nodes[0].properties;
        assert_eq!(props.get("hostname").unwrap(), "web01");
        assert_eq!(props.get("kernel").unwrap(), "6.1");
        assert_eq!(props.get("notas").unwrap(), "prod");
    }
}
