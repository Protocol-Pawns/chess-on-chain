const { game_id } = props;
const contractId = "app.chess-game.near";
if (!game_id) return <div>"game_id" missing in props</div>;

const board = Near.view(contractId, "get_board", {
  game_id,
});
if (!board) return <div />;

const gameInfo = Near.view(contractId, "game_info", {
  game_id,
});
if (!gameInfo) return <div />;

State.init({
  board,
  gameInfo,
  move: "",
});

const BoardView = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
`;
const GameInfo = styled.div`
  display: flex;
  flex-direction: column;
  justify-content: center;
  font-size: 1.4rem;
  margin: 1rem;
`;
const Board = styled.div`
  display: flex;
  flex-direction: column;
  width: 100%;
`;
const BoardRow = styled.div`
  display: flex;
  width: 100%;
`;

const renderPlayer = (color, player) => {
  if (player.Human) {
    return (
      <div>
        Player {color}: {player.Human}
      </div>
    );
  } else if (player.Ai) {
    return (
      <div>
        Player {color}: AI ({player.Ai})
      </div>
    );
  } else {
    const err = new Error(`Unable to render player: ${player}`);
    console.error(err);
    return "";
  }
};

const renderPiece = (piece) => {
  switch (piece) {
    case " ":
      return "";
    case "♟":
      return (
        <img
          alt="black pawn"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/p-black.svg"
        />
      );
    case "♙":
      return (
        <img
          alt="white pawn"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/P-white.svg"
        />
      );
    case "♞":
      return (
        <img
          alt="black knight"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/n-black.svg"
        />
      );
    case "♘":
      return (
        <img
          alt="white knight"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/N-white.svg"
        />
      );
    case "♝":
      return (
        <img
          alt="black bishop"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/b-black.svg"
        />
      );
    case "♗":
      return (
        <img
          alt="white bishop"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/B-white.svg"
        />
      );
    case "♜":
      return (
        <img
          alt="black rook"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/r-black.svg"
        />
      );
    case "♖":
      return (
        <img
          alt="white rook"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/R-white.svg"
        />
      );
    case "♛":
      return (
        <img
          alt="black queen"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/q-black.svg"
        />
      );
    case "♕":
      return (
        <img
          alt="white queen"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/Q-white.svg"
        />
      );
    case "♚":
      return (
        <img
          alt="black king"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/k-black.svg"
        />
      );
    case "♔":
      return (
        <img
          alt="white king"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/K-white.svg"
        />
      );
    default:
      return "";
  }
};

const fieldSize = "3rem";
const Legend = styled.div`
  flex: 1 1 0;
  max-width: ${fieldSize};
  aspect-ratio: 1 / 1;
  font-size: 1.6rem;
  font-weight: 600;
  display: flex;
  justify-content: center;
  align-items: center;
`;
const renderBoard = (board) => {
  const boardRes = board.reverse().map((row, rowIndex) => {
    const res = row.split("").map((c, colIndex) => {
      const background = (rowIndex + colIndex) % 2 === 0 ? "#ddd" : "#555";
      const Field = styled.span`
        flex: 1 1 auto;
        max-width: ${fieldSize};
        aspect-ratio: 1 / 1;
        background: ${background};

        img {
          min-width: 100%;
          min-height: 100%;
          max-width: 100%;
          max-height: 100%;
        }
        `;
      return <Field>{renderPiece(c)}</Field>;
    });
    res.unshift(<Legend>{8 - rowIndex}</Legend>);
    return <BoardRow>{res}</BoardRow>;
  });
  boardRes.push(
    <BoardRow>
      <Legend></Legend>
      <Legend>A</Legend>
      <Legend>B</Legend>
      <Legend>C</Legend>
      <Legend>D</Legend>
      <Legend>E</Legend>
      <Legend>F</Legend>
      <Legend>G</Legend>
      <Legend>H</Legend>
    </BoardRow>
  );
  return boardRes;
};

const TurnInput = styled.input`
  border-radius: 4px;
  border: 1px solid black;
`;
const SendButton = styled.button`
  border-radius: 4px;
`;

const updateMove = (event) => {
  State.update({
    move: event.target.value,
  });
};

const playMove = () => {
  if (!state.move) return;
  Near.call(
    contractId,
    "play_move",
    {
      game_id,
      mv: state.move,
    },
    "300000000000000"
  );
};

const Footer = styled.div`
  display: flex;
  flex-direction: column;
`;

const text = `
  A valid move will be parsed from a string.
  
  Possible valid formats include:
  - \"e2e4\"
  - \"e2 e4\"
  - \"e2 to e4\"
  - \"castle queenside\"
  - \"castle kingside\"'
`;

return (
  <BoardView>
    <GameInfo>
      <div>ID: {game_id[0]}</div>
      {renderPlayer("White", state.gameInfo.white)}
      {renderPlayer("Black", state.gameInfo.black)}
      <div>Turn: {state.gameInfo.turn_color}</div>
    </GameInfo>
    <Board>{renderBoard(state.board)}</Board>
    <Footer>
      <h3>Your Move:</h3>
      <div>
        <TurnInput
          type="text"
          required
          id="turn"
          value={state.move}
          onChange={updateMove}
        />
        <SendButton onClick={playMove}>Play</SendButton>
      </div>
      <Markdown text={text} />
    </Footer>
  </BoardView>
);
