using System;
using System.Collections.Generic;

namespace Haze
{
    /// <summary>
    /// 32-byte hash or address
    /// </summary>
    public struct Hash : IEquatable<Hash>
    {
        public byte[] Bytes { get; }

        public Hash(byte[] bytes)
        {
            if (bytes == null || bytes.Length != 32)
                throw new ArgumentException("Hash must be 32 bytes", nameof(bytes));
            Bytes = bytes;
        }

        public bool Equals(Hash other) => BytesEqual(Bytes, other.Bytes);
        public override bool Equals(object obj) => obj is Hash other && Equals(other);
        public override int GetHashCode() => Bytes != null ? BitConverter.ToInt32(Bytes, 0) : 0;
        private static bool BytesEqual(byte[] a, byte[] b)
        {
            if (a == null || b == null || a.Length != b.Length) return false;
            for (int i = 0; i < a.Length; i++)
                if (a[i] != b[i]) return false;
            return true;
        }
    }

    /// <summary>
    /// 32-byte address (Ed25519 public key)
    /// </summary>
    public struct Address : IEquatable<Address>
    {
        public byte[] Bytes { get; }

        public Address(byte[] bytes)
        {
            if (bytes == null || bytes.Length != 32)
                throw new ArgumentException("Address must be 32 bytes", nameof(bytes));
            Bytes = bytes;
        }

        public bool Equals(Address other) => BytesEqual(Bytes, other.Bytes);
        public override bool Equals(object obj) => obj is Address other && Equals(other);
        public override int GetHashCode() => Bytes != null ? BitConverter.ToInt32(Bytes, 0) : 0;
        private static bool BytesEqual(byte[] a, byte[] b)
        {
            if (a == null || b == null || a.Length != b.Length) return false;
            for (int i = 0; i < a.Length; i++)
                if (a[i] != b[i]) return false;
            return true;
        }
    }

    /// <summary>
    /// Density levels for Mistborn assets
    /// </summary>
    public enum DensityLevel
    {
        Ethereal = 0, // 5KB
        Light = 1,    // 50KB
        Dense = 2,    // 5MB
        Core = 3      // 50MB+
    }

    /// <summary>
    /// Maximum size for each density level (bytes)
    /// </summary>
    public static class DensityLimits
    {
        public const int Ethereal = 5 * 1024;
        public const int Light = 50 * 1024;
        public const int Dense = 5 * 1024 * 1024;
        public const int Core = 50 * 1024 * 1024;
    }

    /// <summary>
    /// Actions for Mistborn assets
    /// </summary>
    public enum AssetAction
    {
        Create = 0,
        Update = 1,
        Condense = 2,
        Evaporate = 3,
        Merge = 4,
        Split = 5
    }

    /// <summary>
    /// Attribute for NFT
    /// </summary>
    [Serializable]
    public class Attribute
    {
        public string name;
        public string value;
        public double? rarity;
    }

    /// <summary>
    /// Asset data with density levels
    /// </summary>
    [Serializable]
    public class AssetData
    {
        public DensityLevel density;
        public Dictionary<string, string> metadata;
        public List<Attribute> attributes;
        public string game_id;
        public Address owner;
    }

    /// <summary>
    /// Transfer transaction
    /// </summary>
    [Serializable]
    public class TransferTransaction
    {
        public const string Type = "Transfer";
        public Address from;
        public Address to;
        public string amount; // bigint as string
        public string fee;   // bigint as string
        public int nonce;
        public int? chain_id;
        public int? valid_until_height;
        public string signature; // hex
    }

    /// <summary>
    /// Mistborn asset transaction
    /// </summary>
    [Serializable]
    public class MistbornAssetTransaction
    {
        public const string Type = "MistbornAsset";
        public Address from;
        public AssetAction action;
        public Hash asset_id;
        public AssetData data;
        public string fee;   // bigint as string
        public int nonce;
        public int? chain_id;
        public int? valid_until_height;
        public string signature; // hex
    }

    /// <summary>
    /// Stake transaction
    /// </summary>
    [Serializable]
    public class StakeTransaction
    {
        public const string Type = "Stake";
        public Address from;
        public Address validator;
        public string amount; // bigint as string
        public string fee;   // bigint as string
        public int nonce;
        public int? chain_id;
        public int? valid_until_height;
        public string signature; // hex
    }

    /// <summary>
    /// Account information
    /// </summary>
    [Serializable]
    public class AccountInfo
    {
        public string address;
        public string balance; // bigint as string
        public int nonce;
        public string staked; // bigint as string
    }

    /// <summary>
    /// Asset information
    /// </summary>
    [Serializable]
    public class AssetInfo
    {
        public string asset_id;
        public string owner;
        public string density;
        public long created_at;
        public long updated_at;
    }

    /// <summary>
    /// Blockchain information
    /// </summary>
    [Serializable]
    public class BlockchainInfo
    {
        public int current_height;
        public string total_supply; // bigint as string
        public int current_wave;
        public string state_root;
        public int last_finalized_height;
        public int last_finalized_wave;
    }

    /// <summary>
    /// Transaction response
    /// </summary>
    [Serializable]
    public class TransactionResponse
    {
        public string hash;
        public string status; // "pending" | "confirmed" | "failed"
    }

    /// <summary>
    /// Block information
    /// </summary>
    [Serializable]
    public class BlockInfo
    {
        public string hash;
        public string parent_hash;
        public int height;
        public long timestamp;
        public string validator;
        public int transaction_count;
        public int wave_number;
    }

    /// <summary>
    /// Liquidity pool
    /// </summary>
    [Serializable]
    public class LiquidityPool
    {
        public string pool_id;
        public string asset1;
        public string asset2;
        public string reserve1; // bigint as string
        public string reserve2; // bigint as string
        public int fee_rate;
        public string total_liquidity; // bigint as string
    }

    /// <summary>
    /// API response wrapper
    /// </summary>
    [Serializable]
    public class ApiResponse<T>
    {
        public bool success;
        public T data;
        public string error;
    }
}
