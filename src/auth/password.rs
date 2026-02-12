// Password hashing with Argon2id
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params, Version,
};
use crate::errors::{AppError, Result};

/// Hash a password using Argon2id with OWASP recommended parameters
///
/// Parameters (OWASP 2023):
/// - Memory: 19 MiB (19456 KiB)
/// - Iterations: 2
/// - Parallelism: 1
/// - Output length: 32 bytes
pub fn hash_password(password: &str) -> Result<String> {
    // Validate password length
    if password.is_empty() {
        return Err(AppError::ValidationError("Password cannot be empty".to_string()));
    }

    if password.len() < 8 {
        return Err(AppError::ValidationError("Password must be at least 8 characters".to_string()));
    }

    // OWASP recommended parameters for Argon2id
    let params = Params::new(
        19456,  // m_cost (memory): 19 MiB
        2,      // t_cost (iterations)
        1,      // p_cost (parallelism)
        Some(32) // output length
    ).map_err(|e| AppError::Cryptographic(format!("Failed to create Argon2 params: {}", e)))?;

    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        Version::V0x13,
        params,
    );

    let salt = SaltString::generate(&mut OsRng);

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Cryptographic(format!("Failed to hash password: {}", e)))?
        .to_string();

    tracing::debug!("Password hashed successfully");

    Ok(password_hash)
}

/// Verify a password against a hash using constant-time comparison
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Cryptographic(format!("Failed to parse password hash: {}", e)))?;

    // Use Argon2 to verify the password
    let argon2 = Argon2::default();

    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => {
            tracing::debug!("Password verified successfully");
            Ok(true)
        }
        Err(argon2::password_hash::Error::Password) => {
            tracing::debug!("Password verification failed");
            Ok(false)
        }
        Err(e) => {
            tracing::error!("Password verification error: {}", e);
            Err(AppError::Cryptographic(format!("Password verification error: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        // Hash should be a valid PHC string
        assert!(hash.starts_with("$argon2id$"));

        // Hash should be different each time (due to random salt)
        let hash2 = hash_password(password).unwrap();
        assert_ne!(hash, hash2);
    }

    #[test]
    fn test_verify_password_success() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
    }

    #[test]
    fn test_verify_password_failure() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_empty_password() {
        let result = hash_password("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ValidationError(_)));
    }

    #[test]
    fn test_short_password() {
        let result = hash_password("short");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ValidationError(_)));
    }
}
