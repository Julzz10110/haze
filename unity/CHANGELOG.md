# Changelog

## [0.1.0] - 2026-02-05

### Added
- Initial release of HAZE Unity SDK
- Core HTTP client (`HazeClient`) for all REST API endpoints
- Ed25519 key pair generation and signing (`KeyPair`)
- Canonical transaction payload signing (matches Rust/TypeScript)
- Transaction builders for Transfer, Stake, and MistbornAsset operations
- Support for all Mistborn operations: Create, Update, Condense, Evaporate, Merge, Split
- Asset search by owner and game_id
- Economy: liquidity pools (get, create)
- Basic usage example sample

### Dependencies
- Chaos.NaCl for Ed25519 cryptography
- Newtonsoft.Json for JSON serialization
