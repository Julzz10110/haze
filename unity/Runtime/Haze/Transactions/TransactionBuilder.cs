using System;
using System.Collections.Generic;
using System.Security.Cryptography;
using System.Text;
using Haze.Crypto;

namespace Haze
{
    /// <summary>
    /// Transaction builder for HAZE blockchain
    /// </summary>
    public static class TransactionBuilder
    {
        /// <summary>
        /// Create a transfer transaction
        /// </summary>
        public static TransferTransaction CreateTransfer(
            Address from,
            Address to,
            ulong amount,
            ulong fee,
            int nonce,
            int? chainId = null,
            int? validUntilHeight = null)
        {
            return new TransferTransaction
            {
                from = from,
                to = to,
                amount = amount.ToString(),
                fee = fee.ToString(),
                nonce = nonce,
                chain_id = chainId,
                valid_until_height = validUntilHeight,
                signature = ""
            };
        }

        /// <summary>
        /// Sign a transfer transaction
        /// </summary>
        public static TransferTransaction Sign(TransferTransaction tx, KeyPair keyPair)
        {
            var signature = TransactionSigning.SignTransaction(tx, keyPair);
            tx.signature = Utils.BytesToHex(signature);
            return tx;
        }

        /// <summary>
        /// Create a stake transaction
        /// </summary>
        public static StakeTransaction CreateStake(
            Address from,
            Address validator,
            ulong amount,
            ulong fee,
            int nonce,
            int? chainId = null,
            int? validUntilHeight = null)
        {
            return new StakeTransaction
            {
                from = from,
                validator = validator,
                amount = amount.ToString(),
                fee = fee.ToString(),
                nonce = nonce,
                chain_id = chainId,
                valid_until_height = validUntilHeight,
                signature = ""
            };
        }

        /// <summary>
        /// Sign a stake transaction
        /// </summary>
        public static StakeTransaction Sign(StakeTransaction tx, KeyPair keyPair)
        {
            var signature = TransactionSigning.SignTransaction(tx, keyPair);
            tx.signature = Utils.BytesToHex(signature);
            return tx;
        }

        /// <summary>
        /// Create asset ID from string seed (SHA-256)
        /// </summary>
        public static Hash CreateAssetId(string seed)
        {
            return Utils.Sha256(seed);
        }

        /// <summary>
        /// Create asset ID from bytes (SHA-256)
        /// </summary>
        public static Hash CreateAssetId(byte[] data)
        {
            return Utils.Sha256(data);
        }

        /// <summary>
        /// Create a MistbornAsset Create transaction
        /// </summary>
        public static MistbornAssetTransaction CreateAsset(
            Hash assetId,
            Address owner,
            DensityLevel density,
            Dictionary<string, string> metadata = null,
            List<Attribute> attributes = null,
            string gameId = null,
            ulong fee = 0,
            int nonce = 0,
            int? chainId = null,
            int? validUntilHeight = null)
        {
            return new MistbornAssetTransaction
            {
                from = owner,
                action = AssetAction.Create,
                asset_id = assetId,
                data = new AssetData
                {
                    density = density,
                    metadata = metadata ?? new Dictionary<string, string>(),
                    attributes = attributes ?? new List<Attribute>(),
                    game_id = gameId,
                    owner = owner
                },
                fee = fee.ToString(),
                nonce = nonce,
                chain_id = chainId,
                valid_until_height = validUntilHeight,
                signature = ""
            };
        }

        /// <summary>
        /// Create a MistbornAsset Update transaction
        /// </summary>
        public static MistbornAssetTransaction UpdateAsset(
            Hash assetId,
            Address owner,
            Dictionary<string, string> metadata = null,
            List<Attribute> attributes = null,
            ulong fee = 0,
            int nonce = 0,
            int? chainId = null,
            int? validUntilHeight = null)
        {
            return new MistbornAssetTransaction
            {
                from = owner,
                action = AssetAction.Update,
                asset_id = assetId,
                data = new AssetData
                {
                    density = DensityLevel.Ethereal, // Will be updated by backend
                    metadata = metadata ?? new Dictionary<string, string>(),
                    attributes = attributes ?? new List<Attribute>(),
                    owner = owner
                },
                fee = fee.ToString(),
                nonce = nonce,
                chain_id = chainId,
                valid_until_height = validUntilHeight,
                signature = ""
            };
        }

        /// <summary>
        /// Create a MistbornAsset Condense transaction
        /// </summary>
        public static MistbornAssetTransaction CondenseAsset(
            Hash assetId,
            Address owner,
            DensityLevel newDensity,
            Dictionary<string, string> additionalMetadata = null,
            List<Attribute> additionalAttributes = null,
            ulong fee = 0,
            int nonce = 0,
            int? chainId = null,
            int? validUntilHeight = null)
        {
            return new MistbornAssetTransaction
            {
                from = owner,
                action = AssetAction.Condense,
                asset_id = assetId,
                data = new AssetData
                {
                    density = newDensity,
                    metadata = additionalMetadata ?? new Dictionary<string, string>(),
                    attributes = additionalAttributes ?? new List<Attribute>(),
                    owner = owner
                },
                fee = fee.ToString(),
                nonce = nonce,
                chain_id = chainId,
                valid_until_height = validUntilHeight,
                signature = ""
            };
        }

        /// <summary>
        /// Create a MistbornAsset Evaporate transaction
        /// </summary>
        public static MistbornAssetTransaction EvaporateAsset(
            Hash assetId,
            Address owner,
            DensityLevel newDensity,
            ulong fee = 0,
            int nonce = 0,
            int? chainId = null,
            int? validUntilHeight = null)
        {
            return new MistbornAssetTransaction
            {
                from = owner,
                action = AssetAction.Evaporate,
                asset_id = assetId,
                data = new AssetData
                {
                    density = newDensity,
                    metadata = new Dictionary<string, string>(),
                    attributes = new List<Attribute>(),
                    owner = owner
                },
                fee = fee.ToString(),
                nonce = nonce,
                chain_id = chainId,
                valid_until_height = validUntilHeight,
                signature = ""
            };
        }

        /// <summary>
        /// Create a MistbornAsset Merge transaction
        /// </summary>
        public static MistbornAssetTransaction MergeAssets(
            Hash assetId,
            Hash otherAssetId,
            Address owner,
            ulong fee = 0,
            int nonce = 0,
            int? chainId = null,
            int? validUntilHeight = null)
        {
            var metadata = new Dictionary<string, string>
            {
                ["_other_asset_id"] = Utils.BytesToHex(otherAssetId.Bytes)
            };

            return new MistbornAssetTransaction
            {
                from = owner,
                action = AssetAction.Merge,
                asset_id = assetId,
                data = new AssetData
                {
                    density = DensityLevel.Ethereal, // Will be updated by backend
                    metadata = metadata,
                    attributes = new List<Attribute>(),
                    owner = owner
                },
                fee = fee.ToString(),
                nonce = nonce,
                chain_id = chainId,
                valid_until_height = validUntilHeight,
                signature = ""
            };
        }

        /// <summary>
        /// Create a MistbornAsset Split transaction
        /// </summary>
        public static MistbornAssetTransaction SplitAsset(
            Hash assetId,
            Address owner,
            List<string> componentIds,
            ulong fee = 0,
            int nonce = 0,
            int? chainId = null,
            int? validUntilHeight = null)
        {
            var metadata = new Dictionary<string, string>
            {
                ["_components"] = string.Join(",", componentIds)
            };

            return new MistbornAssetTransaction
            {
                from = owner,
                action = AssetAction.Split,
                asset_id = assetId,
                data = new AssetData
                {
                    density = DensityLevel.Ethereal, // Will be updated by backend
                    metadata = metadata,
                    attributes = new List<Attribute>(),
                    owner = owner
                },
                fee = fee.ToString(),
                nonce = nonce,
                chain_id = chainId,
                valid_until_height = validUntilHeight,
                signature = ""
            };
        }

        /// <summary>
        /// Sign a MistbornAsset transaction
        /// </summary>
        public static MistbornAssetTransaction Sign(MistbornAssetTransaction tx, KeyPair keyPair)
        {
            var signature = TransactionSigning.SignTransaction(tx, keyPair);
            tx.signature = Utils.BytesToHex(signature);
            return tx;
        }
    }
}
