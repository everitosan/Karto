//! Casos de uso del vault: crear, desbloquear, bloquear, consultar estado.
//! Es el único dueño de la conexión descifrada; el frontend nunca la ve.
//! Depende del puerto `VaultStore`, no de SQLCipher.

use crate::domain::ports::VaultStore;
use crate::domain::{VaultInfo, VaultStatus};
use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
struct Session {
    path: Option<PathBuf>,
    conn: Option<Connection>,
}

pub struct VaultService<S: VaultStore> {
    store: S,
    session: Mutex<Session>,
}

impl<S: VaultStore> VaultService<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            session: Mutex::new(Session::default()),
        }
    }

    pub fn status(&self) -> VaultInfo {
        let session = self.session.lock().unwrap();
        let status = match (&session.path, &session.conn) {
            (_, Some(_)) => VaultStatus::Unlocked,
            (Some(_), None) => VaultStatus::Locked,
            (None, None) => VaultStatus::NoVault,
        };
        VaultInfo {
            path: session.path.as_ref().map(|p| p.display().to_string()),
            status,
        }
    }

    pub fn create(&self, path: &Path, password: &str) -> AppResult<VaultInfo> {
        let conn = self.store.create(path, password)?;
        self.set_open(path, conn);
        Ok(self.status())
    }

    pub fn unlock(&self, path: &Path, password: &str) -> AppResult<VaultInfo> {
        let conn = self.store.open(path, password)?;
        self.set_open(path, conn);
        Ok(self.status())
    }

    pub fn lock(&self) -> VaultInfo {
        self.session.lock().unwrap().conn = None; // mantiene el path como "locked"
        self.status()
    }

    /// Cierra el vault por completo: olvida la conexión **y** el path, volviendo
    /// al estado `NoVault` (la UI regresa a la selección de vaults). A diferencia
    /// de `lock`, no deja el archivo "recordado" para re-desbloquear.
    pub fn close(&self) -> VaultInfo {
        {
            let mut session = self.session.lock().unwrap();
            session.conn = None;
            session.path = None;
        }
        self.status()
    }

    /// Punto único de acceso a la conexión descifrada para el resto de casos de
    /// uso (repos de nodos, credenciales, etc.); falla si el vault está bloqueado.
    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let session = self.session.lock().unwrap();
        let conn = session.conn.as_ref().ok_or(AppError::NoVaultOpen)?;
        f(conn)
    }

    /// Cambia la contraseña maestra re-cifrando el vault en sitio (`PRAGMA rekey`).
    /// Verifica primero la contraseña actual abriendo una conexión efímera al
    /// mismo archivo; si no coincide, aborta sin tocar nada.
    pub fn rekey(&self, current: &str, new: &str) -> AppResult<()> {
        let session = self.session.lock().unwrap();
        let path = session.path.clone().ok_or(AppError::NoVaultOpen)?;
        // Verificación: si la contraseña actual es incorrecta, esto falla.
        drop(self.store.open(&path, current)?);
        let conn = session.conn.as_ref().ok_or(AppError::NoVaultOpen)?;
        conn.pragma_update(None, "rekey", new)?;
        Ok(())
    }

    /// Escribe una copia de respaldo del vault (cifrada con la clave actual) en
    /// `dest` mediante `VACUUM INTO`. Falla si el destino ya existe.
    pub fn export(&self, dest: &Path) -> AppResult<()> {
        if dest.exists() {
            return Err(AppError::Other("ya existe un archivo en esa ruta".into()));
        }
        self.with_conn(|conn| {
            // VACUUM INTO no admite parámetros ligados; escapamos las comillas.
            let dest_sql = dest.to_string_lossy().replace('\'', "''");
            conn.execute_batch(&format!("VACUUM INTO '{dest_sql}';"))?;
            Ok(())
        })
    }

    /// Exporta un subconjunto de nodos del vault abierto a un `.karto` nuevo,
    /// cifrado con `password` (distinta de la maestra, para compartir). Crea el
    /// destino con el store (migrado) y copia la selección con `copy_subset`;
    /// si la copia falla, borra el archivo a medias.
    pub fn export_subset(
        &self,
        dest: &Path,
        password: &str,
        node_ids: &[String],
        map_name: &str,
        opts: crate::usecases::export_subset::ExportOptions,
    ) -> AppResult<()> {
        let session = self.session.lock().unwrap();
        let src = session.conn.as_ref().ok_or(AppError::NoVaultOpen)?;
        let dest_conn = self.store.create(dest, password)?;
        let result =
            crate::usecases::export_subset::copy_subset(src, &dest_conn, node_ids, map_name, opts);
        if result.is_err() {
            drop(dest_conn);
            let _ = std::fs::remove_file(dest);
        }
        result
    }

    fn set_open(&self, path: &Path, conn: Connection) {
        let mut session = self.session.lock().unwrap();
        session.path = Some(path.to_path_buf());
        session.conn = Some(conn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::migrations;
    use std::sync::Mutex as StdMutex;

    /// Store en memoria: ejercita los casos de uso sin SQLCipher ni disco.
    /// Guarda el par (ruta, contraseña) para simular verificación de clave.
    struct FakeStore {
        created: StdMutex<Vec<(PathBuf, String)>>,
    }

    impl FakeStore {
        fn new() -> Self {
            Self {
                created: StdMutex::new(Vec::new()),
            }
        }
        fn migrated_conn() -> Connection {
            let conn = Connection::open_in_memory().unwrap();
            migrations::run(&conn).unwrap();
            conn
        }
    }

    impl VaultStore for FakeStore {
        fn create(&self, path: &Path, passphrase: &str) -> AppResult<Connection> {
            self.created
                .lock()
                .unwrap()
                .push((path.to_path_buf(), passphrase.to_string()));
            Ok(Self::migrated_conn())
        }

        fn open(&self, path: &Path, passphrase: &str) -> AppResult<Connection> {
            let created = self.created.lock().unwrap();
            match created.iter().find(|(p, _)| p == path) {
                None => Err(AppError::NotFound),
                Some((_, pass)) if pass == passphrase => Ok(Self::migrated_conn()),
                Some(_) => Err(AppError::WrongPassword),
            }
        }
    }

    #[test]
    fn full_lifecycle_create_lock_unlock() {
        let service = VaultService::new(FakeStore::new());
        let path = PathBuf::from("/tmp/demo.karto");

        assert_eq!(service.status().status, VaultStatus::NoVault);

        let info = service.create(&path, "s3cret!!").unwrap();
        assert_eq!(info.status, VaultStatus::Unlocked);
        assert_eq!(info.path.as_deref(), Some("/tmp/demo.karto"));

        assert_eq!(service.lock().status, VaultStatus::Locked);

        assert_eq!(service.unlock(&path, "s3cret!!").unwrap().status, VaultStatus::Unlocked);
    }

    #[test]
    fn close_forgets_path_and_returns_to_no_vault() {
        let service = VaultService::new(FakeStore::new());
        let path = PathBuf::from("/tmp/demo.karto");
        service.create(&path, "s3cret!!").unwrap();

        // Cerrar (no bloquear) olvida también el path → NoVault, no Locked.
        assert_eq!(service.close().status, VaultStatus::NoVault);
        assert_eq!(service.status().path, None);
    }

    #[test]
    fn unlock_with_wrong_password_fails_and_stays_locked() {
        let service = VaultService::new(FakeStore::new());
        let path = PathBuf::from("/tmp/demo.karto");
        service.create(&path, "right").unwrap();
        service.lock();

        assert!(matches!(
            service.unlock(&path, "wrong").unwrap_err(),
            AppError::WrongPassword
        ));
        assert_eq!(service.status().status, VaultStatus::Locked);
    }

    #[test]
    fn with_conn_fails_when_locked() {
        let service = VaultService::new(FakeStore::new());
        let err = service.with_conn(|_| Ok(())).unwrap_err();
        assert!(matches!(err, AppError::NoVaultOpen));
    }

    #[test]
    fn rekey_rejects_wrong_current_password() {
        let service = VaultService::new(FakeStore::new());
        let path = PathBuf::from("/tmp/demo.karto");
        service.create(&path, "old-pass").unwrap();

        // Contraseña actual incorrecta → aborta sin re-cifrar.
        assert!(matches!(
            service.rekey("wrong", "new-pass").unwrap_err(),
            AppError::WrongPassword
        ));
    }

    /// El `PRAGMA rekey` real requiere un vault cifrado de verdad, así que este
    /// caso usa el store SQLCipher sobre un archivo temporal: re-cifra y confirma
    /// que solo la contraseña nueva reabre.
    #[test]
    fn rekey_reencrypts_with_real_sqlcipher() {
        use crate::infra::SqlcipherStore;

        let dir = std::env::temp_dir().join(format!("karto-rekey-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("v.karto");
        let _ = std::fs::remove_file(&path);

        let service = VaultService::new(SqlcipherStore::new());
        service.create(&path, "old-pass").unwrap();

        service.rekey("old-pass", "new-pass").unwrap();
        service.lock();

        // La contraseña vieja ya no sirve; la nueva sí.
        assert!(matches!(
            service.unlock(&path, "old-pass").unwrap_err(),
            AppError::WrongPassword
        ));
        assert_eq!(
            service.unlock(&path, "new-pass").unwrap().status,
            VaultStatus::Unlocked
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rekey_fails_when_no_vault_open() {
        let service = VaultService::new(FakeStore::new());
        assert!(matches!(
            service.rekey("a", "b").unwrap_err(),
            AppError::NoVaultOpen
        ));
    }

    #[test]
    fn export_writes_file_and_refuses_existing_dest() {
        let service = VaultService::new(FakeStore::new());
        service.create(&PathBuf::from("/tmp/demo.karto"), "p").unwrap();

        let dir = std::env::temp_dir().join(format!("karto-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("backup.karto");
        let _ = std::fs::remove_file(&dest);

        service.export(&dest).unwrap();
        assert!(dest.exists());

        // Con el destino ya existente, se niega.
        assert!(matches!(
            service.export(&dest).unwrap_err(),
            AppError::Other(_)
        ));

        let _ = std::fs::remove_file(&dest);
    }

    #[test]
    fn with_conn_runs_query_when_unlocked() {
        let service = VaultService::new(FakeStore::new());
        service.create(&PathBuf::from("/tmp/demo.karto"), "p").unwrap();

        let tables: i64 = service
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table'",
                    [],
                    |r| r.get(0),
                )
                .map_err(Into::into)
            })
            .unwrap();
        assert!(tables >= 7); // el esquema inicial crea 8 tablas
    }
}
