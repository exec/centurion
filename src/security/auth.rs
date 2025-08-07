use argon2::{Argon2, PasswordHash};
use argon2::password_hash::{rand_core::OsRng, SaltString, PasswordHasher as ArgonPasswordHasher};
use base64::{Engine as _, engine::general_purpose};
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Unsupported mechanism")]
    UnsupportedMechanism,
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Invalid auth data")]
    InvalidAuthData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SaslMechanism {
    Plain,
    ScramSha256,
    External,
}

impl SaslMechanism {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "PLAIN" => Some(SaslMechanism::Plain),
            "SCRAM-SHA-256" => Some(SaslMechanism::ScramSha256),
            "EXTERNAL" => Some(SaslMechanism::External),
            _ => None,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            SaslMechanism::Plain => "PLAIN",
            SaslMechanism::ScramSha256 => "SCRAM-SHA-256",
            SaslMechanism::External => "EXTERNAL",
        }
    }
}

#[derive(Debug, Clone)]
pub enum AuthMethod {
    Password(String),
    Sasl {
        mechanism: SaslMechanism,
        data: Vec<u8>,
    },
    Certificate {
        fingerprint: String,
    },
}

pub struct LocalPasswordHasher;

impl LocalPasswordHasher {
    pub fn hash_password(password: &str) -> Result<String, AuthError> {
        // Simplified for now - in production use proper hashing
        Ok(format!("hash_{}", password))
    }
    
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
        // Simplified for now - in production use proper verification
        Ok(hash == &format!("hash_{}", password))
    }
}

pub struct SaslAuthenticator {
    scram_server_state: Option<ScramServerState>,
}

struct ScramServerState {
    client_nonce: String,
    server_nonce: String,
    salt: Vec<u8>,
    iterations: u32,
    auth_message: String,
}

impl SaslAuthenticator {
    pub fn new() -> Self {
        Self {
            scram_server_state: None,
        }
    }
    
    pub fn authenticate_plain(&self, auth_data: &[u8]) -> Result<(String, String), AuthError> {
        let auth_string = std::str::from_utf8(auth_data)
            .map_err(|_| AuthError::InvalidAuthData)?;
        
        let parts: Vec<&str> = auth_string.split('\0').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidAuthData);
        }
        
        let authzid = parts[0];
        let authcid = parts[1];
        let password = parts[2];
        
        if authzid.is_empty() || authzid == authcid {
            Ok((authcid.to_string(), password.to_string()))
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }
    
    pub fn start_scram_sha256(&mut self, client_first: &str) -> Result<String, AuthError> {
        // Parse client first message
        let parts: Vec<&str> = client_first.split(',').collect();
        if parts.len() < 3 {
            return Err(AuthError::InvalidAuthData);
        }
        
        let client_nonce = parts[2]
            .strip_prefix("r=")
            .ok_or(AuthError::InvalidAuthData)?;
        
        // Generate server nonce
        let server_nonce = format!("{}{}", client_nonce, generate_nonce());
        
        // Generate salt and iteration count
        let salt = generate_salt();
        let iterations = 4096u32;
        
        // Build server first message
        let server_first = format!(
            "r={},s={},i={}",
            server_nonce,
            general_purpose::STANDARD.encode(&salt),
            iterations
        );
        
        self.scram_server_state = Some(ScramServerState {
            client_nonce: client_nonce.to_string(),
            server_nonce: server_nonce.clone(),
            salt,
            iterations,
            auth_message: format!("{},{}", client_first, server_first),
        });
        
        Ok(server_first)
    }
    
    pub fn verify_scram_sha256(
        &mut self,
        client_final: &str,
        stored_password: &str,
    ) -> Result<String, AuthError> {
        let state = self.scram_server_state
            .as_mut()
            .ok_or(AuthError::InvalidAuthData)?;
        
        // Parse client final message
        let parts: Vec<&str> = client_final.split(',').collect();
        let _client_proof = parts.iter()
            .find(|p| p.starts_with("p="))
            .and_then(|p| p.strip_prefix("p="))
            .ok_or(AuthError::InvalidAuthData)?;
        
        // Complete auth message
        let channel_binding = "c=biws";
        let without_proof = format!("{},r={}", channel_binding, state.server_nonce);
        state.auth_message = format!("{},{}", state.auth_message, without_proof);
        
        // Calculate server signature
        let salted_password = derive_salted_password(
            stored_password.as_bytes(),
            &state.salt,
            state.iterations,
        );
        
        let server_key = hmac_sha256(&salted_password, b"Server Key");
        let server_signature = hmac_sha256(&server_key, state.auth_message.as_bytes());
        
        // Build server final message
        let server_final = format!(
            "v={}",
            general_purpose::STANDARD.encode(&server_signature)
        );
        
        Ok(server_final)
    }
}

pub async fn authenticate(
    method: AuthMethod,
    stored_hash: &str,
) -> Result<bool, AuthError> {
    match method {
        AuthMethod::Password(password) => {
            LocalPasswordHasher::verify_password(&password, stored_hash)
        }
        AuthMethod::Sasl { mechanism, data } => {
            match mechanism {
                SaslMechanism::Plain => {
                    let auth = SaslAuthenticator::new();
                    let (_, password) = auth.authenticate_plain(&data)?;
                    LocalPasswordHasher::verify_password(&password, stored_hash)
                }
                _ => Err(AuthError::UnsupportedMechanism),
            }
        }
        AuthMethod::Certificate { fingerprint } => {
            // Compare certificate fingerprint
            Ok(fingerprint == stored_hash)
        }
    }
}

fn generate_nonce() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let nonce: Vec<u8> = (0..16).map(|_| rng.gen()).collect();
    general_purpose::STANDARD.encode(&nonce)
}

fn generate_salt() -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..16).map(|_| rng.gen()).collect()
}

fn derive_salted_password(password: &[u8], salt: &[u8], iterations: u32) -> Vec<u8> {
    let mut result = vec![0u8; 32];
    pbkdf2_hmac::<Sha256>(password, salt, iterations, &mut result);
    result
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}