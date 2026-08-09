//! Caso de uso: importar hosts desde `~/.ssh/config` para acelerar la adopción.
//! El parseo recibe un lector para testearlo sin el archivo real del usuario.

use crate::domain::ImportedHost;
use crate::error::{AppError, AppResult};
use ssh2_config::{ParseRule, SshConfig};
use std::io::BufRead;
use std::path::PathBuf;

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

/// Ruta por defecto del archivo de configuración SSH del usuario actual.
pub fn default_config_path() -> Option<PathBuf> {
    dirs_home().map(|home| home.join(".ssh").join("config"))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
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
}
