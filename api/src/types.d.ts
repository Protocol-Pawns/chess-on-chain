import type { KVNamespace } from '@cloudflare/workers-types';
import type { Sql } from 'postgres';

export type Optional<T, K extends keyof T> = Omit<T, K> & Partial<T>;

export type AppEnv = {
  Bindings: {
    DATABASE_URL: string;
    VAPID_PRIVATE_KEY: string;
    VAPID_PUBLIC_KEY: string;
    VAPID_SUBJECT: string;
    LEADERBOARD_CACHE: KVNamespace;
    RPC_URL: string;
    CONTRACT_ID: string;
  };
  Variables: {
    DB: Sql;
  };
};
