using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Haze.Crypto;

namespace Haze
{
    /// <summary>
    /// Canonical transaction payload for signing
    /// Must exactly match Rust ConsensusEngine::get_transaction_data_for_signing
    /// </summary>
    public static class TransactionSigning
    {
        /// <summary>
        /// Get transaction data for signing (canonical payload)
        /// </summary>
        public static byte[] GetTransactionDataForSigning(TransferTransaction tx)
        {
            var parts = new List<byte[]>();
            parts.Add(Encoding.UTF8.GetBytes("Transfer"));
            parts.Add(tx.from.Bytes);
            parts.Add(tx.to.Bytes);
            parts.Add(U64LE(ulong.Parse(tx.amount)));
            parts.Add(U64LE(ulong.Parse(tx.fee)));
            parts.Add(U64LE((ulong)tx.nonce));
            AppendChainFields(parts, tx.chain_id, tx.valid_until_height);
            return ConcatBytes(parts);
        }

        /// <summary>
        /// Get transaction data for signing (canonical payload)
        /// </summary>
        public static byte[] GetTransactionDataForSigning(StakeTransaction tx)
        {
            var parts = new List<byte[]>();
            parts.Add(Encoding.UTF8.GetBytes("Stake"));
            parts.Add(tx.from.Bytes);
            parts.Add(tx.validator.Bytes);
            parts.Add(U64LE(ulong.Parse(tx.amount)));
            parts.Add(U64LE(ulong.Parse(tx.fee)));
            parts.Add(U64LE((ulong)tx.nonce));
            AppendChainFields(parts, tx.chain_id, tx.valid_until_height);
            return ConcatBytes(parts);
        }

        /// <summary>
        /// Get transaction data for signing (canonical payload)
        /// </summary>
        public static byte[] GetTransactionDataForSigning(MistbornAssetTransaction tx)
        {
            var parts = new List<byte[]>();
            parts.Add(Encoding.UTF8.GetBytes("MistbornAsset"));
            parts.Add(tx.from.Bytes);
            parts.Add(new byte[] { (byte)tx.action });
            parts.Add(tx.asset_id.Bytes);
            parts.Add(tx.data.owner.Bytes);
            parts.Add(new byte[] { (byte)tx.data.density });

            // For Merge: include other_asset_id in signature
            if (tx.action == AssetAction.Merge)
            {
                if (tx.data.metadata != null && tx.data.metadata.TryGetValue("_other_asset_id", out var otherIdHex))
                {
                    var otherBytes = HexToBytes(otherIdHex);
                    if (otherBytes.Length == 32)
                        parts.Add(otherBytes);
                }
            }

            // For Split: include components in signature
            if (tx.action == AssetAction.Split)
            {
                if (tx.data.metadata != null && tx.data.metadata.TryGetValue("_components", out var componentsStr))
                {
                    parts.Add(Encoding.UTF8.GetBytes(componentsStr));
                }
            }

            parts.Add(U64LE(ulong.Parse(tx.fee)));
            parts.Add(U64LE((ulong)tx.nonce));
            AppendChainFields(parts, tx.chain_id, tx.valid_until_height);

            return ConcatBytes(parts);
        }

        /// <summary>
        /// Sign a transaction with a key pair
        /// </summary>
        public static byte[] SignTransaction(TransferTransaction tx, Crypto.KeyPair keyPair)
        {
            var payload = GetTransactionDataForSigning(tx);
            return keyPair.Sign(payload);
        }

        /// <summary>
        /// Sign a transaction with a key pair
        /// </summary>
        public static byte[] SignTransaction(StakeTransaction tx, Crypto.KeyPair keyPair)
        {
            var payload = GetTransactionDataForSigning(tx);
            return keyPair.Sign(payload);
        }

        /// <summary>
        /// Sign a transaction with a key pair
        /// </summary>
        public static byte[] SignTransaction(MistbornAssetTransaction tx, Crypto.KeyPair keyPair)
        {
            var payload = GetTransactionDataForSigning(tx);
            return keyPair.Sign(payload);
        }

        private static byte[] U64LE(ulong value)
        {
            var bytes = new byte[8];
            for (int i = 0; i < 8; i++)
            {
                bytes[i] = (byte)(value & 0xFF);
                value >>= 8;
            }
            return bytes;
        }

        private static void AppendChainFields(List<byte[]> parts, int? chainId, int? validUntilHeight)
        {
            if (chainId.HasValue)
                parts.Add(U64LE((ulong)chainId.Value));
            if (validUntilHeight.HasValue)
                parts.Add(U64LE((ulong)validUntilHeight.Value));
        }

        private static byte[] ConcatBytes(List<byte[]> parts)
        {
            var totalLength = parts.Sum(p => p.Length);
            var result = new byte[totalLength];
            int offset = 0;
            foreach (var part in parts)
            {
                Buffer.BlockCopy(part, 0, result, offset, part.Length);
                offset += part.Length;
            }
            return result;
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
