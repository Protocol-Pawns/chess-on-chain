import { NearConnector } from '@hot-labs/near-connect';
import { JsonRpcProvider, Account, KeyPairSigner, actions } from 'near-api-js';

import { isAccessKeyError } from './errors';
import { requestRelogin } from './relogin';

import { AI_MOVE_GAS } from '$lib/format';
import type {
  AccountInfo,
  GameInfo,
  QuestInfo,
  AchievementInfo,
  BetInfo,
  GameId,
  Difficulty
} from '$lib/near/contract-types';

const NETWORK = import.meta.env.VITE_NETWORK_ID || 'mainnet';
const CONTRACT_ID = import.meta.env.VITE_CONTRACT_ID || 'app.chess-game.near';
const RPC_URL = import.meta.env.VITE_RPC_URL || 'https://rpc.shitzuapes.xyz';
const GAS = BigInt('30000000000000');
const GAS_HIGH = BigInt('300000000000000');
const WRAP_NEAR_ID = NETWORK === 'testnet' ? 'wrap.testnet' : 'wrap.near';

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

async function withAccessKeyRetry<T>(fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (err) {
    if (!isAccessKeyError(err)) throw err;
    try {
      await requestRelogin();
    } catch {
      throw err;
    }
    return await fn();
  }
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

export async function viewTokenFunction<T = unknown>(
  contractId: string,
  methodName: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  const p = getProvider();
  const result = await p.callFunction({
    contractId,
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

interface FinalExecutionOutcomeLike {
  transaction?: { hash?: string };
  status?:
    | 'NotStarted'
    | 'Started'
    | 'Unknown'
    | { Failure?: unknown }
    | { SuccessValue?: string }
    | Record<string, unknown>;
  final_execution_status?: string;
  receipts_outcome?: unknown;
  transaction_outcome?: unknown;
}

function extractFailureMessage(failure: unknown): string {
  if (!failure || typeof failure !== 'object') return String(failure);
  const f = failure as Record<string, unknown>;
  const actionError = f.ActionError as
    | { kind?: Record<string, unknown>; index?: number }
    | undefined;
  if (actionError?.kind) {
    const fc = actionError.kind.FunctionCallError as
      | { ExecutionError?: string }
      | undefined;
    if (fc?.ExecutionError) return fc.ExecutionError;
  }
  if (actionError?.kind) return JSON.stringify(actionError.kind);
  if (f.InvalidTxError) return JSON.stringify(f.InvalidTxError);
  return JSON.stringify(failure);
}

const TX_POLL_INTERVAL_MS = 1200;
const TX_POLL_MAX_ATTEMPTS = 30;

export async function awaitTxOutcome(
  txHash: string,
  accountId: string
): Promise<FinalExecutionOutcomeLike> {
  const p = getProvider();
  for (let attempt = 0; attempt < TX_POLL_MAX_ATTEMPTS; attempt++) {
    try {
      const res = (await p.sendJsonRpc('EXPERIMENTAL_tx_status', [
        txHash,
        accountId
      ])) as FinalExecutionOutcomeLike;
      const final = res?.final_execution_status;
      const st = res?.status;
      const isDefinitive =
        st !== undefined &&
        st !== null &&
        typeof st === 'object' &&
        ('Failure' in (st as object) || 'SuccessValue' in (st as object));
      if (final === 'FINAL' || final === 'EXECUTED' || isDefinitive) {
        return res;
      }
    } catch (err) {
      if (attempt === TX_POLL_MAX_ATTEMPTS - 1) {
        console.warn('[connector] tx status poll exhausted', err);
      }
    }
    await new Promise(r => setTimeout(r, TX_POLL_INTERVAL_MS));
  }
  throw new Error('Transaction finalisation timed out');
}

export async function verifyOutcome<T>(result: T): Promise<T> {
  const isArray = Array.isArray(result);
  const rawOutcomes: FinalExecutionOutcomeLike[] = isArray
    ? (result as unknown[])
        .filter(Boolean)
        .map(o => o as FinalExecutionOutcomeLike)
    : [result as unknown as FinalExecutionOutcomeLike].filter(Boolean);
  if (rawOutcomes.length === 0) return result;

  let accountId: string | null = null;
  const finalOutcomes: FinalExecutionOutcomeLike[] = [];
  for (const oc of rawOutcomes) {
    const st = oc?.status;
    const alreadyDefinitive =
      st !== undefined &&
      st !== null &&
      typeof st === 'object' &&
      ('Failure' in (st as object) || 'SuccessValue' in (st as object));
    let finalOutcome = oc;
    if (!alreadyDefinitive) {
      const hash = oc?.transaction?.hash;
      if (hash) {
        if (!accountId) accountId = await getAccountId();
        if (accountId) {
          try {
            finalOutcome = await awaitTxOutcome(hash, accountId);
          } catch {
            /* keep original outcome */
          }
        }
      }
    }
    const status = finalOutcome?.status as
      | { Failure?: unknown }
      | { SuccessValue?: string }
      | string
      | undefined;
    if (
      status !== undefined &&
      status !== null &&
      typeof status === 'object' &&
      'Failure' in (status as object)
    ) {
      throw new Error(
        extractFailureMessage((status as { Failure: unknown }).Failure)
      );
    }
    finalOutcomes.push(finalOutcome);
  }
  return (isArray ? finalOutcomes : finalOutcomes[0]) as T;
}

async function _sendTransaction(
  methodName: string,
  args: Record<string, unknown>,
  deposit: string = '0',
  gas: bigint = GAS
) {
  if (deposit === '0') {
    const localResult = await tryLocalSign(methodName, args, deposit, gas);
    if (localResult) return verifyOutcome(localResult);
  }
  const GAS_STR = gas.toString();
  const c = getConnector();
  const wallet = await c.wallet();
  const result = await wallet.signAndSendTransaction({
    receiverId: CONTRACT_ID,
    actions: [
      {
        type: 'FunctionCall',
        params: { methodName, args, gas: GAS_STR, deposit }
      }
    ]
  });
  return verifyOutcome(result);
}

function sendTransaction(
  methodName: string,
  args: Record<string, unknown>,
  deposit: string = '0',
  gas: bigint = GAS
) {
  return withAccessKeyRetry(() =>
    _sendTransaction(methodName, args, deposit, gas)
  );
}

async function tryLocalSign(
  methodName: string,
  args: Record<string, unknown>,
  deposit: string,
  gas: bigint = GAS
): Promise<unknown | null> {
  let keyPair;
  try {
    const { getLocalKeyPair } = await import('./account');
    keyPair = getLocalKeyPair();
  } catch {
    return null;
  }
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
    actions: [actions.functionCall(methodName, args, gas, BigInt(deposit))]
  });
  console.log('[connector] local sign result:', result);
  return result;
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

async function _sendTransactions(
  calls: Array<{
    methodName: string;
    args: Record<string, unknown>;
    deposit?: string;
  }>
) {
  const GAS_STR = '30000000000000';
  const c = getConnector();
  const wallet = await c.wallet();
  const result = await wallet.signAndSendTransactions({
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
  return verifyOutcome(result);
}

function sendTransactions(
  calls: Array<{
    methodName: string;
    args: Record<string, unknown>;
    deposit?: string;
  }>
) {
  return withAccessKeyRetry(() => _sendTransactions(calls));
}

async function _sendTokenTransaction(
  tokenId: string,
  methodName: string,
  args: Record<string, unknown>,
  deposit: string = '1'
) {
  const GAS_STR = '60000000000000';
  const c = getConnector();
  const wallet = await c.wallet();
  const result = await wallet.signAndSendTransaction({
    receiverId: tokenId,
    actions: [
      {
        type: 'FunctionCall',
        params: { methodName, args, gas: GAS_STR, deposit }
      }
    ]
  });
  return verifyOutcome(result);
}

export async function getNearNativeBalance(accountId: string): Promise<bigint> {
  const p = getProvider();
  const result = await p.query({
    request_type: 'view_account',
    account_id: accountId,
    finality: 'optimistic'
  });
  return BigInt((result as unknown as { amount: string }).amount);
}

async function _sendTokenTransactionWithAutoWrap(
  tokenId: string,
  methodName: string,
  args: Record<string, unknown>,
  deposit: string = '1'
) {
  if (tokenId !== WRAP_NEAR_ID) {
    return _sendTokenTransaction(tokenId, methodName, args, deposit);
  }

  const GAS_STR = '30000000000000';
  const amount = args.amount as string;
  const accountId = await getAccountId();

  let wNearBalance = 0n;
  if (accountId) {
    try {
      wNearBalance = BigInt(
        await viewTokenFunction<string>(WRAP_NEAR_ID, 'ft_balance_of', {
          account_id: accountId
        })
      );
    } catch {
      wNearBalance = 0n;
    }
  }

  const needed = BigInt(amount);
  if (wNearBalance >= needed) {
    return _sendTokenTransaction(tokenId, methodName, args, deposit);
  }

  const shortfall = needed - wNearBalance;

  const c = getConnector();
  const wallet = await c.wallet();
  const result = await wallet.signAndSendTransaction({
    receiverId: WRAP_NEAR_ID,
    actions: [
      {
        type: 'FunctionCall',
        params: {
          methodName: 'near_deposit',
          args: {},
          gas: GAS_STR,
          deposit: shortfall.toString()
        }
      },
      {
        type: 'FunctionCall',
        params: { methodName, args, gas: GAS_STR, deposit }
      }
    ]
  });
  return verifyOutcome(result);
}

function sendTokenTransactionWithAutoWrap(
  tokenId: string,
  methodName: string,
  args: Record<string, unknown>,
  deposit: string = '1'
) {
  return withAccessKeyRetry(() =>
    _sendTokenTransactionWithAutoWrap(tokenId, methodName, args, deposit)
  );
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

  playMove(gameId: unknown, mv: string, difficulty?: Difficulty) {
    let gas = GAS_HIGH;
    if (difficulty) {
      gas = AI_MOVE_GAS[difficulty];
    }
    return sendTransaction('play_move', { game_id: gameId, mv }, '0', gas);
  },

  resign(gameId: unknown) {
    return sendTransaction('resign', { game_id: gameId }, '0', GAS_HIGH);
  },

  cancel(gameId: unknown) {
    return sendTransaction('cancel', { game_id: gameId }, '0', GAS_HIGH);
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
    return withAccessKeyRetry(async () =>
      verifyOutcome(await tryLocalSign('claim_points', {}, '0'))
    );
  },

  createAiGame(difficulty: Difficulty) {
    return sendTransaction('create_ai_game', { difficulty });
  },

  getChallenges(accountId: string, isChallenger: boolean): Promise<string[]> {
    return viewFunction('get_challenges', {
      account_id: accountId,
      is_challenger: isChallenger
    });
  },

  getGameIds(accountId: string): Promise<GameId[]> {
    return viewFunction('get_game_ids', { account_id: accountId });
  },

  getBoard(gameId: unknown): Promise<string[]> {
    return viewFunction('get_board', { game_id: gameId });
  },

  getGameInfo(gameId: unknown): Promise<GameInfo> {
    return viewFunction('game_info', { game_id: gameId });
  },

  getAccount(accountId: string): Promise<AccountInfo> {
    return viewFunction('get_account', { account_id: accountId });
  },

  getEloRatings(skip = 0, limit = 100): Promise<[string, number][]> {
    return viewFunction('get_elo_ratings', { skip, limit });
  },

  getEloRatingsByIds(accountIds: string[]): Promise<[string, number][]> {
    return viewFunction('get_elo_ratings_by_ids', { account_ids: accountIds });
  },

  getQuestList(): Promise<QuestInfo[]> {
    return viewFunction('get_quest_list', {});
  },

  getQuestCooldowns(accountId: string): Promise<Array<[number, string]>> {
    return viewFunction('get_quest_cooldowns', { account_id: accountId });
  },

  getAchievementList(): Promise<AchievementInfo[]> {
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

  getBetInfo(players: [string, string]): Promise<BetInfo> {
    return viewFunction('bet_info', { players });
  },

  challengeWithWager(tokenId: string, challenged: string, amount: string) {
    return sendTokenTransactionWithAutoWrap(tokenId, 'ft_transfer_call', {
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
    return sendTokenTransactionWithAutoWrap(tokenId, 'ft_transfer_call', {
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
    return sendTokenTransactionWithAutoWrap(tokenId, 'ft_transfer_call', {
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
