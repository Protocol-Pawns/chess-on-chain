const { board } = props;
if (!board) return "board prop required";

const fieldSize = "3rem";
const Board = styled.div`
  display: flex;
  flex-direction: column;
  width: 100%;
`;
const BoardRow = styled.div`
  display: flex;
  justify-content: center;
  width: 100%;
`;
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

const renderPiece = (piece) => {
  switch (piece) {
    case " ":
      return "";
    case "p":
      return (
        <img
          alt="black pawn"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/p-black.svg"
        />
      );
    case "P":
      return (
        <img
          alt="white pawn"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/P-white.svg"
        />
      );
    case "n":
      return (
        <img
          alt="black knight"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/n-black.svg"
        />
      );
    case "N":
      return (
        <img
          alt="white knight"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/N-white.svg"
        />
      );
    case "b":
      return (
        <img
          alt="black bishop"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/b-black.svg"
        />
      );
    case "B":
      return (
        <img
          alt="white bishop"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/B-white.svg"
        />
      );
    case "r":
      return (
        <img
          alt="black rook"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/r-black.svg"
        />
      );
    case "R":
      return (
        <img
          alt="white rook"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/R-white.svg"
        />
      );
    case "q":
      return (
        <img
          alt="black queen"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/q-black.svg"
        />
      );
    case "Q":
      return (
        <img
          alt="white queen"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/Q-white.svg"
        />
      );
    case "k":
      return (
        <img
          alt="black king"
          src="https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/k-black.svg"
        />
      );
    case "K":
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

return <Board>{renderBoard(board)}</Board>;
