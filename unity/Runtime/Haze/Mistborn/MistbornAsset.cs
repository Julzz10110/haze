using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Haze;
using Haze.Crypto;

namespace Haze.Mistborn
{
    /// <summary>
    /// High-level API for Mistborn asset operations
    /// </summary>
    public class MistbornAsset
    {
        private readonly HazeClient _client;
        private readonly KeyPair _keyPair;

        public MistbornAsset(HazeClient client, KeyPair keyPair)
        {
            _client = client ?? throw new ArgumentNullException(nameof(client));
            _keyPair = keyPair ?? throw new ArgumentNullException(nameof(keyPair));
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
        /// Get asset ID as hex string
        /// </summary>
        public static string AssetIdToHex(Hash assetId)
        {
            return Utils.BytesToHex(assetId.Bytes);
        }

        /// <summary>
        /// Get asset ID from hex string
        /// </summary>
        public static Hash? HexToAssetId(string hex)
        {
            try
            {
                var bytes = Utils.HexToBytes(hex);
                if (bytes.Length != 32)
                    return null;
                return new Hash(bytes);
            }
            catch
            {
                return null;
            }
        }

        /// <summary>
        /// Create a new asset
        /// </summary>
        public async Task<TransactionResponse> CreateAsync(
            Hash assetId,
            DensityLevel density,
            Dictionary<string, string> metadata = null,
            List<Attribute> attributes = null,
            string gameId = null,
            ulong fee = 0,
            int nonce = 0)
        {
            var owner = _keyPair.GetAddress();
            var tx = TransactionBuilder.CreateAsset(
                assetId: assetId,
                owner: owner,
                density: density,
                metadata: metadata,
                attributes: attributes,
                gameId: gameId,
                fee: fee,
                nonce: nonce
            );

            var signed = TransactionBuilder.Sign(tx, _keyPair);
            return await _client.CreateAssetAsync(signed);
        }

        /// <summary>
        /// Update asset metadata and attributes
        /// </summary>
        public async Task<TransactionResponse> UpdateAsync(
            Hash assetId,
            Dictionary<string, string> metadata = null,
            List<Attribute> attributes = null,
            ulong fee = 0,
            int nonce = 0)
        {
            var owner = _keyPair.GetAddress();
            var tx = TransactionBuilder.UpdateAsset(
                assetId: assetId,
                owner: owner,
                metadata: metadata,
                attributes: attributes,
                fee: fee,
                nonce: nonce
            );

            var signed = TransactionBuilder.Sign(tx, _keyPair);
            return await _client.SendTransactionAsync(signed);
        }

        /// <summary>
        /// Condense asset (increase density)
        /// </summary>
        public async Task<TransactionResponse> CondenseAsync(
            Hash assetId,
            DensityLevel newDensity,
            Dictionary<string, string> additionalMetadata = null,
            List<Attribute> additionalAttributes = null,
            ulong fee = 0,
            int nonce = 0)
        {
            var owner = _keyPair.GetAddress();
            var tx = TransactionBuilder.CondenseAsset(
                assetId: assetId,
                owner: owner,
                newDensity: newDensity,
                additionalMetadata: additionalMetadata,
                additionalAttributes: additionalAttributes,
                fee: fee,
                nonce: nonce
            );

            var signed = TransactionBuilder.Sign(tx, _keyPair);
            return await _client.SendTransactionAsync(signed);
        }

        /// <summary>
        /// Evaporate asset (decrease density)
        /// </summary>
        public async Task<TransactionResponse> EvaporateAsync(
            Hash assetId,
            DensityLevel newDensity,
            ulong fee = 0,
            int nonce = 0)
        {
            var owner = _keyPair.GetAddress();
            var tx = TransactionBuilder.EvaporateAsset(
                assetId: assetId,
                owner: owner,
                newDensity: newDensity,
                fee: fee,
                nonce: nonce
            );

            var signed = TransactionBuilder.Sign(tx, _keyPair);
            return await _client.SendTransactionAsync(signed);
        }

        /// <summary>
        /// Merge two assets
        /// </summary>
        public async Task<TransactionResponse> MergeAsync(
            Hash assetId,
            Hash otherAssetId,
            ulong fee = 0,
            int nonce = 0)
        {
            var owner = _keyPair.GetAddress();
            var tx = TransactionBuilder.MergeAssets(
                assetId: assetId,
                otherAssetId: otherAssetId,
                owner: owner,
                fee: fee,
                nonce: nonce
            );

            var signed = TransactionBuilder.Sign(tx, _keyPair);
            return await _client.SendTransactionAsync(signed);
        }

        /// <summary>
        /// Split asset into components
        /// </summary>
        public async Task<TransactionResponse> SplitAsync(
            Hash assetId,
            List<string> componentIds,
            ulong fee = 0,
            int nonce = 0)
        {
            var owner = _keyPair.GetAddress();
            var tx = TransactionBuilder.SplitAsset(
                assetId: assetId,
                owner: owner,
                componentIds: componentIds,
                fee: fee,
                nonce: nonce
            );

            var signed = TransactionBuilder.Sign(tx, _keyPair);
            return await _client.SendTransactionAsync(signed);
        }

        /// <summary>
        /// Get asset information
        /// </summary>
        public async Task<AssetInfo> GetAssetAsync(Hash assetId)
        {
            var assetIdHex = AssetIdToHex(assetId);
            return await _client.GetAssetAsync(assetIdHex);
        }

        /// <summary>
        /// Search assets by owner
        /// </summary>
        public async Task<List<AssetInfo>> SearchByOwnerAsync()
        {
            var ownerHex = _keyPair.GetAddressHex();
            return await _client.SearchAssetsAsync(owner: ownerHex);
        }

        /// <summary>
        /// Search assets by game ID
        /// </summary>
        public async Task<List<AssetInfo>> SearchByGameIdAsync(string gameId)
        {
            return await _client.SearchAssetsAsync(gameId: gameId);
        }

        /// <summary>
        /// Search assets by owner and game ID
        /// </summary>
        public async Task<List<AssetInfo>> SearchAsync(string gameId = null)
        {
            var ownerHex = _keyPair.GetAddressHex();
            return await _client.SearchAssetsAsync(owner: ownerHex, gameId: gameId);
        }

        /// <summary>
        /// Estimate gas for asset transaction
        /// </summary>
        public async Task<GasEstimate> EstimateGasAsync(MistbornAssetTransaction transaction)
        {
            return await _client.EstimateGasAsync(transaction);
        }
    }
}
