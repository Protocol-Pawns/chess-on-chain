# Chess on chain

## Build, deploy & migrate

```sh
# build
./build.sh

# deploy
near deploy app.chess-game.near ./res/chess.wasm

# migrate
near call app.chess-game.near migrate '' --accountId app.chess-game.near --gas 300000000000000
```

## Bundlr uploads

Install [Bundlr CLI](https://docs.bundlr.network/developer-docs/cli).

```sh
# fund account
bundlr -h http://node1.bundlr.network -w $NEAR_PRIVATE_KEY -c near fund 1000000000000000000000000

# check balance
bundlr -h http://node1.bundlr.network -w $NEAR_PRIVATE_KEY -c near balance chess-game.near

# upload folder
bundlr -h http://node1.bundlr.network -w $NEAR_PRIVATE_KEY -c near upload-dir ./assets/hk/
```
