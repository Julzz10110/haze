// Copyright HAZE Blockchain. Canonical transaction payload for signing (matches Rust consensus).

#pragma once

#include "CoreMinimal.h"
#include "HazeTypes.h"

struct HAZEBLOCKCHAIN_API FTransactionSigning
{
	/** Build canonical payload for Transfer (matches Rust get_transaction_data_for_signing) */
	static TArray<uint8> BuildTransferPayload(
		const TArray<uint8>& FromAddress,
		const TArray<uint8>& ToAddress,
		uint64 Amount,
		uint64 Fee,
		uint64 Nonce,
		TOptional<uint64> ChainId = {},
		TOptional<uint64> ValidUntilHeight = {});

	/** Build canonical payload for MistbornAsset Create (and other actions). */
	static TArray<uint8> BuildMistbornAssetPayload(
		const TArray<uint8>& FromAddress,
		EAssetAction Action,
		const TArray<uint8>& AssetId,
		const TArray<uint8>& DataOwner,
		EDensityLevel Density,
		uint64 Fee,
		uint64 Nonce,
		const TMap<FString, FString>& MetadataMergeSplit,
		TOptional<uint64> ChainId = {},
		TOptional<uint64> ValidUntilHeight = {});

	/** Append optional chain_id and valid_until_height (LE) */
	static void AppendChainFields(TArray<uint8>& Payload, TOptional<uint64> ChainId, TOptional<uint64> ValidUntilHeight);
};
