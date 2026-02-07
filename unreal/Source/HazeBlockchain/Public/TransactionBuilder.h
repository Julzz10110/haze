// Copyright HAZE Blockchain. Build and sign Transfer / MistbornAsset (matches API contract).

#pragma once

#include "CoreMinimal.h"
#include "HazeTypes.h"
#include "KeyPair.h"

struct HAZEBLOCKCHAIN_API FTransactionBuilder
{
	/** Build signed Transfer transaction JSON (inner object for SendTransaction). Returns empty on failure. */
	static FString BuildSignedTransfer(
		UHazeKeyPair* KeyPair,
		const FString& ToAddressHex,
		uint64 Amount,
		uint64 Fee,
		uint64 Nonce,
		TOptional<uint64> ChainId = {},
		TOptional<uint64> ValidUntilHeight = {});

	/** Build signed MistbornAsset Create transaction JSON. Returns empty on failure. */
	static FString BuildSignedMistbornCreate(
		UHazeKeyPair* KeyPair,
		const FString& AssetIdHex,
		EDensityLevel Density,
		const TMap<FString, FString>& Metadata,
		const TArray<FString>& Attributes,
		const FString& GameId,
		uint64 Fee,
		uint64 Nonce,
		TOptional<uint64> ChainId = {},
		TOptional<uint64> ValidUntilHeight = {});

	/** Bytes to 64-char hex */
	static FString BytesToHex(const TArray<uint8>& Bytes);
	/** 64-char hex to bytes (32 or 64) */
	static TArray<uint8> HexToBytes(const FString& Hex);
};
