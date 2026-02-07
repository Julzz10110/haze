// Copyright HAZE Blockchain.

#include "HazeBlueprintLibrary.h"
#include "TransactionBuilder.h"
#include "KeyPair.h"

FString UHazeBlueprintLibrary::BuildSignedTransfer(
	UHazeKeyPair* KeyPair,
	const FString& ToAddressHex,
	int64 Amount,
	int64 Fee,
	int64 Nonce)
{
	if (!KeyPair || Amount < 0 || Fee < 0 || Nonce < 0) return FString();
	return FTransactionBuilder::BuildSignedTransfer(
		KeyPair, ToAddressHex, static_cast<uint64>(Amount), static_cast<uint64>(Fee), static_cast<uint64>(Nonce));
}

FString UHazeBlueprintLibrary::BuildSignedMistbornCreate(
	UHazeKeyPair* KeyPair,
	const FString& AssetIdHex,
	EDensityLevel Density,
	const TMap<FString, FString>& Metadata,
	const FString& GameId,
	int64 Fee,
	int64 Nonce)
{
	if (!KeyPair || Fee < 0 || Nonce < 0) return FString();
	TArray<FString> Attrs;
	return FTransactionBuilder::BuildSignedMistbornCreate(
		KeyPair, AssetIdHex, Density, Metadata, Attrs, GameId, static_cast<uint64>(Fee), static_cast<uint64>(Nonce));
}

bool UHazeBlueprintLibrary::IsSigningAvailable()
{
	return UHazeKeyPair::IsSigningAvailable();
}
