using System;
using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Haze
{
    /// <summary>
    /// JSON converter for transactions to API format (hex for bytes, string for bigint)
    /// </summary>
    public class TransactionJsonConverter : JsonConverter
    {
        public override bool CanConvert(Type objectType)
        {
            return objectType == typeof(TransferTransaction) ||
                   objectType == typeof(MistbornAssetTransaction) ||
                   objectType == typeof(StakeTransaction) ||
                   objectType == typeof(Address) ||
                   objectType == typeof(Hash);
        }

        public override object ReadJson(JsonReader reader, Type objectType, object existingValue, JsonSerializer serializer)
        {
            throw new NotImplementedException("Deserialization not implemented");
        }

        public override void WriteJson(JsonWriter writer, object value, JsonSerializer serializer)
        {
            if (value is Address addr)
            {
                writer.WriteValue(Utils.BytesToHex(addr.Bytes));
            }
            else if (value is Hash hash)
            {
                writer.WriteValue(Utils.BytesToHex(hash.Bytes));
            }
            else if (value is TransferTransaction tx)
            {
                var obj = new JObject
                {
                    ["Transfer"] = new JObject
                    {
                    ["from"] = Utils.BytesToHex(tx.from.Bytes),
                    ["to"] = Utils.BytesToHex(tx.to.Bytes),
                        ["amount"] = tx.amount,
                        ["fee"] = tx.fee,
                        ["nonce"] = tx.nonce,
                        ["signature"] = tx.signature
                    }
                };
                if (tx.chain_id.HasValue)
                    obj["Transfer"]["chain_id"] = tx.chain_id.Value;
                if (tx.valid_until_height.HasValue)
                    obj["Transfer"]["valid_until_height"] = tx.valid_until_height.Value;
                obj.WriteTo(writer);
            }
            else if (value is MistbornAssetTransaction mtx)
            {
                var dataObj = new JObject
                {
                    ["density"] = mtx.data.density.ToString(),
                    ["metadata"] = JObject.FromObject(mtx.data.metadata ?? new Dictionary<string, string>()),
                    ["attributes"] = JArray.FromObject(mtx.data.attributes ?? new List<Attribute>()),
                    ["owner"] = Utils.BytesToHex(mtx.data.owner.Bytes)
                };
                if (!string.IsNullOrEmpty(mtx.data.game_id))
                    dataObj["game_id"] = mtx.data.game_id;

                var obj = new JObject
                {
                    ["MistbornAsset"] = new JObject
                    {
                        ["from"] = Utils.BytesToHex(mtx.from.Bytes),
                        ["action"] = mtx.action.ToString(),
                        ["asset_id"] = Utils.BytesToHex(mtx.asset_id.Bytes),
                        ["data"] = dataObj,
                        ["fee"] = mtx.fee,
                        ["nonce"] = mtx.nonce,
                        ["signature"] = mtx.signature
                    }
                };
                if (mtx.chain_id.HasValue)
                    obj["MistbornAsset"]["chain_id"] = mtx.chain_id.Value;
                if (mtx.valid_until_height.HasValue)
                    obj["MistbornAsset"]["valid_until_height"] = mtx.valid_until_height.Value;
                obj.WriteTo(writer);
            }
            else if (value is StakeTransaction stx)
            {
                var obj = new JObject
                {
                    ["Stake"] = new JObject
                    {
                        ["from"] = Utils.BytesToHex(stx.from.Bytes),
                        ["validator"] = Utils.BytesToHex(stx.validator.Bytes),
                        ["amount"] = stx.amount,
                        ["fee"] = stx.fee,
                        ["nonce"] = stx.nonce,
                        ["signature"] = stx.signature
                    }
                };
                if (stx.chain_id.HasValue)
                    obj["Stake"]["chain_id"] = stx.chain_id.Value;
                if (stx.valid_until_height.HasValue)
                    obj["Stake"]["valid_until_height"] = stx.valid_until_height.Value;
                obj.WriteTo(writer);
            }
        }
    }
}
