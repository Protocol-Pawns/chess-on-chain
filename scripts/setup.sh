#!/bin/bash
# Fetch external dependencies needed to generate static chess data.
set -e
cd "$(dirname "$0")"

mkdir -p .pydeps

# python-chess source
if [ ! -d ".pydeps/chess-1.11.2" ]; then
    echo "Downloading python-chess..."
    curl -sL https://files.pythonhosted.org/packages/source/c/chess/chess-1.11.2.tar.gz -o .pydeps/chess.tar.gz
    tar -xzf .pydeps/chess.tar.gz -C .pydeps
    rm .pydeps/chess.tar.gz
fi

# Stockfish binary (Ubuntu x86-64, portable baseline build)
if [ ! -f ".pydeps/stockfish/stockfish-ubuntu-x86-64" ]; then
    echo "Downloading Stockfish..."
    mkdir -p .pydeps/stockfish
    curl -sL -o .pydeps/stockfish.tar https://github.com/official-stockfish/Stockfish/releases/download/sf_18/stockfish-ubuntu-x86-64.tar
    tar -xf .pydeps/stockfish.tar -C .pydeps/stockfish --strip-components=1
    rm .pydeps/stockfish.tar
fi

echo "Static data dependencies ready."
