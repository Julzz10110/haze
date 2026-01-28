//! Cryptographic utilities for HAZE

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::{Zeroize, Zeroizing};
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
        Zeroize::zeroize(&mut secret_bytes[..]);
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
    ).map_err(|_| HazeError::Crypto("Invalid public key bytes".to_string()))?;

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

/// Export signing key as raw 32-byte secret key.
///
/// The returned value is zeroized on drop. Use `.as_ref()` or `Deref` to pass
/// to other APIs; avoid copying the bytes into long-lived storage.
pub fn signing_key_to_bytes(signing_key: &SigningKey) -> Zeroizing<[u8; 32]> {
    Zeroizing::new(signing_key.to_bytes())
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
        .map_err(|_| HazeError::Crypto("Invalid public key bytes".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

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
        let imported = signing_key_from_bytes(exported.as_ref()).unwrap();

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

    // --- Property-style: sign then verify for random keypairs and messages ---

    #[test]
    fn test_prop_sign_verify_random_messages() {
        let mut rng = rand::rngs::OsRng;
        for _ in 0..50 {
            let keypair = KeyPair::generate();
            let public_key = keypair.verifying_key().to_bytes();
            let len = (rng.next_u32() % 1024) as usize;
            let mut message = vec![0u8; len];
            rng.fill_bytes(&mut message);
            let signature = keypair.sign(&message);
            let ok = verify_signature(&public_key, &message, &signature).unwrap();
            assert!(ok, "verify_signature(pk, message, sign(message)) must be true");
        }
    }

    #[test]
    fn test_prop_sign_verify_empty_message() {
        let keypair = KeyPair::generate();
        let public_key = keypair.verifying_key().to_bytes();
        let message: &[u8] = &[];
        let signature = keypair.sign(message);
        let ok = verify_signature(&public_key, message, &signature).unwrap();
        assert!(ok);
    }

    #[test]
    fn test_prop_sign_verify_large_message() {
        let keypair = KeyPair::generate();
        let public_key = keypair.verifying_key().to_bytes();
        let message = vec![0x42u8; 1_000_000];
        let signature = keypair.sign(&message);
        let ok = verify_signature(&public_key, &message, &signature).unwrap();
        assert!(ok);
    }

    // --- Negative: verify_signature with bad lengths / garbage ---

    #[test]
    fn test_verify_signature_err_wrong_public_key_length() {
        let keypair = KeyPair::generate();
        let sig = keypair.sign(b"x");
        let pk = keypair.verifying_key().to_bytes();

        assert!(verify_signature(&[], b"x", &sig).is_err());
        assert!(verify_signature(&pk[..1], b"x", &sig).is_err());
        assert!(verify_signature(&pk[..31], b"x", &sig).is_err());
        let pk33: Vec<u8> = pk.iter().copied().chain(std::iter::once(0)).collect();
        assert!(verify_signature(&pk33, b"x", &sig).is_err());
    }

    #[test]
    fn test_verify_signature_err_wrong_signature_length() {
        let keypair = KeyPair::generate();
        let pk = keypair.verifying_key().to_bytes();
        let sig = keypair.sign(b"x");

        assert!(verify_signature(&pk, b"x", &[]).is_err());
        assert!(verify_signature(&pk, b"x", &sig[..63]).is_err());
        let sig65: Vec<u8> = sig.iter().copied().chain(std::iter::once(0)).collect();
        assert!(verify_signature(&pk, b"x", &sig65).is_err());
    }

    #[test]
    fn test_verify_signature_err_invalid_public_key_bytes() {
        use crate::error::HazeError;
        // Random 32 bytes: from_bytes may reject (Err) or accept and verify fails (Ok(false)).
        // Must never return Ok(true).
        let mut bad_pk = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bad_pk);
        let sig = [0u8; 64];
        let r = verify_signature(&bad_pk, b"msg", &sig);
        match r {
            Ok(valid) => assert!(!valid, "invalid public key must not verify as true"),
            Err(e) => assert!(matches!(e, HazeError::Crypto(_))),
        }
    }

    #[test]
    fn test_verify_signature_ok_false_garbage_signature() {
        let keypair = KeyPair::generate();
        let pk = keypair.verifying_key().to_bytes();
        let mut garbage_sig = [0u8; 64];
        rand::rngs::OsRng.fill_bytes(&mut garbage_sig);
        let r = verify_signature(&pk, b"message", &garbage_sig).unwrap();
        assert!(!r);
    }

    #[test]
    fn test_verify_signature_ok_false_wrong_public_key() {
        let k1 = KeyPair::generate();
        let k2 = KeyPair::generate();
        let msg = b"shared message";
        let sig = k1.sign(msg);
        let pk2 = k2.verifying_key().to_bytes();
        let r = verify_signature(&pk2, msg, &sig).unwrap();
        assert!(!r);
    }
}