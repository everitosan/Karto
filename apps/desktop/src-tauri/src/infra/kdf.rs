//! Derivación de clave del vault con **Argon2id**.
//!
//! La contraseña maestra nunca se pasa a la KDF nativa de SQLCipher (PBKDF2).
//! En su lugar derivamos aquí una clave cruda de 32 bytes y se la entregamos a
//! SQLCipher en modo *raw key* (`PRAGMA key = "x'…'"`), evitando su PBKDF2.
//!
//! El **salt** (16 bytes) se reutiliza del salt de cabecera de SQLCipher: vive
//! en claro en los primeros 16 bytes del archivo `.karto`, así que es legible
//! antes de descifrar y el vault sigue siendo un único archivo portable. En
//! `create` lo fijamos con `PRAGMA cipher_salt`; en `open` lo leemos del header.
//!
//! Los parámetros de Argon2 son constantes de compilación (no hay vaults en
//! producción que migrar). Si en el futuro se suben, habrá que versionarlos.

use crate::error::{AppError, AppResult};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use zeroize::Zeroizing;

/// Longitud del salt de cabecera de SQLCipher (y del salt de Argon2), en bytes.
pub const SALT_LEN: usize = 16;
/// Longitud de la clave cruda que espera SQLCipher (AES-256), en bytes.
pub const KEY_LEN: usize = 32;

/// Coste de memoria de Argon2id (64 MiB). Holgado para un escritorio; muy caro
/// de paralelizar en GPU/ASIC frente a PBKDF2.
const MEM_KIB: u32 = 64 * 1024;
/// Iteraciones (time cost).
const TIME_COST: u32 = 3;
/// Grado de paralelismo (lanes).
const PARALLELISM: u32 = 1;

/// Genera un salt aleatorio de 16 bytes para un vault nuevo.
pub fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// Deriva la clave cruda de 32 bytes desde la contraseña y el salt. El resultado
/// se envuelve en `Zeroizing` para limpiarlo de memoria al soltarlo.
pub fn derive_key(password: &str, salt: &[u8; SALT_LEN]) -> AppResult<Zeroizing<[u8; KEY_LEN]>> {
    let params = Params::new(MEM_KIB, TIME_COST, PARALLELISM, Some(KEY_LEN))
        .map_err(|e| AppError::Other(format!("params Argon2 inválidos: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|e| AppError::Other(format!("derivación Argon2 falló: {e}")))?;
    Ok(key)
}

/// Formatea la clave cruda como literal SQLCipher `x'…'` (64 hex).
pub fn key_pragma(key: &[u8; KEY_LEN]) -> Zeroizing<String> {
    Zeroizing::new(format!("x'{}'", hex::encode(key)))
}

/// Formatea el salt como literal SQLCipher `x'…'` (32 hex) para `cipher_salt`.
pub fn salt_pragma(salt: &[u8; SALT_LEN]) -> String {
    format!("x'{}'", hex::encode(salt))
}

/// Lee el salt de cabecera (primeros 16 bytes en claro) de un `.karto` existente.
pub fn read_header_salt(path: &std::path::Path) -> AppResult<[u8; SALT_LEN]> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut salt = [0u8; SALT_LEN];
    file.read_exact(&mut salt)
        .map_err(|_| AppError::Other("el archivo no parece un vault válido".into()))?;
    Ok(salt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_is_deterministic_and_salt_sensitive() {
        let salt_a = [7u8; SALT_LEN];
        let salt_b = [9u8; SALT_LEN];
        let k1 = derive_key("hunter2", &salt_a).unwrap();
        let k2 = derive_key("hunter2", &salt_a).unwrap();
        let k3 = derive_key("hunter2", &salt_b).unwrap();
        assert_eq!(*k1, *k2, "misma contraseña + salt → misma clave");
        assert_ne!(*k1, *k3, "distinto salt → distinta clave");
    }

    #[test]
    fn wrong_password_derives_different_key() {
        let salt = [3u8; SALT_LEN];
        assert_ne!(
            *derive_key("right", &salt).unwrap(),
            *derive_key("wrong", &salt).unwrap()
        );
    }

    #[test]
    fn pragma_formatting_is_hex_literal() {
        let key = [0xABu8; KEY_LEN];
        let p = key_pragma(&key);
        assert!(p.starts_with("x'") && p.ends_with('\''));
        assert_eq!(p.len(), 2 + KEY_LEN * 2 + 1);
        assert_eq!(salt_pragma(&[0x01u8; SALT_LEN]).len(), 2 + SALT_LEN * 2 + 1);
    }

    #[test]
    fn random_salt_is_not_constant() {
        assert_ne!(random_salt(), random_salt());
    }
}
