---
name: protocol-pawns
description: Use when working on the Protocol Pawns chess-on-chain project — Rust smart contract on NEAR, SvelteKit app, Hono API/indexer, or agent integrations. Covers architecture, build steps, contract methods, and where to make changes.
---

# Protocol Pawns — Project Skill

## When to Use

Use this skill when you are editing, debugging, or extending the Protocol Pawns repository. This includes:

- Modifying the NEAR smart contract (`crates/chess-lib`, `crates/chess-engine`)
- Adding or changing contract methods, events, storage, or migrations
- Working on the SvelteKit frontend (`app/`)
- Working on the Hono REST API or SSE indexer (`api/`, `indexer/processor/`)
- Updating the public agent skill (`app/static/.well-known/ai/skill.md`)
- Building, testing, or deploying any part of the stack
- Integrating external agents or bots with the contract

## Project Overview

Protocol Pawns is a fully on-chain chess game on NEAR Protocol (mainnet). All game logic runs in the smart contract at `app.chess-game.near`.

### Repositories / Workspaces

| Path                  | Package / Crate | Purpose                                                                |
| --------------------- | --------------- | ---------------------------------------------------------------------- |
| `crates/chess-lib`    | `chess-lib`     | Main NEAR smart contract: state, methods, events, bets, wagers, points |
| `crates/chess-engine` | `chess-engine`  | On-chain chess engine: board, move generation, minimax AI              |
| `crates/chess-test`   | `chess-test`    | Integration tests using `near-workspaces`                              |
| `crates/chess-common` | `chess-common`  | Shared types between contract and engine                               |
| `crates/chess`        | `chess`         | Contract entry point that re-exports `chess-lib`                       |
| `app/`                | `app`           | SvelteKit frontend                                                     |
| `api/`                | `api`           | Hono REST API + SSE event streaming                                    |
| `indexer/processor/`  | `processor`     | NEAR indexer that writes events to Postgres                            |
| `landing/`            | `landing`       | Astro landing page                                                     |

### Important External Resources

- Mainnet contract: `app.chess-game.near`
- RPC: `https://rpc.shitzuapes.xyz`
- REST API: `https://api.protocol-pawns.com`
- App: `https://app.protocol-pawns.com`
- Public agent skill: `https://app.protocol-pawns.com/.well-known/ai/skill.md`

## Build & Test

### Smart Contract

```bash
# Build the contract and regenerate res/chess.wasm + res/chess_abi.json
./build.sh

# Run unit/integration tests
cargo test -p chess-lib
cargo test -p chess-engine
cargo test -p chess-test

# Regenerate TypeScript contract types from ABI
yarn gen-abi
```

### Frontend

```bash
yarn install
yarn app dev      # SvelteKit dev server
yarn app build    # Production build
```

### API

```bash
yarn api dev      # Hono dev server (Wrangler)
yarn api deploy   # Deploy via Wrangler
```

### Indexer

```bash
yarn processor dev    # Dev mode
yarn processor deploy # Deploy processor
```

## Contract Architecture

### Entry Point

`crates/chess/src/lib.rs` re-exports `chess-lib` and compiles to `res/chess.wasm`.

### Core Modules

| File                                  | Responsibility                                                                                                    |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `crates/chess-lib/src/lib.rs`         | `Chess` struct, public call methods (`play_move`, `challenge`, `create_ai_game`, `claim_points`, etc.), constants |
| `crates/chess-lib/src/view.rs`        | Public view methods (`get_account`, `game_info`, `bet_info`, etc.)                                                |
| `crates/chess-lib/src/game.rs`        | `GameId`, `Player`, `Difficulty`, `GameInfo`, `GameOutcome`, move execution, AI logic                             |
| `crates/chess-lib/src/account.rs`     | `Account` enum with migrations, `AccountInfo`, stats, game/challenge/bet tracking                                 |
| `crates/chess-lib/src/challenge.rs`   | `Challenge`, `ChallengeId`, wager validation                                                                      |
| `crates/chess-lib/src/bet.rs`         | `BetId`, `Bets`, `BetInfo`, bet placement/resolution logic                                                        |
| `crates/chess-lib/src/points.rs`      | `Quest`, `Achievement`, PPP token metadata, FT core impl                                                          |
| `crates/chess-lib/src/event.rs`       | NEP-297 event definitions (`ChessEvent`)                                                                          |
| `crates/chess-lib/src/ft_receiver.rs` | FT receiver for wager/bet token deposits                                                                          |
| `crates/chess-lib/src/storage.rs`     | Storage management (registration deposit = 0.05 NEAR)                                                             |
| `crates/chess-lib/src/internal.rs`    | Internal helpers for challenge acceptance, outcome handling, wager payouts                                        |
| `crates/chess-lib/src/elo.rs`         | ELO rating calculation                                                                                            |

### Key Types

- `GameId` = `[u64, AccountId, Option<AccountId>]` — `[block_height, white, black]`
- `Player` = `Human(AccountId)` | `Ai(Difficulty)`
- `Difficulty` = `Easy | Medium | Hard | VeryHard`
- `GameOutcome` = `Victory(Color)` | `Stalemate`
- `ChallengeId` = `String` formatted `"{challenger}-vs-{challenged}"`
- `BetId` = sorted pair of player account IDs

### State Migrations

The contract uses versioned enums (`Account::V9`, `Account::V10`; `Game::V4`, `Game::V5`). When adding new fields:

1. Add a new variant to the enum.
2. Implement `migrate()` from the previous variant.
3. Call migration logic from the contract's `migrate()` method.
4. Update `AccountInfo` / `GameInfo` if the public shape changes.

## Adding or Changing a Contract Method

1. **Implement the method** in `crates/chess-lib/src/lib.rs` (mutating) or `view.rs` (read-only).
2. **Use `#[handle_result]`** for methods returning `Result<_, ContractError>`.
3. **Emit an event** in `crates/chess-lib/src/event.rs` if agents, the indexer, or the API need to react.
4. **Update tests** in `crates/chess-test/tests/`.
5. **Rebuild ABI:** `./build.sh` then `yarn gen-abi`.
6. **Update the public agent skill** `app/static/.well-known/ai/skill.md` if the method is agent-facing.
7. **Update the API/indexer** if new event types or data shapes need to be indexed.

### Contract Method Patterns

View method:

```rust
#[handle_result]
pub fn my_view(&self, game_id: GameId) -> Result<MyType, ContractError> {
    let game = self.games.get(&game_id).ok_or(ContractError::GameNotExists)?;
    Ok(game.into())
}
```

Call method:

```rust
#[handle_result]
pub fn my_call(&mut self, game_id: GameId) -> Result<(), ContractError> {
    require!(self.is_running, "Contract is paused");
    let account_id = env::predecessor_account_id();
    // ... logic
    Ok(())
}
```

## Events

The contract emits NEP-297 events. The indexer (`indexer/processor/src/handlers.ts`) consumes these and writes to Postgres.

Event types:

- `challenge`
- `accept_challenge`
- `reject_challenge`
- `create_game`
- `play_move`
- `resign_game`
- `cancel_game`
- `place_bet`
- `cancel_bet`
- `lock_bets`
- `resolve_bets`

When adding a new event:

1. Add the variant to `ChessEvent` in `event.rs`.
2. Emit it where appropriate.
3. Add a handler in `indexer/processor/src/handlers.ts`.
4. Add a resolver in `api/src/events-stream.ts` if it needs SSE routing.

## Frontend / App

- SvelteKit with TypeScript.
- Wallet connection via `@hot-labs/near-connect`.
- Contract interactions in `app/src/lib/near/connector.ts`.
- REST API client in `app/src/lib/api/client.ts`.
- Real-time updates via SSE in `app/src/lib/sse.ts`.

When changing contract call signatures, update `contract` object in `connector.ts` and regenerate contract types with `yarn gen-abi`.

## REST API + SSE

The API (`api/src/index.ts`) exposes OpenAPI-documented endpoints and an SSE stream at `/events?account={id}`.

Key routes:

- `GET /games`, `/game/{id}`, `/game/{id}/moves`
- `GET /account/{id}`, `/account/{id}/stats`, `/account/{id}/active-game`
- `GET /leaderboard/elo`, `/leaderboard/ppp`, `/leaderboard/bets`
- `GET /challenges`, `/account/{id}/challenges`
- `GET /bets`, `/account/{id}/bets`, `/game/{id}/bets`

Schemas live in `api/src/events.ts` and routes in `api/src/routes.ts`.

## Agent Integrations

For external AI agents, point them to the public skill:

```
https://app.protocol-pawns.com/.well-known/ai/skill.md
```

Important agent-facing facts:

- Registration requires 0.05 NEAR via `storage_deposit`.
- Agents should call `set_is_agent(true)` with 1 yoctoNEAR to mark themselves as bots.
- AI difficulty has four levels: `Easy`, `Medium`, `Hard`, `VeryHard`.
- Recommended gas aligned with the app: 100 / 300 / 500 / 800 TGas respectively.
- Moves must be in coordinate notation (`e2e4`, `O-O`, `e7 to e8 Q`).
- Human games emit `play_move` events; agents should listen via SSE or poll the API.
- PPP points are non-transferable; claim pending points with `claim_points`.

## Common Tasks

### "I need to add a new AI difficulty"

1. Add variant to `Difficulty` in `crates/chess-lib/src/game.rs`.
2. Define gas budget and max depths.
3. Add achievement variants in `points.rs` if relevant.
4. Update the app `Difficulty` schema/type if needed.
5. Update `app/static/.well-known/ai/skill.md`.

### "I need to change storage deposit"

1. Update `STORAGE_ACCOUNT_COST` in `crates/chess-lib/src/storage.rs`.
2. Update frontend constant if any.
3. Update `app/static/.well-known/ai/skill.md` and `app/static/llms.txt`.

### "I need to add a new contract method"

See "Adding or Changing a Contract Method" above.

### "I need to verify a deployed method"

Use `near-cli-rs`:

```bash
near contract call-function as-read-only app.chess-game.near \
  get_account json-args '{"account_id":"agent.near"}' \
  network-config mainnet now
```

Or inspect `res/chess_abi.json`.

## Deployment

```bash
# Deploy (owner only)
near deploy app.chess-game.near ./res/chess.wasm

# Migrate state (owner only)
near call app.chess-game.near migrate '' --accountId app.chess-game.near --gas 300000000000000
```

## Useful Files

- `Cargo.toml` — workspace members and dependencies
- `package.json` — yarn workspaces and scripts
- `build.sh` — contract build script
- `abi-gen.sh` — ABI generation
- `res/chess_abi.json` — generated ABI
- `app/src/lib/near/contract-types.d.ts` — generated TypeScript types
- `app/static/.well-known/ai/skill.md` — public agent skill
- `app/static/llms.txt` — short LLM overview
