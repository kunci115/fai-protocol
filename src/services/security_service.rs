//! Security and authentication service for FAI Protocol
//!
//! Provides basic security functionality including:
//! - User authentication
//! - File encryption/decryption
//! - Access control
//! - Key management

use anyhow::Result;
use std::path::Path;
use serde::{Deserialize, Serialize};

/// Security service for FAI Protocol
pub struct SecurityService {
    config_path: std::path::PathBuf,
}

impl SecurityService {
    /// Create a new security service instance
    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {
        Self {
            config_path: config_path.as_ref().to_path_buf(),
        }
    }

    /// Initialize security service
    pub fn initialize(&self) -> Result<()> {
        // Create .fai/security directory if it doesn't exist
        let security_dir = self.config_path.join("security");
        std::fs::create_dir_all(&security_dir)?;

        // Create default security config if it doesn't exist
        let config_file = security_dir.join("config.toml");
        if !config_file.exists() {
            let default_config = SecurityConfig::default();
            self.save_config(&default_config)?;
        }

        Ok(())
    }

    /// Generate a new user key pair (simplified version)
    pub fn generate_user_keypair(&self, username: &str) -> Result<UserKeyPair> {
        use rand::Rng;
        use rand::thread_rng;

        let mut rng = thread_rng();
        let mut public_key = [0u8; 32];
        let mut private_key = [0u8; 32];
        rng.fill(&mut private_key);
        rng.fill(&mut public_key);

        // In a real implementation, these would be a proper key pair
        // For now, we'll use random bytes as placeholders

        let keypair = UserKeyPair {
            username: username.to_string(),
            public_key: public_key.to_vec(),
            private_key: private_key.to_vec(),
            created_at: chrono::Utc::now(),
        };

        // Save keypair to file
        self.save_user_keypair(&keypair)?;

        Ok(keypair)
    }

    /// Authenticate a user (simplified version)
    pub fn authenticate_user(&self, username: &str, signature: &[u8], message: &[u8]) -> Result<bool> {
        let keypair = self.load_user_keypair(username)?;

        // Simplified authentication - in a real implementation, this would use proper cryptographic verification
        // For now, we'll use a simple hash comparison
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(message);
        hasher.update(&keypair.private_key);
        let expected_signature = hasher.finalize();

        Ok(signature == expected_signature.as_bytes())
    }

    /// Encrypt data with user's public key
    pub fn encrypt_for_user(&self, username: &str, data: &[u8]) -> Result<Vec<u8>> {
        // For now, use a simple XOR encryption with user-specific key
        // In a real implementation, you'd use proper asymmetric encryption
        let keypair = self.load_user_keypair(username)?;
        let key = self.derive_encryption_key(&keypair.public_key)?;

        Ok(self.simple_encrypt(data, &key))
    }

    /// Decrypt data with user's private key
    pub fn decrypt_for_user(&self, username: &str, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        let keypair = self.load_user_keypair(username)?;
        let key = self.derive_encryption_key(&keypair.public_key)?;

        Ok(self.simple_decrypt(encrypted_data, &key))
    }

    /// Check if user has permission for an operation
    pub fn check_permission(&self, username: &str, operation: &str, _resource: &str) -> Result<bool> {
        let config = self.load_config()?;

        // Check user permissions
        if let Some(user_perms) = config.users.get(username) {
            return Ok(user_perms.permissions.contains(&operation.to_string()));
        }

        // Check role permissions
        if let Some(user_config) = config.users.get(username) {
            if let Some(role_perms) = config.roles.get(&user_config.role) {
                return Ok(role_perms.permissions.contains(&operation.to_string()));
            }
        }

        Ok(false)
    }

    /// Create a new user with permissions
    pub fn create_user(&self, username: &str, role: &str, permissions: Vec<String>) -> Result<()> {
        let mut config = self.load_config()?;

        let user_config = UserConfig {
            role: role.to_string(),
            permissions,
            created_at: chrono::Utc::now(),
            last_login: None,
        };

        config.users.insert(username.to_string(), user_config);
        self.save_config(&config)?;

        Ok(())
    }

    // Private helper methods

    fn save_config(&self, config: &SecurityConfig) -> Result<()> {
        let config_file = self.config_path.join("security/config.toml");
        let config_str = toml::to_string_pretty(config)?;
        std::fs::write(config_file, config_str)?;
        Ok(())
    }

    fn load_config(&self) -> Result<SecurityConfig> {
        let config_file = self.config_path.join("security/config.toml");
        let config_str = std::fs::read_to_string(config_file)?;
        let config: SecurityConfig = toml::from_str(&config_str)?;
        Ok(config)
    }

    fn save_user_keypair(&self, keypair: &UserKeyPair) -> Result<()> {
        let key_file = self.config_path.join(format!("security/users/{}.json", keypair.username));
        let key_str = serde_json::to_string_pretty(keypair)?;
        std::fs::write(key_file, key_str)?;
        Ok(())
    }

    fn load_user_keypair(&self, username: &str) -> Result<UserKeyPair> {
        let key_file = self.config_path.join(format!("security/users/{}.json", username));
        let key_str = std::fs::read_to_string(key_file)?;
        let keypair: UserKeyPair = serde_json::from_str(&key_str)?;
        Ok(keypair)
    }

    fn derive_encryption_key(&self, public_key: &[u8]) -> Result<[u8; 32]> {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(public_key);
        hasher.update(b"fai-encryption-key");
        let hash = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash.as_bytes()[..32]);
        Ok(key)
    }

    fn simple_encrypt(&self, data: &[u8], key: &[u8; 32]) -> Vec<u8> {
        let mut encrypted = Vec::with_capacity(data.len());
        for (i, &byte) in data.iter().enumerate() {
            encrypted.push(byte ^ key[i % key.len()]);
        }
        encrypted
    }

    fn simple_decrypt(&self, encrypted_data: &[u8], key: &[u8; 32]) -> Vec<u8> {
        // XOR encryption is symmetric, so decryption is the same as encryption
        self.simple_encrypt(encrypted_data, key)
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub users: std::collections::HashMap<String, UserConfig>,
    pub roles: std::collections::HashMap<String, RoleConfig>,
    pub settings: SecuritySettings,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        let mut roles = std::collections::HashMap::new();

        roles.insert("admin".to_string(), RoleConfig {
            permissions: vec![
                "read".to_string(),
                "write".to_string(),
                "delete".to_string(),
                "admin".to_string(),
            ],
        });

        roles.insert("user".to_string(), RoleConfig {
            permissions: vec![
                "read".to_string(),
                "write".to_string(),
            ],
        });

        Self {
            users: std::collections::HashMap::new(),
            roles,
            settings: SecuritySettings::default(),
        }
    }
}

/// User configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub role: String,
    pub permissions: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
}

/// Role configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    pub permissions: Vec<String>,
}

/// Security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub encryption_enabled: bool,
    pub require_auth: bool,
    pub session_timeout_minutes: u64,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            encryption_enabled: true,
            require_auth: false, // Disabled by default for ease of use
            session_timeout_minutes: 60,
        }
    }
}

/// User key pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserKeyPair {
    pub username: String,
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_security_service_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let security_service = SecurityService::new(temp_dir.path());

        assert!(security_service.initialize().is_ok());

        let config_file = temp_dir.path().join("security/config.toml");
        assert!(config_file.exists());
    }
}