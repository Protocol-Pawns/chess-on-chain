mod util;

use chess_engine::Color;
use chess_lib::{ChessEvent, Difficulty, GameId, Player};
use util::*;

#[tokio::test]
async fn test_init() -> anyhow::Result<()> {
    initialize_contracts().await?;

    Ok(())
}

#[tokio::test]
async fn test_ai_game() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts().await?;

    let player_a = worker.dev_create_account().await?;

    call::storage_deposit(&contract, &player_a, None, None).await?;
    let (game_id, events) = call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    let block_height = game_id.0;
    assert_event_emits(
        events,
        vec![ChessEvent::CreateGame {
            game_id: game_id.clone(),
            white: Player::Human(player_a.id().clone().parse()?),
            black: Player::Ai(Difficulty::Easy),
            board: [
                "RNBQKBNR".to_string(),
                "PPPPPPPP".to_string(),
                "        ".to_string(),
                "        ".to_string(),
                "        ".to_string(),
                "        ".to_string(),
                "pppppppp".to_string(),
                "rnbqkbnr".to_string(),
            ],
        }],
    )?;

    let game_ids = call::get_game_ids(&contract, player_a.id()).await?;
    assert_eq!(
        game_ids,
        vec![GameId(block_height, player_a.id().clone().parse()?, None)]
    );

    let ((outcome, _board), events) =
        call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    assert!(outcome.is_none());
    assert_event_emits(
        events,
        vec![
            ChessEvent::PlayMove {
                game_id: game_id.clone(),
                color: Color::White,
                mv: "e2 to e4".to_string(),
            },
            ChessEvent::ChangeBoard {
                game_id: game_id.clone(),
                board: [
                    "RNBQKBNR".to_string(),
                    "PPPP PPP".to_string(),
                    "        ".to_string(),
                    "    P   ".to_string(),
                    "        ".to_string(),
                    "        ".to_string(),
                    "pppppppp".to_string(),
                    "rnbqkbnr".to_string(),
                ],
            },
            ChessEvent::PlayMove {
                game_id: game_id.clone(),
                color: Color::Black,
                mv: "g8 to f6".to_string(),
            },
            ChessEvent::ChangeBoard {
                game_id: game_id.clone(),
                board: [
                    "RNBQKBNR".to_string(),
                    "PPPP PPP".to_string(),
                    "        ".to_string(),
                    "    P   ".to_string(),
                    "        ".to_string(),
                    "     n  ".to_string(),
                    "pppppppp".to_string(),
                    "rnbqkb r".to_string(),
                ],
            },
        ],
    )?;

    Ok(())
}
