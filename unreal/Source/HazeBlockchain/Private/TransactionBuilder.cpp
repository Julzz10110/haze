// Copyright HAZE Blockchain.

#include "TransactionBuilder.h"
#include "TransactionSigning.h"
#include "Misc/Parse.h"

FString FTransactionBuilder::BytesToHex(const TArray<uint8>& Bytes)
{
	FString Hex;
	Hex.Reserve(Bytes.Num() * 2);
	for (uint8 B : Bytes)
	{
		Hex += FString::Printf(TEXT("%02x"), B);
	}
	return Hex;
}

TArray<uint8> FTransactionBuilder::HexToBytes(const FString& Hex)
{
	TArray<uint8> Out;
	FString H = Hex.TrimStartAndEnd().Replace(TEXT(" "), TEXT(""));
	Out.SetNum(H.Len() / 2);
	for (int32 i = 0; i < Out.Num(); i++)
	{
		int32 A = FParse::HexDigit(H[i * 2]);
		int32 B = FParse::HexDigit(H[i * 2 + 1]);
		if (A < 0 || B < 0) return TArray<uint8>();
		Out[i] = static_cast<uint8>((A << 4) | B);
	}
	return Out;
}

FString FTransactionBuilder::BuildSignedTransfer(
	UHazeKeyPair* KeyPair,
	const FString& ToAddressHex,
	uint64 Amount,
	uint64 Fee,
	uint64 Nonce,
	TOptional<uint64> ChainId,
	TOptional<uint64> ValidUntilHeight)
{
	if (!KeyPair || KeyPair->PrivateKey.Num() != 32 || KeyPair->PublicKey.Num() != 32) return FString();
	TArray<uint8> ToBytes = HexToBytes(ToAddressHex);
	if (ToBytes.Num() != 32) return FString();

	TArray<uint8> Payload = FTransactionSigning::BuildTransferPayload(
		KeyPair->PublicKey, ToBytes, Amount, Fee, Nonce, ChainId, ValidUntilHeight);
	TArray<uint8> Sig = KeyPair->Sign(Payload);
	if (Sig.Num() != 64) return FString();

	FString FromHex = FTransactionBuilder::BytesToHex(KeyPair->PublicKey);
	FString ToHex = ToAddressHex.TrimStartAndEnd();
	FString SigHex = FTransactionBuilder::BytesToHex(Sig);

	return FString::Printf(TEXT("{\"Transfer\":{\"from\":\"%s\",\"to\":\"%s\",\"amount\":\"%llu\",\"fee\":\"%llu\",\"nonce\":%llu,\"signature\":\"%s\"}}"),
		*FromHex, *ToHex, Amount, Fee, Nonce, *SigHex);
}

FString FTransactionBuilder::BuildSignedMistbornCreate(
	UHazeKeyPair* KeyPair,
	const FString& AssetIdHex,
	EDensityLevel Density,
	const TMap<FString, FString>& Metadata,
	const TArray<FString>& Attributes,
	const FString& GameId,
	uint64 Fee,
	uint64 Nonce,
	TOptional<uint64> ChainId,
	TOptional<uint64> ValidUntilHeight)
{
	if (!KeyPair || KeyPair->PublicKey.Num() != 32) return FString();
	TArray<uint8> AssetIdBytes = HexToBytes(AssetIdHex);
	if (AssetIdBytes.Num() != 32) return FString();

	TMap<FString, FString> MergeSplit; // empty for Create
	TArray<uint8> Payload = FTransactionSigning::BuildMistbornAssetPayload(
		KeyPair->PublicKey, EAssetAction::Create, AssetIdBytes, KeyPair->PublicKey,
		Density, Fee, Nonce, MergeSplit, ChainId, ValidUntilHeight);
	TArray<uint8> Sig = KeyPair->Sign(Payload);
	if (Sig.Num() != 64) return FString();

	FString FromHex = FTransactionBuilder::BytesToHex(KeyPair->PublicKey);
	FString SigHex = FTransactionBuilder::BytesToHex(Sig);

	// Density string
	const TCHAR* DensityStr = TEXT("Ethereal");
	switch (Density)
	{
		case EDensityLevel::Light: DensityStr = TEXT("Light"); break;
		case EDensityLevel::Dense: DensityStr = TEXT("Dense"); break;
		case EDensityLevel::Core: DensityStr = TEXT("Core"); break;
		default: break;
	}

	// Metadata JSON object
	FString MetaJson;
	for (const auto& Pair : Metadata)
	{
		if (MetaJson.Len()) MetaJson += TEXT(",");
		MetaJson += FString::Printf(TEXT("\"%s\":\"%s\""), *Pair.Key.Replace(TEXT("\""), TEXT("\\\"")), *Pair.Value.Replace(TEXT("\""), TEXT("\\\"")));
	}
	FString DataJson = FString::Printf(TEXT("\"density\":\"%s\",\"metadata\":{%s},\"attributes\":[],\"game_id\":%s,\"owner\":\"%s\""),
		DensityStr, *MetaJson, GameId.IsEmpty() ? TEXT("null") : *FString::Printf(TEXT("\"%s\""), *GameId), *FromHex);

	return FString::Printf(TEXT("{\"MistbornAsset\":{\"from\":\"%s\",\"action\":\"Create\",\"asset_id\":\"%s\",\"data\":{%s},\"fee\":%llu,\"nonce\":%llu,\"signature\":\"%s\"}}"),
		*FromHex, *AssetIdHex.TrimStartAndEnd(), *DataJson, Fee, Nonce, *SigHex);
}
