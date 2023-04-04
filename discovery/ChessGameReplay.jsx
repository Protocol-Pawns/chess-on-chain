const { game_id } = props;
const accountId = game_id[1];
const gameIdStr = JSON.stringify(game_id);
const contractId = "app.chess-game.near";
const chessBoardWidget = "chess-game.near/widget/ChessBoard";
const waitTime = 50;
const waitTimeOnErr = 500;

if (!accountId) {
  return "Malformed game_id prop!";
}

const BoardView = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  max-width: 100vw;
`;
const LoadingWrapper = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  margin: 3rem 0;
`;
const Loading = styled.div`
  width: 80px;
  height: 80px;
  border-radius: 50%;
  border: 7px solid transparent;
  border-top-color: rgba(0, 0, 0, 0.6);
  animation: rotate 800ms linear infinite;

  @keyframes rotate {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }
`;

const fetchOptions = {
  headers: {
    "x-api-key": "36f2b87a-7ee6-40d8-80b9-5e68e587a5b5",
  },
};

let transactions = state?.transactions ?? [];
if (!state.transactions) {
  let offset = 0;
  while (true) {
    const res = fetch(
      `https://api.pikespeak.ai/event-historic/${contractId}?offset=${offset}&contractFilter=${accountId}&filters=FUNCTION_CALL`,
      fetchOptions
    );
    offset += 50;
    if (!res.ok) {
      return `Pikespeak API returned error: ${JSON.stringify(res)}`;
    }

    if (res.body.length === 0) break;
    transactions = transactions.concat(res.body);
    if (res.body.length < 50) break;
  }
}

let events = state?.events ?? [];
State.init({
  transactions,
  events,
  tabIndex: 0,
  errCount: state?.errCount ?? 0,
});

if (transactions.length > 0) {
  const tx = transactions.pop();

  asyncFetch(
    `https://api.pikespeak.ai/tx/graph-by-hash/${tx.transaction_id}`,
    fetchOptions
  ).then(({ ok, body }) => {
    if (!ok) {
      transactions.push(tx);
      setTimeout(() => {
        State.update({
          errCount: state.errCount + 1,
        });
      }, waitTimeOnErr);
      return;
    }
    const { logs } = body[0].transaction_graph.eoNode.childs[0].content;
    const newEvents = logs
      .filter((log) => log.startsWith("EVENT_JSON:"))
      .map((log) => JSON.parse(log.substr(11)))
      .filter(({ data }) => JSON.stringify(data.game_id) == gameIdStr);
    if (newEvents.length > 0) {
      State.update({
        transactions,
        events: events.concat(newEvents),
      });
      return;
    }

    setTimeout(() => {
      State.update({
        transactions,
      });
    }, waitTime);
  });
  return (
    <LoadingWrapper>
      <div>Scanning transactions. Remaining: {transactions.length}</div>
      <Loading />
    </LoadingWrapper>
  );
}

const GameInfo = styled.div`
  display: flex;
  flex-direction: column;
  justify-content: center;
  font-size: 1.4rem;
  margin: 1rem 0.3rem;
  width: 350px;
  max-width: calc(100% - 0.6rem);
`;
const Button = styled.button`
  display: flex;
  flex-direction: column;
  border: 1px solid black;
  border-radius: 4px;
  visibility: ${(props) => (props.invisible ? "hidden" : "visible")};
`;
const ButtonWrapper = styled.div`
  display: flex;
  justify-content: space-around;
`;
const HorizontalLine = styled.div`
  width: 100%;
  border: 1px solid black;
  margin: 1rem 0;
`;
const Move = styled.div`
  text-align: center;
  visibility: ${(props) => (props.invisible ? "hidden" : "visible")};
`;
const Outcome = styled.div`
  display: flex;
  justify-content: center;
  margin-top: 1rem;
  font-weight: 600;
  font-size: 1.8rem;
  visibility: ${(props) => (props.invisible ? "hidden" : "visible")};
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
const renderMove = (move, label) => (
  <Move invisible={!move}>
    {label}: {move && move.color + " " + move.mv}
  </Move>
);
const renderOutcome = (outcome) => (
  <Outcome invisible={!outcome}>
    {outcome
      ? outcome.Victory
        ? `Victory: ${outcome.Victory}`
        : outcome
      : "placeholder"}
  </Outcome>
);
const setTabIndex = (index) => () => {
  State.update({
    tabIndex: index,
  });
};

const prevMove = state.events[state.tabIndex - 1]?.data;
const nextMove = state.events[state.tabIndex + 1]?.data;
const boardState = state.events[state.tabIndex].data;
if (!boardState.board) {
  return (
    <BoardView>
      Unable to render board. It looks like this game has been created with an
      older version of the contract and it's incompatible with replay rendering.
    </BoardView>
  );
}

return (
  <BoardView>
    <GameInfo>
      <div>ID: {game_id[0]}</div>
      {renderPlayer("White", state.events[0].data.white)}
      {renderPlayer("Black", state.events[0].data.black)}
    </GameInfo>
    <HorizontalLine />
    <GameInfo>
      {renderMove(prevMove, "Previous Move")}
      {renderMove(nextMove, "Next Move")}
      <ButtonWrapper>
        <Button invisible={!prevMove} onClick={setTabIndex(state.tabIndex - 2)}>
          ⇦
        </Button>
        <Button invisible={!nextMove} onClick={setTabIndex(state.tabIndex + 2)}>
          ⇨
        </Button>
      </ButtonWrapper>
      {renderOutcome(boardState.outcome)}
    </GameInfo>
    <Widget src={chessBoardWidget} props={{ board: boardState.board }} />
  </BoardView>
);
