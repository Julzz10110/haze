// Copyright HAZE Blockchain. Canonical payload construction.

#include "TransactionSigning.h"
#include "Misc/Parse.h"

void FTransactionSigning::AppendChainFields(TArray<uint8>& Payload, TOptional<uint64> ChainId, TOptional<uint64> ValidUntilHeight)
{
	if (ChainId.IsSet())
	{
		uint64 C = ChainId.GetValue();
		Payload.Append(reinterpret_cast<uint8*>(&C), sizeof(uint64));
	}
	if (ValidUntilHeight.IsSet())
	{
		uint64 V = ValidUntilHeight.GetValue();
		Payload.Append(reinterpret_cast<uint8*>(&V), sizeof(uint64));
	}
}

TArray<uint8> FTransactionSigning::BuildTransferPayload(
	const TArray<uint8>& FromAddress,
	const TArray<uint8>& ToAddress,
	uint64 Amount,
	uint64 Fee,
	uint64 Nonce,
	TOptional<uint64> ChainId,
	TOptional<uint64> ValidUntilHeight)
{
	TArray<uint8> Data;
	Data.Append(reinterpret_cast<const uint8*>(TCHAR_TO_UTF8("Transfer")), 8);
	if (FromAddress.Num() >= 32) Data.Append(FromAddress.GetData(), 32);
	if (ToAddress.Num() >= 32) Data.Append(ToAddress.GetData(), 32);
	Data.Append(reinterpret_cast<uint8*>(&Amount), sizeof(uint64));
	Data.Append(reinterpret_cast<uint8*>(&Fee), sizeof(uint64));
	Data.Append(reinterpret_cast<uint8*>(&Nonce), sizeof(uint64));
	AppendChainFields(Data, ChainId, ValidUntilHeight);
	return Data;
}

TArray<uint8> FTransactionSigning::BuildMistbornAssetPayload(
	const TArray<uint8>& FromAddress,
	EAssetAction Action,
	const TArray<uint8>& AssetId,
	const TArray<uint8>& DataOwner,
	EDensityLevel Density,
	uint64 Fee,
	uint64 Nonce,
	const TMap<FString, FString>& MetadataMergeSplit,
	TOptional<uint64> ChainId,
	TOptional<uint64> ValidUntilHeight)
{
	TArray<uint8> Data;
	Data.Append(reinterpret_cast<const uint8*>(TCHAR_TO_UTF8("MistbornAsset")), 13);
	if (FromAddress.Num() >= 32) Data.Append(FromAddress.GetData(), 32);
	uint8 ActionByte = static_cast<uint8>(Action);
	Data.Add(ActionByte);
	if (AssetId.Num() >= 32) Data.Append(AssetId.GetData(), 32);
	if (DataOwner.Num() >= 32) Data.Append(DataOwner.GetData(), 32);
	uint8 DensityByte = static_cast<uint8>(Density);
	Data.Add(DensityByte);

	if (Action == EAssetAction::Merge)
	{
		const FString* OtherId = MetadataMergeSplit.Find(TEXT("_other_asset_id"));
		if (OtherId && OtherId->Len() >= 64)
		{
			// Decode hex to 32 bytes and append
			TArray<uint8> OtherBytes;
			OtherBytes.SetNum(32);
			for (int32 i = 0; i < 32; i++)
			{
				int32 A = FParse::HexDigit((*OtherId)[i * 2]);
				int32 B = FParse::HexDigit((*OtherId)[i * 2 + 1]);
				OtherBytes[i] = static_cast<uint8>((A << 4) | B);
			}
			Data.Append(OtherBytes);
		}
	}
	if (Action == EAssetAction::Split)
	{
		const FString* Components = MetadataMergeSplit.Find(TEXT("_components"));
		if (Components)
		{
			Data.Append(reinterpret_cast<const uint8*>(TCHAR_TO_UTF8(**Components)), Components->Len());
		}
	}

	Data.Append(reinterpret_cast<uint8*>(&Fee), sizeof(uint64));
	Data.Append(reinterpret_cast<uint8*>(&Nonce), sizeof(uint64));
	AppendChainFields(Data, ChainId, ValidUntilHeight);
	return Data;
}
