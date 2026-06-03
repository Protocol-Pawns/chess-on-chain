BEGIN;

CREATE TABLE IF NOT EXISTS chess_events (
  id TEXT PRIMARY KEY,
  trigger_block_height BIGINT NOT NULL,
  trigger_block_timestamp BIGINT NOT NULL,
  event_type TEXT NOT NULL,
  event_data JSONB NOT NULL,
  processed BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_chess_events_type ON chess_events (event_type);
CREATE INDEX IF NOT EXISTS idx_chess_events_unprocessed ON chess_events (processed) WHERE NOT processed;

CREATE TABLE IF NOT EXISTS games (
  game_id TEXT PRIMARY KEY,
  trigger_block_height BIGINT NOT NULL,
  white_type TEXT NOT NULL,
  white_value TEXT NOT NULL,
  black_type TEXT NOT NULL,
  black_value TEXT,
  board JSONB NOT NULL,
  moves JSONB NOT NULL DEFAULT '[]'::JSONB,
  outcome JSONB,
  resigner TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  finished_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_games_created ON games (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_games_finished ON games (finished_at DESC) WHERE finished_at IS NOT NULL;

CREATE TABLE IF NOT EXISTS account_finished_games (
  account_id TEXT NOT NULL,
  game_id TEXT NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
  PRIMARY KEY (account_id, game_id)
);

CREATE INDEX IF NOT EXISTS idx_account_games ON account_finished_games (account_id);

CREATE OR REPLACE FUNCTION process_chess_event() RETURNS TRIGGER AS $$
DECLARE
  v_game_id TEXT;
  v_event_data JSONB;
  v_white_type TEXT;
  v_white_value TEXT;
  v_black_type TEXT;
  v_black_value TEXT;
  v_board JSONB;
  v_outcome JSONB;
  v_color TEXT;
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

    INSERT INTO games (game_id, trigger_block_height, white_type, white_value, black_type, black_value, board)
    VALUES (v_game_id, NEW.trigger_block_height, v_white_type, v_white_value, v_black_type, v_black_value, v_board)
    ON CONFLICT (game_id) DO NOTHING;

  ELSIF NEW.event_type = 'play_move' THEN
    v_game_id := (v_event_data->>'game_id')::TEXT;
    IF v_game_id IS NULL THEN
      RAISE WARNING 'play_move event missing game_id: %', NEW.id;
      RETURN NEW;
    END IF;

    v_board := v_event_data->'board';
    v_outcome := v_event_data->'outcome';
    v_color := v_event_data->>'color';

    UPDATE games SET
      board = COALESCE(v_board, board),
      moves = moves || jsonb_build_array(
        jsonb_build_object(
          'color', v_color,
          'mv', v_event_data->>'mv',
          'board', v_board
        )
      )
    WHERE game_id = v_game_id;

    IF v_outcome IS NOT NULL THEN
      UPDATE games SET
        outcome = v_outcome,
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
      finished_at = to_timestamp(NEW.trigger_block_timestamp / 1000.0)
    WHERE game_id = v_game_id;

    PERFORM insert_account_finished_games(v_game_id);

  ELSIF NEW.event_type = 'cancel_game' THEN
    v_game_id := (v_event_data->>'game_id')::TEXT;
    IF v_game_id IS NULL THEN
      RAISE WARNING 'cancel_game event missing game_id: %', NEW.id;
      RETURN NEW;
    END IF;

    DELETE FROM account_finished_games WHERE game_id = v_game_id;
    DELETE FROM games WHERE game_id = v_game_id;

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
