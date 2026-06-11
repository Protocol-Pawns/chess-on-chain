BEGIN;

CREATE TABLE IF NOT EXISTS chess_events (
  id TEXT PRIMARY KEY,
  trigger_block_height TEXT NOT NULL,
  trigger_block_timestamp TEXT NOT NULL,
  event_type TEXT NOT NULL,
  event_data TEXT NOT NULL,
  processed BOOLEAN NOT NULL DEFAULT FALSE,
  notified BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_chess_events_type ON chess_events (event_type);
CREATE INDEX IF NOT EXISTS idx_chess_events_unprocessed ON chess_events (processed) WHERE NOT processed;
CREATE INDEX IF NOT EXISTS idx_chess_events_unnotified ON chess_events (notified) WHERE processed = true AND notified = false;

CREATE TABLE IF NOT EXISTS games (
  game_id TEXT PRIMARY KEY,
  trigger_block_height BIGINT NOT NULL,
  white_type TEXT NOT NULL,
  white_value TEXT NOT NULL,
  black_type TEXT NOT NULL,
  black_value TEXT,
  board JSONB NOT NULL,
  fen TEXT,
  outcome JSONB,
  resigner TEXT,
  status TEXT NOT NULL DEFAULT 'in_progress',
  created_at BIGINT NOT NULL,
  finished_at BIGINT
);

CREATE INDEX IF NOT EXISTS idx_games_created ON games (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_games_finished ON games (finished_at DESC) WHERE finished_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_games_status ON games (status);
CREATE INDEX IF NOT EXISTS idx_games_white_value ON games (white_value);
CREATE INDEX IF NOT EXISTS idx_games_black_value ON games (black_value);

CREATE TABLE IF NOT EXISTS game_moves (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
  move_number INT NOT NULL,
  color TEXT NOT NULL,
  move_notation TEXT NOT NULL,
  fen TEXT NOT NULL,
  outcome JSONB,
  trigger_block_height BIGINT NOT NULL,
  trigger_block_timestamp BIGINT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_game_moves_game_move ON game_moves (game_id, move_number);
CREATE INDEX IF NOT EXISTS idx_game_moves_game ON game_moves (game_id);

CREATE TABLE IF NOT EXISTS challenges (
  id SERIAL PRIMARY KEY,
  challenge_id TEXT NOT NULL,
  challenger TEXT NOT NULL,
  challenged TEXT NOT NULL,
  wager_token TEXT,
  wager_amount TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  game_id TEXT,
  created_at BIGINT NOT NULL,
  resolved_at BIGINT
);

CREATE INDEX IF NOT EXISTS idx_challenges_challenge_id ON challenges (challenge_id);
CREATE INDEX IF NOT EXISTS idx_challenges_challenger ON challenges (challenger);
CREATE INDEX IF NOT EXISTS idx_challenges_challenged ON challenges (challenged);
CREATE INDEX IF NOT EXISTS idx_challenges_status ON challenges (status);

CREATE TABLE IF NOT EXISTS account_finished_games (
  account_id TEXT NOT NULL,
  game_id TEXT NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
  PRIMARY KEY (account_id, game_id)
);

CREATE INDEX IF NOT EXISTS idx_account_games ON account_finished_games (account_id);

CREATE TABLE IF NOT EXISTS bets (
  id SERIAL PRIMARY KEY,
  bet_key TEXT NOT NULL,
  bettor TEXT NOT NULL,
  player_0 TEXT NOT NULL,
  player_1 TEXT NOT NULL,
  game_id TEXT,
  token_id TEXT NOT NULL,
  amount TEXT NOT NULL,
  winner TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  payout TEXT,
  created_at BIGINT NOT NULL,
  locked_at BIGINT,
  resolved_at BIGINT
);

CREATE INDEX IF NOT EXISTS idx_bets_bet_key ON bets (bet_key);
CREATE INDEX IF NOT EXISTS idx_bets_bettor ON bets (bettor);
CREATE INDEX IF NOT EXISTS idx_bets_game ON bets (game_id);
CREATE INDEX IF NOT EXISTS idx_bets_status ON bets (status);
CREATE INDEX IF NOT EXISTS idx_bets_players ON bets (player_0, player_1);
CREATE INDEX IF NOT EXISTS idx_bets_token ON bets (token_id);

CREATE TABLE IF NOT EXISTS push_subscriptions (
  endpoint TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  p256dh TEXT NOT NULL,
  auth TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_push_subs_account ON push_subscriptions (account_id);

CREATE TABLE IF NOT EXISTS quest_cooldowns (
  account_id TEXT NOT NULL,
  quest TEXT NOT NULL,
  expires_at BIGINT NOT NULL,
  notified BOOLEAN NOT NULL DEFAULT FALSE,
  PRIMARY KEY (account_id, quest)
);

CREATE INDEX IF NOT EXISTS idx_quest_cooldowns_expires ON quest_cooldowns (expires_at) WHERE NOT notified;

COMMIT;
