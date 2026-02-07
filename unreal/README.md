# HAZE Blockchain Plugin for Unreal Engine

C++ plugin for HAZE blockchain: HTTP client, Ed25519 signing, transaction build/send. Same REST and transaction contract as the node and [Unity SDK](../unity/README.md).

## Requirements

- Unreal Engine 5.x (5.2+ recommended)
- HAZE node running (e.g. `cargo run` in the repo root)

## Installation

1. Copy the `unreal` folder into your project's `Plugins` directory, or add as a Git submodule:
   ```
   Plugins/
     HazeBlockchain/   <- contents of unreal/
         HazeBlockchain.uplugin
         Source/
         README.md (this file at unreal/README.md)
   ```
2. In Unreal Editor: **Edit → Plugins**, find **HAZE Blockchain**, enable it, restart if prompted.
3. In your module's `Build.cs`, add `"HazeBlockchain"` to `PublicDependencyModuleNames` or `PrivateDependencyModuleNames` if you use it from C++.

## Configuration

Set the HAZE node base URL when creating the client:

- **Blueprint:** Use **Create Haze Client** with base URL (e.g. `http://localhost:8080`).
- **C++:** `UHazeClient::CreateClient(TEXT("http://localhost:8080"))`.

## Quick Start (Blueprint)

1. Create a Haze Client: **Create Haze Client** (base URL `http://localhost:8080`).
2. **Get Health** or **Get Balance** (address as 64-char hex) – connect to the node.
3. **Generate Key Pair** (or **Restore Key Pair from Hex**) – get an address with **Get Address Hex**.
4. **Get Balance** with that address.
5. To send a transfer you need a signed transaction: use **Transaction Builder** from C++ or a Blueprint node that builds the JSON (see below). **Send Transaction** accepts the inner transaction JSON (e.g. `{"Transfer":{...}}`).

## C++ Usage

```cpp
#include "HazeClient.h"
#include "KeyPair.h"
#include "TransactionBuilder.h"

// Create client and key
UHazeClient* Client = UHazeClient::CreateClient(TEXT("http://localhost:8080"));
UHazeKeyPair* Key = UHazeKeyPair::Generate();
FString Address = Key->GetAddressHex();

// Get balance (async)
Client->GetBalance(Address, FHazeBalanceDelegate::CreateLambda([](const FString& Balance) {
    UE_LOG(LogTemp, Log, TEXT("Balance: %s"), *Balance);
}));

// Build and send transfer (requires Ed25519 linked – see Ed25519 section)
if (UHazeKeyPair::IsSigningAvailable())
{
    FString TxJson = FTransactionBuilder::BuildSignedTransfer(Key, ToAddressHex, Amount, Fee, Nonce);
    Client->SendTransaction(TxJson, FHazeTransactionDelegate::CreateLambda([](bool bOk, const FTransactionResponse& R) {
        if (bOk) UE_LOG(LogTemp, Log, TEXT("Tx hash: %s"), *R.Hash);
    }));
}
```

## Ed25519 (signing)

The plugin uses the same canonical payload and Ed25519 as the node. To **enable signing** you must link an Ed25519 implementation:

1. **Option A – ThirdParty lib:** Build or obtain a static library that exposes:
   - `void ed25519_seed_keypair(const uint8_t seed[32], uint8_t public_key[32], uint8_t private_key[64]);`
   - `void ed25519_sign(uint8_t signature[64], const uint8_t* message, size_t message_len, const uint8_t public_key[32], const uint8_t private_key[64]);`
   Place the library and header in `Plugins/HazeBlockchain/ThirdParty/ed25519/` and define `HAZE_HAS_ED25519=1` in Build.cs when the lib is present. See [ThirdParty/ed25519/README.md](ThirdParty/ed25519/README.md) (if present).

2. **Option B – libsodium:** Use libsodium’s `crypto_sign_*` API and adapt `KeyPair.cpp` to call it (e.g. `crypto_sign_seed_keypair`, `crypto_sign_detached`). Then define `HAZE_HAS_ED25519=1` and link libsodium.

Without an Ed25519 library, **Generate Key Pair** and **Restore Key Pair from Hex** still work (address is derived for display), but **Sign** returns an empty array and **BuildSignedTransfer** / **BuildSignedMistbornCreate** will return an empty string. Get Health, Get Balance, Get Account, and Send Transaction (with externally built JSON) work without signing.

## API coverage (5.1)

- **Client:** Health, Blockchain Info, Account, Balance, Send Transaction.
- **KeyPair:** Generate, FromPrivateKeyHex, GetAddressHex, Sign (when Ed25519 linked).
- **TransactionBuilder:** BuildSignedTransfer, BuildSignedMistbornCreate (when Ed25519 linked).

Mistborn (high-level) and Economy (pools, swap quote) are planned for later milestones; you can call the same REST endpoints from C++ or Blueprint in the meantime.

## References

- [API_TRANSACTIONS.md](../docs/API_TRANSACTIONS.md) – transaction format and signing
- [Unity SDK](../unity/README.md) – same API contract
