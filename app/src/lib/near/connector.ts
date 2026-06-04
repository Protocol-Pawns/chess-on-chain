import { NearConnector } from '@hot-labs/near-connect';
import { JsonRpcProvider } from 'near-api-js';

const NETWORK = import.meta.env.VITE_NETWORK_ID || 'mainnet';
const CONTRACT_ID = import.meta.env.VITE_CONTRACT_ID || 'app.chess-game.near';
const RPC_URL = import.meta.env.VITE_RPC_URL || 'https://rpc.shitzuapes.xyz';
const GAS = '30000000000000';

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
		args: Buffer.from(JSON.stringify(args))
	});
	return result as T;
}

async function sendTransaction(methodName: string, args: Record<string, unknown>, deposit: string = '0') {
	const c = getConnector();
	const wallet = await c.wallet();
	return wallet.signAndSendTransaction({
		receiverId: CONTRACT_ID,
		actions: [{
			type: 'FunctionCall',
			params: { methodName, args, gas: GAS, deposit }
		}]
	});
}

export const contract = {
	storageDeposit() {
		return sendTransaction('storage_deposit', { registration_only: true }, '50000000000000000000000');
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

	challenge(challenged: string) {
		return sendTransaction('challenge', { challenged });
	},

	acceptChallenge(challengeId: string) {
		return sendTransaction('accept_challenge', { challenge_id: challengeId });
	},

	rejectChallenge(challengeId: string) {
		return sendTransaction('reject_challenge', { challenge_id: challengeId });
	},

	createAiGame(difficulty: 'Easy' | 'Medium' | 'Hard') {
		return sendTransaction('create_ai_game', { difficulty });
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

	getQuestList(): Promise<Array<{
		name: string;
		points: string;
		points_on_cd: string;
		cooldown: number;
	}>> {
		return viewFunction('get_quest_list', {});
	},

	getQuestCooldowns(accountId: string): Promise<Array<[number, string]>> {
		return viewFunction('get_quest_cooldowns', { account_id: accountId });
	},

	getAchievementList(): Promise<Array<{
		name: string;
		points: string;
	}>> {
		return viewFunction('get_achievement_list', {});
	},

	getAchievements(accountId: string): Promise<Array<[number, string]>> {
		return viewFunction('get_achievements', { account_id: accountId });
	}
};
