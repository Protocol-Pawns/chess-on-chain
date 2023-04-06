const { board } = props;
if (!board) return "board prop required";

const assetType = props.assetType || "default";

const assets = {
  default: {
    bp: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/p-black.svg",
    wp: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/P-white.svg",
    bn: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/n-black.svg",
    wn: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/N-white.svg",
    bb: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/b-black.svg",
    wb: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/B-white.svg",
    br: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/r-black.svg",
    wr: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/R-white.svg",
    bq: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/q-black.svg",
    wq: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/Q-white.svg",
    bk: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/k-black.svg",
    wk: "https://raw.githubusercontent.com/nikfrank/react-chess-pieces/master/src/K-white.svg",
  },
  hk: {
    bp: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-bp.png",
    wp: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-wp.png",
    bn: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-bn.png",
    wn: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-wn.png",
    bb: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-bb.png",
    wb: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-wb.png",
    br: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-br.png",
    wr: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-wr.png",
    bq: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-bq.png",
    wq: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-wq.png",
    bk: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-bk.png",
    wk: "https://arweave.net/OqbQKrKHNPuIqJ-hsuzRpu-JNI4S04Zbb9qJW2j7RQQ/chess-hk-wk.png",
  },
};

const fieldSize = "4rem";
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
      return <img alt="black pawn" src={assets[assetType].bp} />;
    case "P":
      return <img alt="white pawn" src={assets[assetType].wp} />;
    case "n":
      return <img alt="black knight" src={assets[assetType].bn} />;
    case "N":
      return <img alt="white knight" src={assets[assetType].wn} />;
    case "b":
      return <img alt="black bishop" src={assets[assetType].bb} />;
    case "B":
      return <img alt="white bishop" src={assets[assetType].wb} />;
    case "r":
      return <img alt="black rook" src={assets[assetType].br} />;
    case "R":
      return <img alt="white rook" src={assets[assetType].wr} />;
    case "q":
      return <img alt="black queen" src={assets[assetType].bq} />;
    case "Q":
      return <img alt="white queen" src={assets[assetType].wq} />;
    case "k":
      return <img alt="black king" src={assets[assetType].bk} />;
    case "K":
      return <img alt="white king" src={assets[assetType].wk} />;
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
        position: relative;

        img {
          min-width: 100%;
          min-height: 100%;
          max-width: 100%;
          position: absolute;
          bottom: 0;
          left: 0;
          right: 0;
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
