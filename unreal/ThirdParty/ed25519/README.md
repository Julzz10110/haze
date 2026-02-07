# Ed25519 for HAZE signing

To enable transaction signing in the HAZE Unreal plugin, provide an Ed25519 implementation here.

## Required API (C linkage)

```c
void ed25519_seed_keypair(const uint8_t seed[32], uint8_t public_key[32], uint8_t private_key[64]);
void ed25519_sign(uint8_t signature[64], const uint8_t* message, size_t message_len,
                  const uint8_t public_key[32], const uint8_t private_key[64]);
```

- **seed:** 32-byte secret; the plugin generates this randomly for new keys.
- **public_key:** 32-byte Ed25519 public key (= HAZE address).
- **private_key:** 64-byte expanded secret used by `ed25519_sign` (output of seed_keypair).
- **signature:** 64-byte Ed25519 signature.

## Option 1: libsodium

1. Build [libsodium](https://github.com/jedisct1/libsodium) for your platform.
2. Copy `sodium.h` and the static library (e.g. `sodium.lib` on Windows, `libsodium.a` elsewhere) into this folder.
3. In `HazeBlockchain.Build.cs`, add the include path and library, and in `KeyPair.cpp` call `crypto_sign_seed_keypair` and `crypto_sign_detached` instead of the above (adapt types as needed). Define `HAZE_HAS_ED25519=1`.

## Option 2: ed25519-donna

1. Get [ed25519-donna](https://github.com/floodyberry/ed25519-donna) (or a compatible implementation that provides the two functions above).
2. Add `ed25519.h` and `ed25519.c` (or the amalgamation) to this folder.
3. In `HazeBlockchain.Build.cs`, add this folder to include paths and add the compiled object or static lib; define `HAZE_HAS_ED25519=1`.

Until an implementation is added, the plugin compiles and runs but **Sign** returns empty and **BuildSignedTransfer** / **BuildSignedMistbornCreate** return an empty string.
