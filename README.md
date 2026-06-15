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

### Install dependencies

```sh
yarn install
```

### Smart contracts

Build all contracts:

```sh
./build.sh
```

Run Rust tests and checks:

```sh
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

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
