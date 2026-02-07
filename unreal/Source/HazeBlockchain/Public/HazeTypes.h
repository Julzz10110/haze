// Copyright HAZE Blockchain. Types for HAZE API (match REST/OpenAPI).

#pragma once

#include "CoreMinimal.h"
#include "HazeTypes.generated.h"

USTRUCT(BlueprintType)
struct HAZEBLOCKCHAIN_API FBlockchainInfo
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly) int64 CurrentHeight = 0;
	UPROPERTY(BlueprintReadOnly) FString TotalSupply;
	UPROPERTY(BlueprintReadOnly) int64 CurrentWave = 0;
	UPROPERTY(BlueprintReadOnly) FString StateRoot;
	UPROPERTY(BlueprintReadOnly) int64 LastFinalizedHeight = 0;
	UPROPERTY(BlueprintReadOnly) int64 LastFinalizedWave = 0;
};

USTRUCT(BlueprintType)
struct HAZEBLOCKCHAIN_API FAccountInfo
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly) FString Balance;
	UPROPERTY(BlueprintReadOnly) int32 Nonce = 0;
	UPROPERTY(BlueprintReadOnly) FString Staked;
};

USTRUCT(BlueprintType)
struct HAZEBLOCKCHAIN_API FTransactionResponse
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly) FString Hash;
	UPROPERTY(BlueprintReadOnly) FString Status;
};

USTRUCT(BlueprintType)
struct HAZEBLOCKCHAIN_API FLiquidityPool
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly) FString PoolId;
	UPROPERTY(BlueprintReadOnly) FString Asset1;
	UPROPERTY(BlueprintReadOnly) FString Asset2;
	UPROPERTY(BlueprintReadOnly) FString Reserve1;
	UPROPERTY(BlueprintReadOnly) FString Reserve2;
	UPROPERTY(BlueprintReadOnly) int32 FeeRate = 0;
	UPROPERTY(BlueprintReadOnly) FString TotalLiquidity;
};

UENUM(BlueprintType)
enum class EDensityLevel : uint8
{
	Ethereal = 0,
	Light = 1,
	Dense = 2,
	Core = 3
};

UENUM(BlueprintType)
enum class EAssetAction : uint8
{
	Create = 0,
	Update = 1,
	Condense = 2,
	Evaporate = 3,
	Merge = 4,
	Split = 5
};
