# Protocol Pawns

Protocol Pawns is a fully on-chain chess game built on [NEAR Protocol](https://near.org/). Every move, game outcome, challenge, wager, and ELO update is settled and stored by the smart contract — no off-chain game state required.

## Try it

- **Landing page:** https://protocol-pawns.com
- **Play the game:** https://app.protocol-pawns.com
- **Smart contract:** `app.chess-game.near` on NEAR mainnet
- **Telegram:** https://t.me/protocolpawns

## Features

- **Play vs AI** — three difficulty levels (Easy, Medium, Hard) with the AI executing moves on-chain.
- **PvP matches** — challenge other players or accept open challenges.
- **ELO ranking** — on-chain player ratings, gated by [I-Am-Human](https://i-am-human.app/) verification.
- **Spectator betting** — place wagers on live games and have payouts resolved by the contract.
- **Leaderboard** — global rankings backed by the API and on-chain events.
- **AI-agent ready** — autonomous agents can register, play, challenge, bet, and manage games via the published skill guide at `/.well-known/ai/skill.md`.

## Architecture

Protocol Pawns is a monorepo managed with [Yarn 4 workspaces](https://yarnpkg.com/).

| Layer           | Technology                                                                                                     | Purpose                                                             |
| --------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Smart contracts | Rust + [near-sdk](https://github.com/near/near-sdk-rs) 5.27 + [cargo-near](https://github.com/near/cargo-near) | On-chain chess engine, game state, rankings, and wagers.            |
| App             | [SvelteKit 5](https://svelte.dev/) + [UnoCSS](https://unocss.dev/) + Cloudflare Pages                          | Main player interface at `app.protocol-pawns.com`.                  |
| Landing         | [Astro 5](https://astro.build/) + [Tailwind CSS 4](https://tailwindcss.com/) + Cloudflare Pages                | Marketing site at `protocol-pawns.com`.                             |
| API             | [Hono](https://hono.dev/) + Zod OpenAPI + Cloudflare Workers + Postgres                                        | Leaderboards, game metadata, push notifications, and SSE updates.   |
| Indexer         | NEAR QueryAPI / Lake pipeline + Node processor + Postgres                                                      | Indexes contract events into Postgres for the API and leaderboards. |

## Project structure

```
.
├── crates/                 # Rust workspace
│   ├── chess/              # Main NEAR smart contract
│   ├── chess-lib/          # Shared contract logic
│   ├── chess-engine/       # Chess move validation / engine
│   ├── chess-common/       # Shared types and helpers
│   ├── chess-test/         # Integration tests
│   ├── test-token/         # Test fungible token contract
│   └── download-mainnet-state/ # Utility for downloading mainnet state
├── app/                    # SvelteKit player app
├── landing/                # Astro marketing site
├── api/                    # Hono API on Cloudflare Workers
├── indexer/                # QueryAPI pipeline and event processor
├── res/                    # Compiled WASM outputs
└── build.sh                # Contract build script
```

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Yarn](https://yarnpkg.com/) 4 (already configured via `.yarnrc.yml`)
- [Rust](https://www.rust-lang.org/) toolchain
- [cargo-near](https://github.com/near/cargo-near)
- Python 3 (for data generation scripts)

### Install dependencies

```sh
yarn install
```

### Smart contracts

Build all contracts. By default this **skips** data generation and uses the committed static data:

```sh
./build.sh
```

To regenerate the opening book and Zobrist keys from scratch (downloads Stockfish, takes ~20-30 min):

```sh
./build.sh --regen-data
```

The build also runs `wasm-opt -Oz` if available for smaller binaries.

Run Rust tests and checks:

```sh
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

### Data Generation

Several static data files are compiled into the WASM binary. They are **committed** to the repo and only need regeneration when opening theory or hash keys change.

| Script                            | Output                                    | Purpose                                                                                                                                                                                                                                                                             |
| --------------------------------- | ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `scripts/setup.sh`                | `scripts/.pydeps/`                        | Downloads python-chess and Stockfish (prerequisite for other scripts)                                                                                                                                                                                                               |
| `scripts/generate_zobrist.py`     | `crates/chess-engine/src/zobrist_keys.rs` | Precomputed Zobrist hash keys (781 random `u64` values with deterministic seed)                                                                                                                                                                                                     |
| `scripts/generate_static_data.py` | `crates/chess-engine/src/static_book.rs`  | Opening book — two-phase generation: **Phase 1** walks ~240 hand-curated opening lines at Stockfish depth 18; **Phase 2** tree-expands every position with multiPV=2 to cover opponent deviations. Produces **2,700+ `(zobrist_key, encoded_move)` pairs** sorted for binary search |

**Manual regeneration (without build.sh):**

```sh
# 1. Fetch python-chess and Stockfish (one-time)
bash scripts/setup.sh

# 2. Generate Zobrist keys
PYTHONPATH=scripts/.pydeps/chess-1.11.2 python3 scripts/generate_zobrist.py

# 3. Generate opening book (~20 min)
PYTHONPATH=scripts/.pydeps/chess-1.11.2 python3 scripts/generate_static_data.py
```

The Zobrist keys use a deterministic random seed (`0x70726F746F636F6C207061776E73` = "protocol pawns") so they are reproducible across runs. To expand the opening book, add more UCI move sequences to the `LINES` list in `generate_static_data.py` — the tree expansion phase automatically adds coverage for opponent deviations.

### App

```sh
yarn app dev
```

### Landing page

```sh
yarn landing dev
```

### API

```sh
yarn api start
```

### Indexer processor

```sh
yarn processor start
```

### Lint and format

```sh
yarn lint
yarn format
```

## Credits

- [Adam McDaniel](https://github.com/adam-mcdaniel) for the original [chess engine](https://github.com/adam-mcdaniel/chess-engine).
- [atomflunder](https://github.com/atomflunder) for the [skillratings](https://github.com/atomflunder/skillratings/) ELO implementation.

## License

MIT — see [LICENSE](./LICENSE).
