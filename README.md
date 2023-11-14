# Protocol Pawns

_Protocol Pawns_ is your very first turn based fully on chain chess game.

Try out the [dapp](https://near.org/chess-game.near/widget/ChessGameLobby)!

Features:

- play against an AI (3 difficulties)
- PvP
- ELO ranking, if [I-Am-Human](https://i-am-human.app/) verified

Upcoming features:

- gaming token backed by treasury
- money matches

## Build, deploy & migrate

```sh
# build
./build.sh

# deploy
near deploy app.chess-game.near ./res/chess.wasm

# migrate
near call app.chess-game.near migrate '' --accountId app.chess-game.near --gas 300000000000000
```

## Irys uploads

Install [Irys CLI](https://docs.irys.network/developer-docs/cli).

```sh
# fund account
bundlr -h http://node1.bundlr.network -w $NEAR_PRIVATE_KEY -t near fund 1000000000000000000000000

# check balance
bundlr -h http://node1.bundlr.network -w $NEAR_PRIVATE_KEY -t near balance chess-game.near

# upload folder
bundlr -h http://node1.bundlr.network -w $NEAR_PRIVATE_KEY -t near upload-dir ./assets/hk/
```

### Uploaded files

- https://6pvogaxcfxogppk7kexyngfi3uzdlay77zowch4gcaoswmrcyooa.arweave.net/8-rjAuIt3Ge9X1Evhpio3TI1gx_-XWEfhhAdKzIiw5w

## Credits

- Adam Mcdaniel for his [chess engine](https://github.com/adam-mcdaniel/chess-engine)
- atomflunder for his [ELO rating algorithm](https://github.com/atomflunder/skillratings/)
