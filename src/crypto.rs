//! Cryptographic utilities for HAZE

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use rand::RngCore;
use crate::types::Address;
use crate::error::{HazeError, Result};

/// Key pair for signing transactions
pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    /// Generate new key pair
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        Self { signing_key }
    }

    /// Get public key as address
    pub fn address(&self) -> Address {
        let verifying_key = self.signing_key.verifying_key();
        let bytes = verifying_key.to_bytes();
        let mut address = [0u8; 32];
        address.copy_from_slice(&bytes);
        address
    }

    /// Sign data
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        self.signing_key.sign(data).to_bytes().to_vec()
    }

    /// Get verifying key
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }
}

/// Verify signature
pub fn verify_signature(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<bool> {
    let verifying_key = VerifyingKey::from_bytes(
        public_key.try_into()
            .map_err(|_| HazeError::Crypto("Invalid public key length".to_string()))?
    ).map_err(|e| HazeError::Crypto(format!("Invalid public key: {}", e)))?;

    let sig_bytes: [u8; 64] = signature.try_into()
        .map_err(|_| HazeError::Crypto("Invalid signature length".to_string()))?;
    let sig = Signature::from_bytes(&sig_bytes);

    Ok(verifying_key.verify(message, &sig).is_ok())
}

/// Address from public key bytes
pub fn address_from_public_key(public_key: &[u8]) -> Address {
    let mut address = [0u8; 32];
    if public_key.len() >= 32 {
        address.copy_from_slice(&public_key[..32]);
    } else {
        let hash = crate::types::sha256(public_key);
        address.copy_from_slice(&hash);
    }
    address
}