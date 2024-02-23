import type { ConnectConfig, Contract } from "near-api-js";
import type { ContractMethods } from "near-api-js/lib/contract";
import { derived, readable } from "svelte/store";

import type {
  GameInfo,
  GameId,
  Quest,
  Achievement,
  Challenge,
  AccountInfo,
  AccountId,
  QuestInfo,
  AchievementInfo,
} from "$abi";
import type { ContractViewCall } from "$lib/models";

const nearConfig = {
  networkId: import.meta.env.VITE_NETWORK_ID,
  nodeUrl: import.meta.env.VITE_NODE_URL,
  walletUrl: import.meta.env.VITE_WALLET_URL,
  helperUrl: import.meta.env.VITE_HELPER_URL,
} as const satisfies ConnectConfig;

export const near$ = readable(
  import("near-api-js").then(({ connect, Account, Contract }) =>
    connect(nearConfig).then((near) => [near, Account, Contract] as const),
  ),
);

export const contract$ = derived(near$, async (n) => {
  const [near, Account, Contract] = await n;
  const account = new Account(
    near.connection,
    import.meta.env.VITE_CONTRACT_ID,
  );
  return new Contract(account, import.meta.env.VITE_CONTRACT_ID, {
    viewMethods: ["storage_balance_of", "ft_balance_of", "ft_metadata"],
    changeMethods: [],
    useLocalViewExecution: false,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
  } satisfies ContractMethods as any) as ProtocolPawnsContract;
});

export interface ProtocolPawnsContract extends Contract {
  get_board: ContractViewCall<
    {
      game_id: GameId;
    },
    string[]
  >;
  game_info: ContractViewCall<
    {
      game_id: GameId;
    },
    GameInfo
  >;
  get_game_ids: ContractViewCall<
    {
      account_id: AccountId;
    },
    GameId[]
  >;
  get_account: ContractViewCall<
    {
      account_id: AccountId;
    },
    AccountInfo
  >;
  get_elo_ratings: ContractViewCall<
    {
      skip?: number;
      limit?: number;
    },
    [AccountId, number][]
  >;
  get_quest_list: ContractViewCall<
    {
      account_id: AccountId;
    },
    QuestInfo[]
  >;
  get_quest_cooldowns: ContractViewCall<
    {
      account_id: AccountId;
    },
    [number, Quest][]
  >;
  get_achievement_list: ContractViewCall<
    {
      account_id: AccountId;
    },
    AchievementInfo[]
  >;
  get_achievements: ContractViewCall<
    {
      account_id: AccountId;
    },
    [number, Achievement][]
  >;
  get_challenge: ContractViewCall<
    {
      challenge_id: string;
    },
    Challenge
  >;
  get_challenges: ContractViewCall<
    {
      account_id: AccountId;
      is_challenger: boolean;
    },
    string[]
  >;

  storage_balance_of: ContractViewCall<
    {
      account_id: string;
    },
    string | null
  >;

  ft_balance_of: ContractViewCall<
    {
      account_id: string;
    },
    string
  >;
  ft_metadata: ContractViewCall<undefined, FtMetadata>;
}

export interface FtMetadata {
  name: string;
  symbol: string;
  icon: string;
  decimals: number;
}

export * from "./wallet";
