BEGIN;

CREATE TABLE IF NOT EXISTS chess_events (
  id TEXT PRIMARY KEY,
  trigger_block_height BIGINT NOT NULL,
  trigger_block_timestamp BIGINT NOT NULL,
  event_type TEXT NOT NULL,
  event_data JSONB NOT NULL,
  processed BOOLEAN NOT NULL DEFAULT FALSE,
  notified BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
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
  moves JSONB NOT NULL DEFAULT '[]'::JSONB,
  outcome JSONB,
  resigner TEXT,
  status TEXT NOT NULL DEFAULT 'in_progress',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  finished_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_games_created ON games (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_games_finished ON games (finished_at DESC) WHERE finished_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_games_status ON games (status);

CREATE TABLE IF NOT EXISTS game_moves (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
  move_number INT NOT NULL,
  color TEXT NOT NULL,
  move_notation TEXT NOT NULL,
  fen TEXT NOT NULL,
  outcome JSONB,
  trigger_block_height BIGINT NOT NULL,
  trigger_block_timestamp BIGINT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_game_moves_game_move ON game_moves (game_id, move_number);
CREATE INDEX IF NOT EXISTS idx_game_moves_game ON game_moves (game_id);

CREATE TABLE IF NOT EXISTS challenges (
  id TEXT PRIMARY KEY,
  challenger TEXT NOT NULL,
  challenged TEXT NOT NULL,
  wager_token TEXT,
  wager_amount TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  game_id TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  resolved_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_challenges_challenger ON challenges (challenger);
CREATE INDEX IF NOT EXISTS idx_challenges_challenged ON challenges (challenged);
CREATE INDEX IF NOT EXISTS idx_challenges_status ON challenges (status);

CREATE TABLE IF NOT EXISTS account_finished_games (
  account_id TEXT NOT NULL,
  game_id TEXT NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
  PRIMARY KEY (account_id, game_id)
);

CREATE INDEX IF NOT EXISTS idx_account_games ON account_finished_games (account_id);

CREATE TABLE IF NOT EXISTS push_subscriptions (
  endpoint TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  p256dh TEXT NOT NULL,
  auth TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_push_subs_account ON push_subscriptions (account_id);

CREATE OR REPLACE FUNCTION ascii_board_to_fen(board JSONB) RETURNS TEXT AS $$
DECLARE
  result TEXT := '';
  row_text TEXT;
  ch TEXT;
  empty_count INT;
  fen_row TEXT;
  i INT;
  j INT;
BEGIN
  FOR i IN REVERSE 7..0 LOOP
    row_text := board->>i;
    fen_row := '';
    empty_count := 0;
    FOR j IN 0..7 LOOP
      ch := substring(row_text FROM j + 1 FOR 1);
      IF ch = ' ' OR ch IS NULL THEN
        empty_count := empty_count + 1;
      ELSE
        IF empty_count > 0 THEN
          fen_row := fen_row || empty_count::TEXT;
          empty_count := 0;
        END IF;
        fen_row := fen_row || ch;
      END IF;
    END LOOP;
    IF empty_count > 0 THEN
      fen_row := fen_row || empty_count::TEXT;
    END IF;
    IF result = '' THEN
      result := fen_row;
    ELSE
      result := result || '/' || fen_row;
    END IF;
  END LOOP;
  RETURN result;
END;
$$ LANGUAGE plpgsql IMMUTABLE STRICT;

CREATE OR REPLACE FUNCTION process_chess_event() RETURNS TRIGGER AS $$
DECLARE
  v_game_id TEXT;
  v_event_data JSONB;
  v_white_type TEXT;
  v_white_value TEXT;
  v_black_type TEXT;
  v_black_value TEXT;
  v_board JSONB;
  v_fen TEXT;
  v_outcome JSONB;
  v_color TEXT;
  v_move_num INT;
  v_fullmove INT;
  v_active_color TEXT;
  v_challenge_id TEXT;
  v_wager JSONB;
BEGIN
  v_event_data := NEW.event_data;

  IF NEW.event_type = 'create_game' THEN
    v_game_id := v_event_data->>'game_id';
    IF v_game_id IS NULL THEN
      RAISE WARNING 'create_game event missing game_id: %', NEW.id;
      RETURN NEW;
    END IF;

    v_game_id := v_game_id::TEXT;
    v_white_type := v_event_data->'white'->>'type';
    v_white_value := v_event_data->'white'->>'value';
    v_black_type := v_event_data->'black'->>'type';
    v_black_value := v_event_data->'black'->>'value';
    v_board := v_event_data->'board';
    v_fen := ascii_board_to_fen(v_board) || ' w - - 0 1';

    INSERT INTO games (game_id, trigger_block_height, white_type, white_value, black_type, black_value, board, fen)
    VALUES (v_game_id, NEW.trigger_block_height, v_white_type, v_white_value, v_black_type, v_black_value, v_board, v_fen)
    ON CONFLICT (game_id) DO NOTHING;

  ELSIF NEW.event_type = 'play_move' THEN
    v_game_id := (v_event_data->>'game_id')::TEXT;
    IF v_game_id IS NULL THEN
      RAISE WARNING 'play_move event missing game_id: %', NEW.id;
      RETURN NEW;
    END IF;

    v_board := v_event_data->'board';
    v_fen := ascii_board_to_fen(v_board);
    v_outcome := v_event_data->'outcome';
    v_color := v_event_data->>'color';

    IF v_color = 'White' THEN
      v_active_color := 'b';
    ELSE
      v_active_color := 'w';
    END IF;

    SELECT COALESCE(MAX(move_number), 0) + 1 INTO v_move_num
    FROM game_moves WHERE game_id = v_game_id;

    v_fullmove := (v_move_num + 1) / 2 + 1;
    v_fen := v_fen || ' ' || v_active_color || ' - - 0 ' || v_fullmove::TEXT;

    INSERT INTO game_moves (id, game_id, move_number, color, move_notation, fen, outcome, trigger_block_height, trigger_block_timestamp)
    VALUES (NEW.id, v_game_id, v_move_num, v_color, v_event_data->>'mv', v_fen, v_outcome, NEW.trigger_block_height, NEW.trigger_block_timestamp)
    ON CONFLICT (id) DO NOTHING;

    UPDATE games SET
      board = COALESCE(v_board, board),
      fen = v_fen,
      moves = moves || jsonb_build_array(
        jsonb_build_object(
          'color', v_color,
          'mv', v_event_data->>'mv',
          'board', v_board,
          'fen', v_fen
        )
      )
    WHERE game_id = v_game_id;

    IF v_outcome IS NOT NULL THEN
      UPDATE games SET
        outcome = v_outcome,
        status = 'finished',
        finished_at = to_timestamp(NEW.trigger_block_timestamp / 1000.0)
      WHERE game_id = v_game_id;

      PERFORM insert_account_finished_games(v_game_id);
    END IF;

  ELSIF NEW.event_type = 'resign_game' THEN
    v_game_id := (v_event_data->>'game_id')::TEXT;
    IF v_game_id IS NULL THEN
      RAISE WARNING 'resign_game event missing game_id: %', NEW.id;
      RETURN NEW;
    END IF;

    v_outcome := v_event_data->'outcome';

    UPDATE games SET
      outcome = v_outcome,
      resigner = v_event_data->>'resigner',
      status = 'finished',
      finished_at = to_timestamp(NEW.trigger_block_timestamp / 1000.0)
    WHERE game_id = v_game_id;

    PERFORM insert_account_finished_games(v_game_id);

  ELSIF NEW.event_type = 'cancel_game' THEN
    v_game_id := (v_event_data->>'game_id')::TEXT;
    IF v_game_id IS NULL THEN
      RAISE WARNING 'cancel_game event missing game_id: %', NEW.id;
      RETURN NEW;
    END IF;

    UPDATE games SET status = 'cancelled' WHERE game_id = v_game_id;
    DELETE FROM account_finished_games WHERE game_id = v_game_id;

  ELSIF NEW.event_type = 'challenge' THEN
    v_challenge_id := v_event_data->>'id';
    IF v_challenge_id IS NULL THEN
      RAISE WARNING 'challenge event missing id: %', NEW.id;
      RETURN NEW;
    END IF;

    v_wager := v_event_data->'wager';

    INSERT INTO challenges (id, challenger, challenged, wager_token, wager_amount)
    VALUES (
      v_challenge_id,
      v_event_data->>'challenger',
      v_event_data->>'challenged',
      CASE WHEN v_wager IS NOT NULL THEN v_wager->>0 END,
      CASE WHEN v_wager IS NOT NULL THEN v_wager->>1 END
    )
    ON CONFLICT (id) DO NOTHING;

  ELSIF NEW.event_type = 'accept_challenge' THEN
    v_challenge_id := v_event_data->>'challenge_id';
    IF v_challenge_id IS NULL THEN
      RAISE WARNING 'accept_challenge event missing challenge_id: %', NEW.id;
      RETURN NEW;
    END IF;

    UPDATE challenges SET
      status = 'accepted',
      game_id = v_event_data->>'game_id',
      resolved_at = to_timestamp(NEW.trigger_block_timestamp / 1000.0)
    WHERE id = v_challenge_id;

  ELSIF NEW.event_type = 'reject_challenge' THEN
    v_challenge_id := v_event_data->>'challenge_id';
    IF v_challenge_id IS NULL THEN
      RAISE WARNING 'reject_challenge event missing challenge_id: %', NEW.id;
      RETURN NEW;
    END IF;

    UPDATE challenges SET
      status = 'rejected',
      resolved_at = to_timestamp(NEW.trigger_block_timestamp / 1000.0)
    WHERE id = v_challenge_id;

  END IF;

  NEW.processed := TRUE;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION insert_account_finished_games(p_game_id TEXT) RETURNS VOID AS $$
DECLARE
  v_game RECORD;
BEGIN
  SELECT white_type, white_value, black_type, black_value INTO v_game FROM games WHERE game_id = p_game_id;
  IF NOT FOUND THEN RETURN; END IF;

  IF v_game.white_type = 'Human' THEN
    INSERT INTO account_finished_games (account_id, game_id)
    VALUES (v_game.white_value, p_game_id)
    ON CONFLICT DO NOTHING;
  END IF;

  IF v_game.black_type = 'Human' THEN
    INSERT INTO account_finished_games (account_id, game_id)
    VALUES (v_game.black_value, p_game_id)
    ON CONFLICT DO NOTHING;
  END IF;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_process_chess_event ON chess_events;
CREATE TRIGGER trg_process_chess_event
  AFTER INSERT ON chess_events
  FOR EACH ROW
  EXECUTE FUNCTION process_chess_event();

COMMIT;
