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

    /// Punto único de acceso a la conexión descifrada para el resto de casos de
    /// uso (repos de nodos, credenciales, etc.); falla si el vault está bloqueado.
    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let session = self.session.lock().unwrap();
        let conn = session.conn.as_ref().ok_or(AppError::NoVaultOpen)?;
        f(conn)
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
