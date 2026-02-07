// Copyright HAZE Blockchain. HTTP client for HAZE REST API.

#pragma once

#include "CoreMinimal.h"
#include "HazeTypes.h"
#include "Interfaces/IHttpRequest.h"
#include "HazeClient.generated.h"

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FHazeHealthDelegate, const FString&, Health);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FHazeBalanceDelegate, const FString&, Balance);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FHazeBlockchainInfoDelegate, const FBlockchainInfo&, Info);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FHazeAccountInfoDelegate, const FAccountInfo&, Info);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FHazeTransactionDelegate, bool, bSuccess, const FTransactionResponse&, Response);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FHazeErrorDelegate, bool, bSuccess, const FString&, ErrorMessage);

UCLASS(BlueprintType)
class HAZEBLOCKCHAIN_API UHazeClient : public UObject
{
	GENERATED_BODY()
public:
	UHazeClient();

	/** Base URL of the HAZE node (e.g. http://localhost:8080) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "HAZE")
	FString BaseUrl;

	/** Timeout in seconds */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "HAZE", meta = (ClampMin = "1", ClampMax = "120"))
	int32 TimeoutSeconds = 30;

	/** Create client with base URL (Blueprint factory) */
	UFUNCTION(BlueprintCallable, Category = "HAZE", meta = (DisplayName = "Create Haze Client"))
	static UHazeClient* CreateClient(const FString& InBaseUrl);

	/** GET /health */
	UFUNCTION(BlueprintCallable, Category = "HAZE")
	void GetHealth(const FHazeHealthDelegate& OnComplete);

	/** GET /api/v1/blockchain/info */
	UFUNCTION(BlueprintCallable, Category = "HAZE")
	void GetBlockchainInfo(const FHazeBlockchainInfoDelegate& OnComplete);

	/** GET /api/v1/accounts/{address}/balance */
	UFUNCTION(BlueprintCallable, Category = "HAZE")
	void GetBalance(const FString& AddressHex, const FHazeBalanceDelegate& OnComplete);

	/** GET /api/v1/accounts/{address} */
	UFUNCTION(BlueprintCallable, Category = "HAZE")
	void GetAccount(const FString& AddressHex, const FHazeAccountInfoDelegate& OnComplete);

	/** POST /api/v1/transactions with body { "transaction": <object> }. TransactionJson must be the inner object (e.g. {"Transfer":{...}}) */
	UFUNCTION(BlueprintCallable, Category = "HAZE")
	void SendTransaction(const FString& TransactionJson, const FHazeTransactionDelegate& OnComplete);

	/** One-shot GET health (for simple tests). Returns health string or empty on error. */
	UFUNCTION(BlueprintPure, Category = "HAZE")
	static void GetHealthSync(const FString& BaseUrl, FString& OutHealth, FString& OutError);

private:
	void OnHttpResponse(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bSuccess,
		TSharedPtr<FHazeHealthDelegate> OnHealth);
	void OnHttpResponseBlockchain(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bSuccess,
		TSharedPtr<FHazeBlockchainInfoDelegate> OnInfo);
	void OnHttpResponseBalance(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bSuccess,
		TSharedPtr<FHazeBalanceDelegate> OnBalance);
	void OnHttpResponseAccount(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bSuccess,
		TSharedPtr<FHazeAccountInfoDelegate> OnAccount);
	void OnHttpResponseSendTx(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bSuccess,
		TSharedPtr<FHazeTransactionDelegate> OnTx);

	FString NormalizeBaseUrl() const;
};
