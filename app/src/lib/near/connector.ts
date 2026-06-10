import { NearConnector } from '@hot-labs/near-connect';
import { JsonRpcProvider, Account, KeyPairSigner, actions } from 'near-api-js';

const NETWORK = import.meta.env.VITE_NETWORK_ID || 'mainnet';
const CONTRACT_ID = import.meta.env.VITE_CONTRACT_ID || 'app.chess-game.near';
const RPC_URL = import.meta.env.VITE_RPC_URL || 'https://rpc.shitzuapes.xyz';
const GAS = BigInt('30000000000000');

let connector: NearConnector | undefined;
let provider: JsonRpcProvider;

export function getConnector(): NearConnector {
  if (!connector) {
    connector = new NearConnector({
      network: NETWORK as 'mainnet' | 'testnet',
      features: {
        signAndSendTransaction: true,
        signAndSendTransactions: true
      }
    });
  }
  return connector;
}

export function getProvider(): JsonRpcProvider {
  if (!provider) {
    provider = new JsonRpcProvider({ url: RPC_URL });
  }
  return provider;
}

export function getContractId(): string {
  return CONTRACT_ID;
}

export async function viewFunction<T = unknown>(
  methodName: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  const p = getProvider();
  const result = await p.callFunction({
    contractId: CONTRACT_ID,
    method: methodName,
    args
  });
  return result as T;
}

export async function getTxLogs(txHash: string): Promise<string[]> {
  const p = getProvider();
  const wallet = await (await getConnector().wallet()).getAccounts();
  const accountId = wallet[0]?.accountId;
  const status = await p.sendJsonRpc('EXPERIMENTAL_tx_status', [
    txHash,
    accountId
  ]);
  const logs: string[] = [];
  if (status.receipts_outcome) {
    for (const receipt of status.receipts_outcome) {
      logs.push(...(receipt.outcome?.logs ?? []));
    }
  }
  return logs;
}

async function sendTransaction(
  methodName: string,
  args: Record<string, unknown>,
  deposit: string = '0'
) {
  if (deposit === '0') {
    const localResult = await tryLocalSign(methodName, args, deposit);
    if (localResult) return localResult;
  }
  const GAS_STR = '30000000000000';
  const c = getConnector();
  const wallet = await c.wallet();
  return wallet.signAndSendTransaction({
    receiverId: CONTRACT_ID,
    actions: [
      {
        type: 'FunctionCall',
        params: { methodName, args, gas: GAS_STR, deposit }
      }
    ]
  });
}

async function tryLocalSign(
  methodName: string,
  args: Record<string, unknown>,
  deposit: string
): Promise<unknown | null> {
  try {
    const { getLocalKeyPair } = await import('./account');
    const keyPair = getLocalKeyPair();
    if (!keyPair) {
      console.log('[connector] no local fc keypair, falling back to wallet');
      return null;
    }

    const accountId = await getAccountId();
    if (!accountId) return null;

    const signer = new KeyPairSigner(keyPair);
    const account = new Account(accountId, getProvider(), signer);

    console.log('[connector] local sign via Account for', methodName);
    const result = await account.signAndSendTransaction({
      receiverId: CONTRACT_ID,
      actions: [actions.functionCall(methodName, args, GAS, BigInt(deposit))]
    });
    console.log('[connector] local sign result:', result);
    return result;
  } catch (e) {
    console.warn('[connector] local sign failed, falling back to wallet:', e);
    return null;
  }
}

async function getAccountId(): Promise<string | null> {
  try {
    const c = getConnector();
    const wallet = await c.wallet();
    const accounts = await wallet.getAccounts();
    return accounts?.[0]?.accountId ?? null;
  } catch {
    return null;
  }
}

async function sendTransactions(
  calls: Array<{
    methodName: string;
    args: Record<string, unknown>;
    deposit?: string;
  }>
) {
  const GAS_STR = '30000000000000';
  const c = getConnector();
  const wallet = await c.wallet();
  return wallet.signAndSendTransactions({
    transactions: calls.map(({ methodName, args, deposit = '0' }) => ({
      receiverId: CONTRACT_ID,
      actions: [
        {
          type: 'FunctionCall',
          params: { methodName, args, gas: GAS_STR, deposit }
        }
      ]
    }))
  });
}

async function sendTokenTransaction(
  tokenId: string,
  methodName: string,
  args: Record<string, unknown>,
  deposit: string = '1'
) {
  const GAS_STR = '30000000000000';
  const c = getConnector();
  const wallet = await c.wallet();
  return wallet.signAndSendTransaction({
    receiverId: tokenId,
    actions: [
      {
        type: 'FunctionCall',
        params: { methodName, args, gas: GAS_STR, deposit }
      }
    ]
  });
}

export const contract = {
  storageDeposit() {
    return sendTransaction(
      'storage_deposit',
      { registration_only: true },
      '50000000000000000000000'
    );
  },

  storageDepositFor(accountId: string) {
    return sendTransaction(
      'storage_deposit',
      { account_id: accountId, registration_only: true },
      '50000000000000000000000'
    );
  },

  storageBalanceOf(accountId: string): Promise<string | null> {
    return viewFunction('storage_balance_of', { account_id: accountId });
  },

  playMove(gameId: unknown, mv: string) {
    return sendTransaction('play_move', { game_id: gameId, mv });
  },

  resign(gameId: unknown) {
    return sendTransaction('resign', { game_id: gameId });
  },

  cancel(gameId: unknown) {
    return sendTransaction('cancel', { game_id: gameId });
  },

  challengeWithRegistration(challenged: string) {
    return sendTransactions([
      {
        methodName: 'storage_deposit',
        args: { account_id: challenged, registration_only: true },
        deposit: '50000000000000000000000'
      },
      {
        methodName: 'challenge',
        args: { challenged_id: challenged }
      }
    ]);
  },

  challenge(challenged: string) {
    return sendTransaction('challenge', { challenged_id: challenged });
  },

  acceptChallenge(challengeId: string) {
    return sendTransaction('accept_challenge', { challenge_id: challengeId });
  },

  rejectChallenge(challengeId: string, isChallenger: boolean) {
    return sendTransaction('reject_challenge', {
      challenge_id: challengeId,
      is_challenger: isChallenger
    });
  },

  claimPoints() {
    return tryLocalSign('claim_points', {}, '0');
  },

  createAiGame(difficulty: 'Easy' | 'Medium' | 'Hard') {
    return sendTransaction('create_ai_game', { difficulty });
  },

  getChallenges(accountId: string, isChallenger: boolean): Promise<string[]> {
    return viewFunction('get_challenges', {
      account_id: accountId,
      is_challenger: isChallenger
    });
  },

  getGameIds(accountId: string): Promise<[number, string, string | null][]> {
    return viewFunction('get_game_ids', { account_id: accountId });
  },

  getBoard(gameId: unknown): Promise<string[]> {
    return viewFunction('get_board', { game_id: gameId });
  },

  getGameInfo(gameId: unknown): Promise<{
    white: { type: string; value: string };
    black: { type: string; value: string | null };
    turn_color: string;
    last_block_height: number;
    has_bets: boolean;
  }> {
    return viewFunction('game_info', { game_id: gameId });
  },

  getAccount(accountId: string): Promise<{
    near_amount: string;
    is_agent: boolean;
    points: string;
    pending_points: string;
    elo: number | null;
  }> {
    return viewFunction('get_account', { account_id: accountId });
  },

  getEloRatings(skip = 0, limit = 100): Promise<[string, number][]> {
    return viewFunction('get_elo_ratings', { skip, limit });
  },

  getEloRatingsByIds(accountIds: string[]): Promise<[string, number][]> {
    return viewFunction('get_elo_ratings_by_ids', { account_ids: accountIds });
  },

  getQuestList(): Promise<
    Array<{
      name: string;
      points: string;
      points_on_cd: string;
      cooldown: number;
    }>
  > {
    return viewFunction('get_quest_list', {});
  },

  getQuestCooldowns(accountId: string): Promise<Array<[number, string]>> {
    return viewFunction('get_quest_cooldowns', { account_id: accountId });
  },

  getAchievementList(): Promise<
    Array<{
      name: string;
      points: string;
    }>
  > {
    return viewFunction('get_achievement_list', {});
  },

  getAchievements(accountId: string): Promise<Array<[number, string]>> {
    return viewFunction('get_achievements', { account_id: accountId });
  },

  getTokenWhitelist(): Promise<string[]> {
    return viewFunction('get_token_whitelist', {});
  },

  getTokens(accountId: string): Promise<Array<[string, string]>> {
    return viewFunction('get_tokens', { account_id: accountId });
  },

  getBetInfo(players: [string, string]): Promise<{
    is_locked: boolean;
    bets: Record<string, Array<[string, { amount: string; winner: string }]>>;
  }> {
    return viewFunction('bet_info', { players });
  },

  challengeWithWager(tokenId: string, challenged: string, amount: string) {
    return sendTokenTransaction(tokenId, 'ft_transfer_call', {
      receiver_id: CONTRACT_ID,
      amount,
      msg: JSON.stringify({
        Challenge: { challenged_id: challenged }
      })
    });
  },

  acceptChallengeWithWager(
    tokenId: string,
    challengeId: string,
    amount: string
  ) {
    return sendTokenTransaction(tokenId, 'ft_transfer_call', {
      receiver_id: CONTRACT_ID,
      amount,
      msg: JSON.stringify({
        AcceptChallenge: { challenge_id: challengeId }
      })
    });
  },

  placeBet(
    tokenId: string,
    players: [string, string],
    winner: string,
    amount: string
  ) {
    return sendTokenTransaction(tokenId, 'ft_transfer_call', {
      receiver_id: CONTRACT_ID,
      amount,
      msg: JSON.stringify({
        Bet: { players, winner }
      })
    });
  },

  withdrawToken(tokenId: string) {
    return sendTransaction('withdraw_token', { token_id: tokenId }, '1');
  }
};
