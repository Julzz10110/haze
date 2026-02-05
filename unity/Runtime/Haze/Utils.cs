using System;
using System.Security.Cryptography;
using System.Text;

namespace Haze
{
    /// <summary>
    /// Utility functions
    /// </summary>
    public static class Utils
    {
        /// <summary>
        /// Convert bytes to hex string
        /// </summary>
        public static string BytesToHex(byte[] bytes)
        {
            if (bytes == null) return string.Empty;
            var sb = new StringBuilder(bytes.Length * 2);
            foreach (var b in bytes)
                sb.Append(b.ToString("x2"));
            return sb.ToString();
        }

        /// <summary>
        /// Convert hex string to bytes
        /// </summary>
        public static byte[] HexToBytes(string hex)
        {
            if (string.IsNullOrEmpty(hex))
                return Array.Empty<byte>();
            if (hex.Length % 2 != 0)
                throw new ArgumentException("Hex string must have even length", nameof(hex));

            var bytes = new byte[hex.Length / 2];
            for (int i = 0; i < bytes.Length; i++)
                bytes[i] = Convert.ToByte(hex.Substring(i * 2, 2), 16);
            return bytes;
        }

        /// <summary>
        /// Compute SHA-256 hash
        /// </summary>
        public static Hash Sha256(byte[] data)
        {
            using (var sha256 = SHA256.Create())
            {
                return new Hash(sha256.ComputeHash(data));
            }
        }

        /// <summary>
        /// Compute SHA-256 hash from string
        /// </summary>
        public static Hash Sha256(string data)
        {
            return Sha256(Encoding.UTF8.GetBytes(data));
        }

        /// <summary>
        /// Convert ulong to string (for bigint fields in API)
        /// </summary>
        public static string UlongToString(ulong value)
        {
            return value.ToString();
        }

        /// <summary>
        /// Convert string to ulong (for bigint fields from API)
        /// </summary>
        public static ulong StringToUlong(string value)
        {
            return ulong.Parse(value);
        }
    }
}
