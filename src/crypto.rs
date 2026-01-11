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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        let address = keypair.address();
        
        // Address should not be all zeros
        assert_ne!(address, [0u8; 32]);
    }

    #[test]
    fn test_keypair_unique_addresses() {
        let keypair1 = KeyPair::generate();
        let keypair2 = KeyPair::generate();
        
        let address1 = keypair1.address();
        let address2 = keypair2.address();
        
        // Different keypairs should have different addresses
        assert_ne!(address1, address2);
    }

    #[test]
    fn test_keypair_sign_and_verify() {
        let keypair = KeyPair::generate();
        let message = b"Hello, HAZE!";
        
        // Sign message
        let signature = keypair.sign(message);
        assert_eq!(signature.len(), 64); // ED25519 signature is 64 bytes
        
        // Get public key
        let verifying_key = keypair.verifying_key();
        let public_key = verifying_key.to_bytes();
        
        // Verify signature
        let is_valid = verify_signature(&public_key, message, &signature).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_keypair_sign_and_verify_wrong_message() {
        let keypair = KeyPair::generate();
        let message = b"Hello, HAZE!";
        let wrong_message = b"Wrong message";
        
        // Sign message
        let signature = keypair.sign(message);
        
        // Get public key
        let verifying_key = keypair.verifying_key();
        let public_key = verifying_key.to_bytes();
        
        // Verify with wrong message
        let is_valid = verify_signature(&public_key, wrong_message, &signature).unwrap();
        assert!(!is_valid);
    }

    #[test]
    fn test_verify_signature_invalid_signature() {
        // Create a valid keypair first
        let keypair = KeyPair::generate();
        let verifying_key = keypair.verifying_key();
        let valid_public_key = verifying_key.to_bytes();
        
        // Use wrong signature (all zeros)
        let message = b"test";
        let invalid_signature = [0u8; 64];
        
        // Should return Ok(false) for invalid signature
        let result = verify_signature(&valid_public_key, message, &invalid_signature).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_address_from_public_key() {
        let keypair = KeyPair::generate();
        let verifying_key = keypair.verifying_key();
        let public_key = verifying_key.to_bytes();
        
        let address1 = keypair.address();
        let address2 = address_from_public_key(&public_key);
        
        // Address should be consistent
        assert_eq!(address1, address2);
    }
}