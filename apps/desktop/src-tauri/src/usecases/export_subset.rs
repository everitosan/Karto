//! Export selectivo: copia un subconjunto de nodos (los seleccionados en el
//! lienzo) y las aristas entre ellos del vault abierto a un `.karto` nuevo,
//! cifrado con **otra** contraseña (para compartir sin revelar la maestra).
//!
//! El núcleo (`copy_subset`) opera sobre dos conexiones ya abiertas (origen y
//! destino, ambas migradas) → testeable con dos DBs in-memory, sin SQLCipher.
//! La creación cifrada del destino la hace `VaultService::export_subset`.

use crate::error::AppResult;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

/// Qué datos sensibles/opcionales incluir en el export. La identidad del nodo
/// (etiqueta, tipo, posición, hostname y propiedades genéricas) siempre viaja.
#[derive(Debug, Clone, Copy)]
pub struct ExportOptions {
    /// Credenciales del nodo (usuario, secreto, llave privada…).
    pub credentials: bool,
    /// Metadata del equipo sondeada por SSH (SO, kernel, recursos, uptime…).
    pub facts: bool,
    /// Direcciones por contexto (`node_endpoints`).
    pub ip: bool,
    /// Notas libres del nodo.
    pub notes: bool,
}

/// Propiedades que provienen del sondeo (facts). Se excluyen si `!opts.facts`.
const FACTS_KEYS: &[&str] = &["os", "recursos", "kernel", "arch", "uptime", "virt"];
/// Clave de la propiedad de notas.
const NOTES_KEY: &str = "notas";

/// Copia los `node_ids` (y las aristas cuyos dos extremos estén en el conjunto)
/// del vault `src` a `dest`, dentro de un mapa nuevo llamado `map_name`. Filtra
/// el contenido según `opts`. Envuelto en una transacción del destino.
pub fn copy_subset(
    src: &Connection,
    dest: &Connection,
    node_ids: &[String],
    map_name: &str,
    opts: ExportOptions,
) -> AppResult<()> {
    let set: HashSet<&str> = node_ids.iter().map(String::as_str).collect();
    let tx = dest.unchecked_transaction()?;

    // Mapa contenedor de todos los nodos exportados.
    let map_id: String =
        dest.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
    dest.execute(
        "INSERT INTO maps (id, name) VALUES (?1, ?2)",
        params![map_id, map_name],
    )?;

    // 1) Nodos (parent_id se rellena en un segundo paso para no violar la FK).
    for id in node_ids {
        let row = src
            .query_row(
                "SELECT kind, label, x, y FROM nodes WHERE id = ?1",
                params![id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, f64>(2)?,
                        r.get::<_, f64>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((kind, label, x, y)) = row else {
            continue; // nodo inexistente: se ignora
        };
        dest.execute(
            "INSERT INTO nodes (id, map_id, kind, label, x, y) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, map_id, kind, label, x, y],
        )?;

        // Propiedades filtradas por las opciones de contenido.
        let mut stmt =
            src.prepare("SELECT key, value FROM node_properties WHERE node_id = ?1")?;
        let props = stmt.query_map(params![id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        for prop in props {
            let (key, value) = prop?;
            let include = if FACTS_KEYS.contains(&key.as_str()) {
                opts.facts
            } else if key == NOTES_KEY {
                opts.notes
            } else {
                true
            };
            if include {
                dest.execute(
                    "INSERT INTO node_properties (node_id, key, value) VALUES (?1, ?2, ?3)",
                    params![id, key, value],
                )?;
            }
        }

        // Direcciones por contexto (opcional). Copia también el contexto para no
        // romper la FK (INSERT OR IGNORE: 'default' ya existe por la migración).
        if opts.ip {
            let mut es =
                src.prepare("SELECT context_id, address FROM node_endpoints WHERE node_id = ?1")?;
            let eps = es.query_map(params![id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            for ep in eps {
                let (context_id, address) = ep?;
                if let Some((name, position)) = src
                    .query_row(
                        "SELECT name, position FROM access_contexts WHERE id = ?1",
                        params![context_id],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
                    )
                    .optional()?
                {
                    dest.execute(
                        "INSERT OR IGNORE INTO access_contexts (id, name, position) VALUES (?1, ?2, ?3)",
                        params![context_id, name, position],
                    )?;
                }
                dest.execute(
                    "INSERT INTO node_endpoints (node_id, context_id, address) VALUES (?1, ?2, ?3)",
                    params![id, context_id, address],
                )?;
            }
        }

        // Credenciales (opcional). Viajan con su secreto/llave (el .karto va cifrado).
        if opts.credentials {
            let mut cs = src.prepare(
                "SELECT id, kind, username, secret, port, key_path, is_default, extras, options, private_key
                 FROM credentials WHERE node_id = ?1",
            )?;
            let creds = cs.query_map(params![id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                ))
            })?;
            for cred in creds {
                let c = cred?;
                dest.execute(
                    "INSERT INTO credentials
                        (id, node_id, kind, username, secret, port, key_path, is_default, extras, options, private_key)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![c.0, id, c.1, c.2, c.3, c.4, c.5, c.6, c.7, c.8, c.9],
                )?;
            }
        }
    }

    // 2) parent_id: solo si el padre también se exportó (si no, queda NULL).
    for id in node_ids {
        let parent: Option<String> = src
            .query_row(
                "SELECT parent_id FROM nodes WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        if let Some(pid) = parent {
            if set.contains(pid.as_str()) {
                dest.execute(
                    "UPDATE nodes SET parent_id = ?1 WHERE id = ?2",
                    params![pid, id],
                )?;
            }
        }
    }

    // 3) Aristas cuyos dos extremos están en la selección.
    let mut stmt = src.prepare(
        "SELECT id, source_id, target_id, label, style FROM edges
         WHERE source_id = ?1 OR target_id = ?1",
    )?;
    let mut seen: HashSet<String> = HashSet::new();
    for id in node_ids {
        let edges = stmt.query_map(params![id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        for edge in edges {
            let (eid, source, target, label, style) = edge?;
            if set.contains(source.as_str()) && set.contains(target.as_str()) && seen.insert(eid.clone())
            {
                dest.execute(
                    "INSERT INTO edges (id, map_id, source_id, target_id, label, style)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![eid, map_id, source, target, label, style],
                )?;
            }
        }
    }

    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::migrations;

    fn migrated() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrations::run(&c).unwrap();
        c
    }

    /// Origen con nodos a-b-c-d-e-f y aristas consecutivas, más props/cred/endpoint.
    fn src_graph() -> Connection {
        let c = migrated();
        c.execute("INSERT INTO maps (id, name) VALUES ('m1','Infra')", []).unwrap();
        for n in ["a", "b", "c", "d", "e", "f"] {
            c.execute(
                "INSERT INTO nodes (id, map_id, kind, label, x, y) VALUES (?1,'m1','server',?1,0,0)",
                params![n],
            )
            .unwrap();
        }
        // Aristas a-b, b-c, c-d, d-e, e-f.
        for (i, (s, t)) in [("a", "b"), ("b", "c"), ("c", "d"), ("d", "e"), ("e", "f")]
            .iter()
            .enumerate()
        {
            c.execute(
                "INSERT INTO edges (id, map_id, source_id, target_id) VALUES (?1,'m1',?2,?3)",
                params![format!("e{i}"), s, t],
            )
            .unwrap();
        }
        // A 'c' le damos hostname (siempre), virt (fact), notas, endpoint y credencial.
        c.execute("INSERT INTO node_properties (node_id,key,value) VALUES ('c','hostname','c.local')", []).unwrap();
        c.execute("INSERT INTO node_properties (node_id,key,value) VALUES ('c','virt','kvm')", []).unwrap();
        c.execute("INSERT INTO node_properties (node_id,key,value) VALUES ('c','notas','secreta')", []).unwrap();
        c.execute("INSERT INTO node_endpoints (node_id,context_id,address) VALUES ('c','default','10.0.0.3')", []).unwrap();
        c.execute(
            "INSERT INTO credentials (id,node_id,kind,username,secret,is_default) VALUES ('cr1','c','ssh','root','pw',1)",
            [],
        )
        .unwrap();
        c
    }

    fn count(c: &Connection, sql: &str) -> i64 {
        c.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn copies_selected_nodes_and_only_internal_edges() {
        let src = src_graph();
        let dest = migrated();
        let opts = ExportOptions { credentials: true, facts: true, ip: true, notes: true };
        copy_subset(&src, &dest, &["b".into(), "c".into(), "d".into()], "Sub", opts).unwrap();

        // 3 nodos; solo las aristas b-c y c-d (a-b y d-e quedan fuera).
        assert_eq!(count(&dest, "SELECT count(*) FROM nodes"), 3);
        assert_eq!(count(&dest, "SELECT count(*) FROM edges"), 2);
        assert_eq!(count(&dest, "SELECT count(*) FROM maps"), 1);
        // El mapa nuevo se llama como se pidió.
        let name: String = dest.query_row("SELECT name FROM maps", [], |r| r.get(0)).unwrap();
        assert_eq!(name, "Sub");
    }

    #[test]
    fn content_options_filter_sensitive_fields() {
        let src = src_graph();
        let dest = migrated();
        // Sin credenciales, sin facts, sin ip, sin notas.
        let opts = ExportOptions { credentials: false, facts: false, ip: false, notes: false };
        copy_subset(&src, &dest, &["c".into()], "Sub", opts).unwrap();

        assert_eq!(count(&dest, "SELECT count(*) FROM credentials"), 0);
        assert_eq!(count(&dest, "SELECT count(*) FROM node_endpoints"), 0);
        // hostname (identidad) se mantiene; virt (fact) y notas se excluyen.
        assert_eq!(count(&dest, "SELECT count(*) FROM node_properties WHERE key='hostname'"), 1);
        assert_eq!(count(&dest, "SELECT count(*) FROM node_properties WHERE key='virt'"), 0);
        assert_eq!(count(&dest, "SELECT count(*) FROM node_properties WHERE key='notas'"), 0);
    }

    #[test]
    fn content_options_include_sensitive_fields_when_enabled() {
        let src = src_graph();
        let dest = migrated();
        let opts = ExportOptions { credentials: true, facts: true, ip: true, notes: true };
        copy_subset(&src, &dest, &["c".into()], "Sub", opts).unwrap();

        assert_eq!(count(&dest, "SELECT count(*) FROM credentials"), 1);
        assert_eq!(count(&dest, "SELECT count(*) FROM node_endpoints"), 1);
        assert_eq!(count(&dest, "SELECT count(*) FROM node_properties WHERE key='virt'"), 1);
        assert_eq!(count(&dest, "SELECT count(*) FROM node_properties WHERE key='notas'"), 1);
        // El secreto viaja (el .karto va cifrado).
        let secret: String = dest.query_row("SELECT secret FROM credentials WHERE id='cr1'", [], |r| r.get(0)).unwrap();
        assert_eq!(secret, "pw");
    }

    #[test]
    fn keeps_parent_only_when_parent_is_selected() {
        let src = migrated();
        src.execute("INSERT INTO maps (id,name) VALUES ('m1','M')", []).unwrap();
        src.execute("INSERT INTO nodes (id,map_id,kind,label) VALUES ('z','m1','zone','Zona')", []).unwrap();
        src.execute("INSERT INTO nodes (id,map_id,kind,label,parent_id) VALUES ('n','m1','server','N','z')", []).unwrap();

        // Exportar solo el hijo: parent_id debe quedar NULL (el padre no viaja).
        let dest = migrated();
        let opts = ExportOptions { credentials: false, facts: false, ip: false, notes: false };
        copy_subset(&src, &dest, &["n".into()], "Sub", opts).unwrap();
        let parent: Option<String> = dest.query_row("SELECT parent_id FROM nodes WHERE id='n'", [], |r| r.get(0)).unwrap();
        assert_eq!(parent, None);

        // Exportar ambos: se conserva la relación.
        let dest2 = migrated();
        copy_subset(&src, &dest2, &["n".into(), "z".into()], "Sub", opts).unwrap();
        let parent2: Option<String> = dest2.query_row("SELECT parent_id FROM nodes WHERE id='n'", [], |r| r.get(0)).unwrap();
        assert_eq!(parent2.as_deref(), Some("z"));
    }
}
