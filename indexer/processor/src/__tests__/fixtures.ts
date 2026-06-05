import type { RawEvent } from '../handlers.js';

const EMPTY_ROW = '        ';

export const STARTING_BOARD = [
  'RNBQKBNR',
  'PPPPPPPP',
  EMPTY_ROW,
  EMPTY_ROW,
  EMPTY_ROW,
  EMPTY_ROW,
  'pppppppp',
  'rnbqkbnr'
];

const BOARD_AFTER_E4 = [
  'RNBQKBNR',
  'PPPP PPP',
  EMPTY_ROW,
  '    P   ',
  EMPTY_ROW,
  EMPTY_ROW,
  'pppppppp',
  'rnbqkbnr'
];

let counter = 0;
function uid(): string {
  return `test_${++counter}_${Date.now()}`;
}

export function makeEvent(
  type: string,
  data: Record<string, unknown>,
  overrides?: Partial<
    Pick<RawEvent, 'id' | 'trigger_block_height' | 'trigger_block_timestamp'>
  >
): RawEvent {
  return {
    id: `${uid()}_${type}`,
    trigger_block_height: '12345678',
    trigger_block_timestamp: '1700000000000',
    event_type: type,
    event_data: data,
    ...overrides
  };
}

export function makeCreateGame(
  gameId: [number, string, string | null] = [123, 'alice.near', 'bob.near'],
  white = { type: 'Human', value: 'alice.near' },
  black = { type: 'Human', value: 'bob.near' },
  board = STARTING_BOARD
): RawEvent {
  return makeEvent('create_game', { game_id: gameId, white, black, board });
}

export function makePlayMove(
  gameId: [number, string, string | null] = [123, 'alice.near', 'bob.near'],
  color = 'White',
  mv = 'e4',
  board = BOARD_AFTER_E4,
  outcome?: Record<string, string> | null
): RawEvent {
  const data: Record<string, unknown> = { game_id: gameId, color, mv, board };
  if (outcome !== undefined) data.outcome = outcome;
  return makeEvent('play_move', data);
}

export function makeResignGame(
  gameId: [number, string, string | null] = [123, 'alice.near', 'bob.near'],
  resigner = 'White',
  outcome = { result: 'Victory', color: 'Black' }
): RawEvent {
  return makeEvent('resign_game', { game_id: gameId, resigner, outcome });
}

export function makeCancelGame(
  gameId: [number, string, string | null] = [123, 'alice.near', 'bob.near'],
  cancelledBy = 'alice.near'
): RawEvent {
  return makeEvent('cancel_game', {
    game_id: gameId,
    cancelled_by: cancelledBy
  });
}

export function makeChallenge(
  id = 'challenge_1',
  challenger = 'alice.near',
  challenged = 'bob.near',
  wager?: [string, string]
): RawEvent {
  const data: Record<string, unknown> = { id, challenger, challenged };
  if (wager) data.wager = wager;
  return makeEvent('challenge', data);
}

export function makeAcceptChallenge(
  challengeId = 'challenge_1',
  gameId: [number, string, string | null] = [123, 'alice.near', 'bob.near']
): RawEvent {
  return makeEvent('accept_challenge', {
    challenge_id: challengeId,
    game_id: gameId
  });
}

export function makeRejectChallenge(challengeId = 'challenge_1'): RawEvent {
  return makeEvent('reject_challenge', { challenge_id: challengeId });
}

export function makePlaceBet(
  bettor = 'carol.near',
  players = ['alice.near', 'bob.near'] as [string, string],
  tokenId = 'usdc.testnet',
  amount = '1000000',
  winner = 'alice.near'
): RawEvent {
  return makeEvent('place_bet', {
    bettor,
    players,
    token_id: tokenId,
    amount,
    winner
  });
}

export function makeCancelBet(
  bettor = 'carol.near',
  players = ['alice.near', 'bob.near'] as [string, string],
  tokenId = 'usdc.testnet',
  amount = '1000000'
): RawEvent {
  return makeEvent('cancel_bet', {
    bettor,
    players,
    token_id: tokenId,
    amount
  });
}

export function makeLockBets(
  players = ['alice.near', 'bob.near'] as [string, string],
  gameId: [number, string, string | null] = [123, 'alice.near', 'bob.near']
): RawEvent {
  return makeEvent('lock_bets', { players, game_id: gameId });
}

export function makeResolveBets(
  players = ['alice.near', 'bob.near'] as [string, string],
  gameId: [number, string, string | null] = [123, 'alice.near', 'bob.near'],
  outcome = { result: 'Victory', color: 'White' },
  payouts: Array<{ bettor: string; token_id: string; amount: string }> = []
): RawEvent {
  return makeEvent('resolve_bets', {
    players,
    game_id: gameId,
    outcome,
    payouts
  });
}
