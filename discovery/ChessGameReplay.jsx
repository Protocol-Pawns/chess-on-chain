const { gameId } = props;
const accountId = gameId[1];
const gameIdStr = JSON.stringify(gameId);
const contractId = "app.chess-game.near";
if (!accountId) {
  return "You need to login with your Near wallet first!";
}

const fetchOptions = {
  headers: {
    "x-api-key": "36f2b87a-7ee6-40d8-80b9-5e68e587a5b5",
  },
};
let events = [];
let offset = 0;
while (true) {
  const res = fetch(
    `https://api.pikespeak.ai/account/transactions/${contractId}?offset=${offset}`,
    fetchOptions
  );
  offset += 50;
  if (!res.ok) {
    return `Pikespeak API returned error: ${JSON.stringify(res)}`;
  }
  const newEvents = res.body
    .filter(({ signer }) => signer === accountId)
    .flatMap(({ id }) => {
      const res = fetch(
        `https://api.pikespeak.ai/tx/graph-by-hash/${id}`,
        fetchOptions
      );
      const { logs } = res.body[0].transaction_graph.eoNode.childs[0].content;
      return logs
        .filter((log) => log.startsWith("EVENT_JSON:"))
        .map((log) => JSON.parse(log.substr(11)))
        .filter(({ data }) => JSON.stringify(data.game_id) == gameIdStr);
    });
  if (newEvents.length > 0) {
    events = events.concat(newEvents);
  }
  if (res.body.length < 50) break;
}
events.reverse();
console.log("events", events);

return "";
