export interface ContractAccountInfo {
  account_id: string;
  elo?: number | null;
  points: string;
  wins: number;
  win_streak: number;
  max_win_streak: number;
  bets_placed: number;
  bets_won: number;
  wagers_played: number;
  wager_wins: number;
  challenges_sent: number;
}

export async function getAccountInfo(
  rpcUrl: string,
  contractId: string,
  accountId: string
): Promise<ContractAccountInfo | null> {
  const args = btoa(JSON.stringify({ account_id: accountId }));
  const res = await fetch(rpcUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'query',
      params: {
        request_type: 'call_function',
        account_id: contractId,
        method_name: 'get_account',
        args_base64: args,
        finality: 'final'
      }
    })
  });

  const json = await res.json<{
    result?: { result: number[] };
    error?: { message: string };
  }>();

  if (json.error) {
    console.error(`RPC error fetching account info: ${json.error.message}`);
    return null;
  }

  const resultBytes = json.result?.result;
  if (!resultBytes || resultBytes.length === 0) return null;

  const decoded = new TextDecoder().decode(new Uint8Array(resultBytes));
  return JSON.parse(decoded) as ContractAccountInfo;
}
