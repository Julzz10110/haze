using System;
using System.Security.Cryptography;
using System.Text;
using Chaos.NaCl;

namespace Haze.Crypto
{
    /// <summary>
    /// Ed25519 key pair for signing transactions
    /// Uses Chaos.NaCl library (compatible with Rust ed25519-dalek)
    /// </summary>
    public class KeyPair
    {
        private readonly byte[] _privateKey;
        private readonly byte[] _publicKey;

        private KeyPair(byte[] privateKey, byte[] publicKey)
        {
            if (privateKey == null || privateKey.Length != 32)
                throw new ArgumentException("Private key must be 32 bytes", nameof(privateKey));
            if (publicKey == null || publicKey.Length != 32)
                throw new ArgumentException("Public key must be 32 bytes", nameof(publicKey));

            _privateKey = privateKey;
            _publicKey = publicKey;
        }

        /// <summary>
        /// Generate a new key pair
        /// </summary>
        public static KeyPair Generate()
        {
            var privateKey = new byte[32];
            using (var rng = RandomNumberGenerator.Create())
            {
                rng.GetBytes(privateKey);
            }
            var publicKey = Ed25519.PublicKeyFromSeed(privateKey);
            return new KeyPair(privateKey, publicKey);
        }

        /// <summary>
        /// Create key pair from private key (32 bytes)
        /// </summary>
        public static KeyPair FromPrivateKey(byte[] privateKey)
        {
            if (privateKey == null || privateKey.Length != 32)
                throw new ArgumentException("Private key must be 32 bytes", nameof(privateKey));

            var publicKey = Ed25519.PublicKeyFromSeed(privateKey);
            return new KeyPair(privateKey, publicKey);
        }

        /// <summary>
        /// Create key pair from private key (hex string)
        /// </summary>
        public static KeyPair FromPrivateKeyHex(string privateKeyHex)
        {
            if (string.IsNullOrEmpty(privateKeyHex) || privateKeyHex.Length != 64)
                throw new ArgumentException("Private key hex must be 64 characters", nameof(privateKeyHex));

            var privateKey = HexToBytes(privateKeyHex);
            return FromPrivateKey(privateKey);
        }

        /// <summary>
        /// Get address (32-byte Ed25519 public key)
        /// </summary>
        public Address GetAddress()
        {
            return new Address(_publicKey);
        }

        /// <summary>
        /// Get address as hex string
        /// </summary>
        public string GetAddressHex()
        {
            return BytesToHex(_publicKey);
        }

        /// <summary>
        /// Get public key as hex string
        /// </summary>
        public string GetPublicKeyHex()
        {
            return BytesToHex(_publicKey);
        }

        /// <summary>
        /// Get private key as hex string (use with caution!)
        /// </summary>
        public string GetPrivateKeyHex()
        {
            return BytesToHex(_privateKey);
        }

        /// <summary>
        /// Sign a message
        /// </summary>
        public byte[] Sign(byte[] message)
        {
            if (message == null)
                throw new ArgumentNullException(nameof(message));

            return Ed25519.Sign(message, _privateKey);
        }

        /// <summary>
        /// Verify a signature
        /// </summary>
        public static bool Verify(byte[] message, byte[] signature, byte[] publicKey)
        {
            if (message == null || signature == null || publicKey == null)
                return false;
            if (signature.Length != 64 || publicKey.Length != 32)
                return false;

            try
            {
                return Ed25519.Verify(signature, message, publicKey);
            }
            catch
            {
                return false;
            }
        }

        private static string BytesToHex(byte[] bytes)
        {
            var sb = new StringBuilder(bytes.Length * 2);
            foreach (var b in bytes)
                sb.Append(b.ToString("x2"));
            return sb.ToString();
        }

        private static byte[] HexToBytes(string hex)
        {
            if (hex.Length % 2 != 0)
                throw new ArgumentException("Hex string must have even length", nameof(hex));

            var bytes = new byte[hex.Length / 2];
            for (int i = 0; i < bytes.Length; i++)
                bytes[i] = Convert.ToByte(hex.Substring(i * 2, 2), 16);
            return bytes;
        }
    }
}
