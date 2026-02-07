// Copyright HAZE Blockchain. Blueprint-callable helpers.

#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "HazeTypes.h"
#include "HazeBlueprintLibrary.generated.h"

class UHazeClient;
class UHazeKeyPair;

UCLASS()
class HAZEBLOCKCHAIN_API UHazeBlueprintLibrary : public UBlueprintFunctionLibrary
{
	GENERATED_BODY()
public:
	/** Build signed Transfer JSON for Send Transaction. Returns empty if signing not available. */
	UFUNCTION(BlueprintCallable, Category = "HAZE", meta = (DisplayName = "Build Signed Transfer"))
	static FString BuildSignedTransfer(
		UHazeKeyPair* KeyPair,
		const FString& ToAddressHex,
		int64 Amount,
		int64 Fee,
		int64 Nonce);

	/** Build signed MistbornAsset Create JSON. Returns empty if signing not available. */
	UFUNCTION(BlueprintCallable, Category = "HAZE", meta = (DisplayName = "Build Signed Mistborn Create"))
	static FString BuildSignedMistbornCreate(
		UHazeKeyPair* KeyPair,
		const FString& AssetIdHex,
		EDensityLevel Density,
		const TMap<FString, FString>& Metadata,
		const FString& GameId,
		int64 Fee,
		int64 Nonce);

	/** Check if signing is available (Ed25519 linked). */
	UFUNCTION(BlueprintPure, Category = "HAZE")
	static bool IsSigningAvailable();
};
