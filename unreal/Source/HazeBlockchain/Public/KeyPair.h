// Copyright HAZE Blockchain. Ed25519 key pair for signing (matches node and Unity/TS SDK).

#pragma once

#include "CoreMinimal.h"
#include "KeyPair.generated.h"

UCLASS(BlueprintType)
class HAZEBLOCKCHAIN_API UHazeKeyPair : public UObject
{
	GENERATED_BODY()
public:
	/** 32-byte secret seed / private key (for restore). Public key derived via Ed25519. */
	UPROPERTY(BlueprintReadOnly, Category = "HAZE")
	TArray<uint8> PrivateKey;

	/** 32-byte public key (= address in HAZE) */
	UPROPERTY(BlueprintReadOnly, Category = "HAZE")
	TArray<uint8> PublicKey;

	/** Generate new key pair. Returns nullptr if Ed25519 not available. */
	UFUNCTION(BlueprintCallable, Category = "HAZE", meta = (DisplayName = "Generate Key Pair"))
	static UHazeKeyPair* Generate();

	/** Restore from 32-byte private key (hex string = 64 chars). Returns nullptr on invalid input or if Ed25519 not available. */
	UFUNCTION(BlueprintCallable, Category = "HAZE", meta = (DisplayName = "Restore Key Pair from Hex"))
	static UHazeKeyPair* FromPrivateKeyHex(const FString& PrivateKeyHex);

	/** Address as 64-character hex string */
	UFUNCTION(BlueprintCallable, Category = "HAZE")
	FString GetAddressHex() const;

	/** Sign message (canonical payload). Returns 64-byte signature or empty if Ed25519 not available. */
	UFUNCTION(BlueprintCallable, Category = "HAZE")
	TArray<uint8> Sign(const TArray<uint8>& Message) const;

	/** Whether this key pair can sign (Ed25519 library linked) */
	UFUNCTION(BlueprintPure, Category = "HAZE")
	static bool IsSigningAvailable();
};
