// Copyright HAZE Blockchain. HTTP client implementation.

#include "HazeClient.h"
#include "HttpModule.h"
#include "Interfaces/IHttpResponse.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

UHazeClient::UHazeClient()
{
}

UHazeClient* UHazeClient::CreateClient(const FString& InBaseUrl)
{
	UHazeClient* Client = NewObject<UHazeClient>();
	Client->BaseUrl = InBaseUrl;
	return Client;
}

FString UHazeClient::NormalizeBaseUrl() const
{
	FString Url = BaseUrl.TrimStartAndEnd();
	if (Url.EndsWith(TEXT("/")))
	{
		Url.LeftChopInline(1);
	}
	return Url;
}

void UHazeClient::GetHealth(const FHazeHealthDelegate& OnComplete)
{
	FString Url = NormalizeBaseUrl() + TEXT("/health");
	TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
	Request->SetURL(Url);
	Request->SetVerb(TEXT("GET"));
	Request->SetTimeout(TimeoutSeconds);

	auto Ctx = MakeShared<FHazeHealthDelegate>(OnComplete);
	Request->OnProcessRequestComplete().BindLambda([this, Ctx](FHttpRequestPtr Req, FHttpResponsePtr Res, bool bOk)
	{
		FString Health;
		if (bOk && Res.IsValid() && Res->GetResponseCode() == 200)
		{
			Health = Res->GetContentAsString();
			TSharedPtr<FJsonObject> Json;
			TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Health);
			if (FJsonSerializer::Deserialize(Reader, Json) && Json.IsValid() && Json->HasField(TEXT("data")))
			{
				Health = Json->GetStringField(TEXT("data"));
			}
		}
		if (Ctx->IsBound())
		{
			Ctx->Execute(Health);
		}
	});
	Request->ProcessRequest();
}

void UHazeClient::GetBlockchainInfo(const FHazeBlockchainInfoDelegate& OnComplete)
{
	FString Url = NormalizeBaseUrl() + TEXT("/api/v1/blockchain/info");
	TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
	Request->SetURL(Url);
	Request->SetVerb(TEXT("GET"));
	Request->SetTimeout(TimeoutSeconds);

	auto Ctx = MakeShared<FHazeBlockchainInfoDelegate>(OnComplete);
	Request->OnProcessRequestComplete().BindLambda([this, Ctx](FHttpRequestPtr Req, FHttpResponsePtr Res, bool bOk)
	{
		FBlockchainInfo Info;
		if (bOk && Res.IsValid() && Res->GetResponseCode() == 200)
		{
			TSharedPtr<FJsonObject> Root;
			TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Res->GetContentAsString());
			if (FJsonSerializer::Deserialize(Reader, Root) && Root.IsValid() && Root->HasField(TEXT("data")))
			{
				TSharedPtr<FJsonObject> Data = Root->GetObjectField(TEXT("data"));
				Info.CurrentHeight = Data->GetIntegerField(TEXT("current_height"));
				Info.TotalSupply = Data->GetStringField(TEXT("total_supply"));
				Info.CurrentWave = Data->GetIntegerField(TEXT("current_wave"));
				Info.StateRoot = Data->GetStringField(TEXT("state_root"));
				Info.LastFinalizedHeight = Data->GetIntegerField(TEXT("last_finalized_height"));
				Info.LastFinalizedWave = Data->GetIntegerField(TEXT("last_finalized_wave"));
			}
		}
		if (Ctx->IsBound())
		{
			Ctx->Execute(Info);
		}
	});
	Request->ProcessRequest();
}

void UHazeClient::GetBalance(const FString& AddressHex, const FHazeBalanceDelegate& OnComplete)
{
	FString Url = NormalizeBaseUrl() + FString::Printf(TEXT("/api/v1/accounts/%s/balance"), *AddressHex);
	TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
	Request->SetURL(Url);
	Request->SetVerb(TEXT("GET"));
	Request->SetTimeout(TimeoutSeconds);

	auto Ctx = MakeShared<FHazeBalanceDelegate>(OnComplete);
	Request->OnProcessRequestComplete().BindLambda([Ctx](FHttpRequestPtr Req, FHttpResponsePtr Res, bool bOk)
	{
		FString Balance;
		if (bOk && Res.IsValid() && Res->GetResponseCode() == 200)
		{
			TSharedPtr<FJsonObject> Root;
			TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Res->GetContentAsString());
			if (FJsonSerializer::Deserialize(Reader, Root) && Root.IsValid() && Root->HasField(TEXT("data")))
			{
				Balance = Root->GetStringField(TEXT("data"));
			}
		}
		if (Ctx->IsBound())
		{
			Ctx->Execute(Balance);
		}
	});
	Request->ProcessRequest();
}

void UHazeClient::GetAccount(const FString& AddressHex, const FHazeAccountInfoDelegate& OnComplete)
{
	FString Url = NormalizeBaseUrl() + FString::Printf(TEXT("/api/v1/accounts/%s"), *AddressHex);
	TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
	Request->SetURL(Url);
	Request->SetVerb(TEXT("GET"));
	Request->SetTimeout(TimeoutSeconds);

	auto Ctx = MakeShared<FHazeAccountInfoDelegate>(OnComplete);
	Request->OnProcessRequestComplete().BindLambda([Ctx](FHttpRequestPtr Req, FHttpResponsePtr Res, bool bOk)
	{
		FAccountInfo Info;
		if (bOk && Res.IsValid() && Res->GetResponseCode() == 200)
		{
			TSharedPtr<FJsonObject> Root;
			TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Res->GetContentAsString());
			if (FJsonSerializer::Deserialize(Reader, Root) && Root.IsValid() && Root->HasField(TEXT("data")))
			{
				TSharedPtr<FJsonObject> Data = Root->GetObjectField(TEXT("data"));
				Info.Balance = Data->GetStringField(TEXT("balance"));
				Info.Nonce = Data->GetIntegerField(TEXT("nonce"));
				Info.Staked = Data->GetStringField(TEXT("staked"));
			}
		}
		if (Ctx->IsBound())
		{
			Ctx->Execute(Info);
		}
	});
	Request->ProcessRequest();
}

void UHazeClient::SendTransaction(const FString& TransactionJson, const FHazeTransactionDelegate& OnComplete)
{
	FString Url = NormalizeBaseUrl() + TEXT("/api/v1/transactions");
	FString Body = FString::Printf(TEXT("{\"transaction\":%s}"), *TransactionJson);

	TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
	Request->SetURL(Url);
	Request->SetVerb(TEXT("POST"));
	Request->SetHeader(TEXT("Content-Type"), TEXT("application/json"));
	Request->SetContentAsString(Body);
	Request->SetTimeout(TimeoutSeconds);

	auto Ctx = MakeShared<FHazeTransactionDelegate>(OnComplete);
	Request->OnProcessRequestComplete().BindLambda([Ctx](FHttpRequestPtr Req, FHttpResponsePtr Res, bool bOk)
	{
		FTransactionResponse TxRes;
		bool bSuccess = false;
		if (bOk && Res.IsValid())
		{
			TSharedPtr<FJsonObject> Root;
			TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Res->GetContentAsString());
			if (FJsonSerializer::Deserialize(Reader, Root) && Root.IsValid())
			{
				bSuccess = Root->GetBoolField(TEXT("success"));
				if (bSuccess && Root->HasField(TEXT("data")))
				{
					TSharedPtr<FJsonObject> Data = Root->GetObjectField(TEXT("data"));
					TxRes.Hash = Data->GetStringField(TEXT("hash"));
					TxRes.Status = Data->GetStringField(TEXT("status"));
				}
			}
		}
		if (Ctx->IsBound())
		{
			Ctx->Execute(bSuccess, TxRes);
		}
	});
	Request->ProcessRequest();
}

void UHazeClient::GetHealthSync(const FString& BaseUrl, FString& OutHealth, FString& OutError)
{
	OutHealth.Empty();
	OutError.Empty();
	// Sync HTTP in Unreal is not straightforward; typically use async. For BlueprintPure we return empty and document "use async GetHealth".
	OutError = TEXT("Use async GetHealth instead");
}
