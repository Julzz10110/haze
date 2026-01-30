# Security

Brief description of the signing model and security recommendations for HAZE.

## Signing model

- **Who signs:** For all user-initiated transactions, the signer is the **`from`** field (key owner and fee payer).
- **Format:** Ed25519. In HAZE, the address is the 32-byte Ed25519 public key. The signature is 64 bytes.
- **What is signed:** The canonical transaction payload (without the `signature` field), serialized to binary. The format must match between the node (Rust) and the SDK (TypeScript): see `ConsensusEngine::get_transaction_data_for_signing` and the SDK’s `getTransactionDataForSigning`.

### Where signatures are verified

- **Consensus (node):** When a transaction is added to the mempool, `validate_transaction` is called; inside it, `verify_transaction_signature` is used for each transaction type:
  - **Transfer** — signature from `from`
  - **Stake** — signature from `from` (the staker)
  - **ContractCall** — signature from `from`
  - **MistbornAsset** — signature from `from` (and must match `data.owner`)
  - **SetAssetPermissions** — signature from `from` (in code, `owner`)

Signatures are not verified again when applying a block; transactions are assumed already validated by consensus.

## Recommendations

- **Do not log:** Raw signature bytes, private keys, or full transaction objects including the signature. In logs and errors, use only the transaction hash or masked identifiers.
- **Do not expose in API errors:** Internal details of signatures or keys. In `ApiResponse::error`, return only a message suitable for the client.
- **SDK:** Do not log `privateKey` or the full signature in production; do not commit real keys in tests or examples.