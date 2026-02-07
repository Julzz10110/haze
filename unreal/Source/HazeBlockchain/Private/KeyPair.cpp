// Copyright HAZE Blockchain. Key pair and signing.

#include "KeyPair.h"
#include "TransactionSigning.h"
#include "Misc/Parse.h"
#include "Misc/SecureHash.h"
#include "HAL/PlatformMisc.h"

// Optional: link against ThirdParty/ed25519 when available
#if defined(HAZE_HAS_ED25519) && HAZE_HAS_ED25519
extern "C" {
	void ed25519_seed_keypair(const uint8_t seed[32], uint8_t public_key[32], uint8_t private_key[64]);
	void ed25519_sign(uint8_t signature[64], const uint8_t* message, size_t message_len, const uint8_t public_key[32], const uint8_t private_key[64]);
}
#endif

static TArray<uint8> HexToBytes(const FString& Hex)
{
	TArray<uint8> Out;
	Out.SetNum(Hex.Len() / 2);
	for (int32 i = 0; i < Out.Num(); i++)
	{
		int32 A = FParse::HexDigit(Hex[i * 2]);
		int32 B = FParse::HexDigit(Hex[i * 2 + 1]);
		if (A < 0 || B < 0) return TArray<uint8>();
		Out[i] = static_cast<uint8>((A << 4) | B);
	}
	return Out;
}

static FString BytesToHex(const TArray<uint8>& Bytes)
{
	FString Hex;
	Hex.Reserve(Bytes.Num() * 2);
	for (uint8 B : Bytes)
	{
		Hex += FString::Printf(TEXT("%02x"), B);
	}
	return Hex;
}

bool UHazeKeyPair::IsSigningAvailable()
{
#if defined(HAZE_HAS_ED25519) && HAZE_HAS_ED25519
	return true;
#else
	return false;
#endif
}

UHazeKeyPair* UHazeKeyPair::Generate()
{
	UHazeKeyPair* K = NewObject<UHazeKeyPair>();
	K->PrivateKey.SetNum(32);
	// Fill with secure random
	FPlatformMisc::GenRandom(K->PrivateKey.GetData(), 32);

#if defined(HAZE_HAS_ED25519) && HAZE_HAS_ED25519
	uint8 PublicKey[32];
	uint8 PrivateKeyExpanded[64];
	ed25519_seed_keypair(K->PrivateKey.GetData(), PublicKey, PrivateKeyExpanded);
	K->PublicKey.SetNum(32);
	FMemory::Memcpy(K->PublicKey.GetData(), PublicKey, 32);
	// Keep only 32-byte seed in PrivateKey (our API); ed25519 uses 64-byte expanded internally but we restore from 32-byte seed)
#else
	// Stub: no Ed25519; use hash of seed as fake "address" for display only
	K->PublicKey.SetNum(32);
	uint8 Hash[32];
	FMemory::Memzero(Hash, 32);
	FSHA1::HashBuffer(K->PrivateKey.GetData(), 32, Hash); // writes 20 bytes
	FMemory::Memcpy(K->PublicKey.GetData(), Hash, 32);
#endif
	return K;
}

UHazeKeyPair* UHazeKeyPair::FromPrivateKeyHex(const FString& PrivateKeyHex)
{
	FString Hex = PrivateKeyHex.TrimStartAndEnd().Replace(TEXT(" "), TEXT(""));
	if (Hex.Len() != 64) return nullptr;
	TArray<uint8> Seed = HexToBytes(Hex);
	if (Seed.Num() != 32) return nullptr;

	UHazeKeyPair* K = NewObject<UHazeKeyPair>();
	K->PrivateKey = MoveTemp(Seed);

#if defined(HAZE_HAS_ED25519) && HAZE_HAS_ED25519
	uint8 PublicKey[32];
	uint8 PrivateKeyExpanded[64];
	ed25519_seed_keypair(K->PrivateKey.GetData(), PublicKey, PrivateKeyExpanded);
	K->PublicKey.SetNum(32);
	FMemory::Memcpy(K->PublicKey.GetData(), PublicKey, 32);
#else
	K->PublicKey.SetNum(32);
	uint8 Hash[32];
	FMemory::Memzero(Hash, 32);
	FSHA1::HashBuffer(K->PrivateKey.GetData(), 32, Hash);
	FMemory::Memcpy(K->PublicKey.GetData(), Hash, 32);
#endif
	return K;
}

FString UHazeKeyPair::GetAddressHex() const
{
	return BytesToHex(PublicKey);
}

TArray<uint8> UHazeKeyPair::Sign(const TArray<uint8>& Message) const
{
	TArray<uint8> Signature;
#if defined(HAZE_HAS_ED25519) && HAZE_HAS_ED25519
	if (PrivateKey.Num() != 32) return Signature;
	uint8 PublicKeyBuf[32];
	uint8 PrivateKeyExpanded[64];
	ed25519_seed_keypair(PrivateKey.GetData(), PublicKeyBuf, PrivateKeyExpanded);
	Signature.SetNum(64);
	ed25519_sign(Signature.GetData(), Message.GetData(), Message.Num(), PublicKeyBuf, PrivateKeyExpanded);
#else
	// Stub: return empty when Ed25519 not linked
	(void)Message;
#endif
	return Signature;
}
