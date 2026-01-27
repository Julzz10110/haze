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
    ///
    /// # Example
    /// ```
    /// use haze::crypto::KeyPair;
    ///
    /// let keypair = KeyPair::generate();
    /// let address = keypair.address();
    /// ```
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        Self { signing_key }
    }

    /// Get public key as address
    ///
    /// Returns a 32-byte address derived from the public key.
    ///
    /// # Example
    /// ```
    /// use haze::crypto::KeyPair;
    ///
    /// let keypair = KeyPair::generate();
    /// let address = keypair.address();
    /// assert_eq!(address.len(), 32);
    /// ```
    pub fn address(&self) -> Address {
        let verifying_key = self.signing_key.verifying_key();
        let bytes = verifying_key.to_bytes();
        let mut address = [0u8; 32];
        address.copy_from_slice(&bytes);
        address
    }

    /// Sign data with this key pair
    ///
    /// # Arguments
    /// * `data` - The data to sign
    ///
    /// # Returns
    /// A 64-byte ED25519 signature
    ///
    /// # Example
    /// ```
    /// use haze::crypto::{KeyPair, verify_signature};
    ///
    /// let keypair = KeyPair::generate();
    /// let message = b"Hello, HAZE!";
    /// let signature = keypair.sign(message);
    /// let public_key = keypair.verifying_key().to_bytes();
    ///
    /// assert!(verify_signature(&public_key, message, &signature).unwrap());
    /// ```
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        self.signing_key.sign(data).to_bytes().to_vec()
    }

    /// Get the verifying key (public key)
    ///
    /// # Example
    /// ```
    /// use haze::crypto::KeyPair;
    ///
    /// let keypair = KeyPair::generate();
    /// let verifying_key = keypair.verifying_key();
    /// let public_key_bytes = verifying_key.to_bytes();
    /// assert_eq!(public_key_bytes.len(), 32);
    /// ```
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }
}

/// Verify a signature
///
/// # Arguments
/// * `public_key` - The public key (32 bytes)
/// * `message` - The original message that was signed
/// * `signature` - The signature to verify (64 bytes)
///
/// # Returns
/// `Ok(true)` if the signature is valid, `Ok(false)` if invalid, or an error if the inputs are malformed.
///
/// # Example
/// ```
/// use haze::crypto::{KeyPair, verify_signature};
///
/// let keypair = KeyPair::generate();
/// let message = b"Hello, HAZE!";
/// let signature = keypair.sign(message);
/// let public_key = keypair.verifying_key().to_bytes();
///
/// assert!(verify_signature(&public_key, message, &signature).unwrap());
/// assert!(!verify_signature(&public_key, b"Wrong message", &signature).unwrap());
/// ```
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

    match public_key.len() {
        32 => {
            // Standard case: ed25519 public key length
            address.copy_from_slice(public_key);
        }
        _ => {
            // For any non-standard length, derive address via SHA-256 hash
            let hash = crate::types::sha256(public_key);
            address.copy_from_slice(&hash);
        }
    }

    address
}

/// Export signing key as raw 32-byte secret key
pub fn signing_key_to_bytes(signing_key: &SigningKey) -> [u8; 32] {
    signing_key.to_bytes()
}

/// Import signing key from raw 32-byte secret key
pub fn signing_key_from_bytes(bytes: &[u8]) -> Result<SigningKey> {
    let secret: &[u8; 32] = bytes
        .try_into()
        .map_err(|_| HazeError::Crypto("Invalid signing key length".to_string()))?;
    Ok(SigningKey::from_bytes(secret))
}

/// Export verifying key (public key) as 32-byte array
pub fn verifying_key_to_bytes(verifying_key: &VerifyingKey) -> [u8; 32] {
    verifying_key.to_bytes()
}

/// Import verifying key (public key) from 32-byte array
pub fn verifying_key_from_bytes(bytes: &[u8]) -> Result<VerifyingKey> {
    let pk_bytes: &[u8; 32] = bytes
        .try_into()
        .map_err(|_| HazeError::Crypto("Invalid public key length".to_string()))?;
    VerifyingKey::from_bytes(pk_bytes)
        .map_err(|e| HazeError::Crypto(format!("Invalid public key: {}", e)))
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

    #[test]
    fn test_address_from_short_public_key_uses_hash() {
        let short_public_key = [1u8, 2, 3];

        let address = address_from_public_key(&short_public_key);

        // Address should not be all zeros and must be derived deterministically
        assert_ne!(address, [0u8; 32]);
    }

    #[test]
    fn test_address_from_long_public_key_uses_hash_not_truncation() {
        let mut long_public_key = [0u8; 64];
        for (i, b) in long_public_key.iter_mut().enumerate() {
            *b = i as u8;
        }

        let address = address_from_public_key(&long_public_key);

        let mut truncated = [0u8; 32];
        truncated.copy_from_slice(&long_public_key[..32]);

        // Address for long key should not be simple truncation
        assert_ne!(address, truncated);
    }

    #[test]
    fn test_signing_key_serialize_roundtrip() {
        let keypair = KeyPair::generate();
        let original_signing = keypair.signing_key;

        let exported = signing_key_to_bytes(&original_signing);
        let imported = signing_key_from_bytes(&exported).unwrap();

        assert_eq!(original_signing.to_bytes(), imported.to_bytes());
    }

    #[test]
    fn test_verifying_key_serialize_roundtrip() {
        let keypair = KeyPair::generate();
        let original_verifying = keypair.verifying_key();

        let exported = verifying_key_to_bytes(&original_verifying);
        let imported = verifying_key_from_bytes(&exported).unwrap();

        assert_eq!(original_verifying.to_bytes(), imported.to_bytes());
    }
}