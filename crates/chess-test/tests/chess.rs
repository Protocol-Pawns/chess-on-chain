mod points;
mod util;
mod wager;

use chess_engine::Color;
use chess_lib::{
    create_challenge_id, Challenge, ChessEvent, ChessNotification, Difficulty, GameId, GameInfo,
    GameOutcome, Player,
};
use futures::future::try_join_all;
use maplit::hashmap;
use tokio::fs;
use util::*;

#[tokio::test]
async fn test_migrate() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) =
        initialize_contracts(Some("../../res/chess_old.wasm")).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let player_c = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &player_c, None, None)
    )?;

    let (_res, _events) = call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (game_id, _events) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );

    call::create_ai_game(&contract, &player_c, Difficulty::Easy).await?;

    contract
        .as_account()
        .deploy(&fs::read("../../res/chess_testing.wasm").await?)
        .await?
        .into_result()?;
    call::migrate(&contract, contract.as_account()).await?;

    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert_eq!(game_ids, vec![game_id.clone()]);
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert_eq!(game_ids, vec![game_id.clone()]);
    let ai_game_ids = view::get_game_ids(&contract, player_c.id()).await?;
    assert!(!ai_game_ids.is_empty());
    let res = view::finished_games(&contract, player_a.id()).await;
    assert!(res.is_ok());
    let res = view::recent_finished_games(&contract).await;
    assert!(res.is_ok());
    let game_info = view::get_game_info(&contract, &game_id).await?;
    let mut actual = serde_json::to_value(game_info)?;
    actual["last_block_height"].take();
    let mut expected = serde_json::to_value(GameInfo {
        white: Player::Human(player_a.id().parse()?),
        black: Player::Human(player_b.id().parse()?),
        turn_color: Color::White,
        last_block_height: block_height,
    })?;
    expected["last_block_height"].take();
    assert_eq!(actual, expected);

    Ok(())
}

#[tokio::test]
async fn test_ai_game() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;

    call::storage_deposit(&contract, &player_a, None, None).await?;
    let (game_id, events) = call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    let block_height = game_id.0;
    let game_id = GameId(block_height, player_a.id().clone().parse()?, None);
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

    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert_eq!(game_ids, vec![game_id.clone()]);

    let ((outcome, _board), _, events) =
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

    let (res, events) = call::resign(&contract, &player_a, &game_id).await?;
    assert_eq!(res, GameOutcome::Victory(Color::Black));
    let expected_board = [
        "RNBQKBNR".to_string(),
        "PPPP PPP".to_string(),
        "        ".to_string(),
        "    P   ".to_string(),
        "        ".to_string(),
        "     n  ".to_string(),
        "pppppppp".to_string(),
        "rnbqkb r".to_string(),
    ];
    assert_event_emits(
        events,
        vec![
            ChessEvent::ResignGame {
                game_id: game_id.clone(),
                resigner: player_a.id().parse()?,
            },
            ChessEvent::FinishGame {
                game_id: game_id.clone(),
                outcome: GameOutcome::Victory(Color::Black),
                board: expected_board,
            },
        ],
    )?;
    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_accept_challenge() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    let (_res, events) = call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let challenge_ids = view::get_challenges(&contract, player_a.id(), true).await?;
    assert_eq!(challenge_ids, vec![challenge_id.clone()]);
    let challenge_ids = view::get_challenges(&contract, player_a.id(), false).await?;
    assert!(challenge_ids.is_empty());
    let challenge_ids = view::get_challenges(&contract, player_b.id(), true).await?;
    assert!(challenge_ids.is_empty());
    let challenge_ids = view::get_challenges(&contract, player_b.id(), false).await?;
    assert_eq!(challenge_ids, vec![challenge_id.clone()]);
    let challenge = view::get_challenge(&contract, &challenge_id).await?;
    let expected_challenge = Challenge::new(player_a.id().parse()?, player_b.id().parse()?, None);
    assert_eq!(&challenge, &expected_challenge);
    assert_event_emits(events, vec![ChessEvent::Challenge(expected_challenge)])?;

    let (game_id, events) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );
    assert_event_emits(
        events,
        vec![
            ChessEvent::AcceptChallenge {
                challenge_id: create_challenge_id(player_a.id(), player_b.id()),
                game_id: game_id.clone(),
            },
            ChessEvent::CreateGame {
                game_id: game_id.clone(),
                white: Player::Human(player_a.id().clone().parse()?),
                black: Player::Human(player_b.id().clone().parse()?),
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
            },
        ],
    )?;

    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert_eq!(game_ids, vec![game_id.clone()]);
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert_eq!(game_ids, vec![game_id.clone()]);

    let ((outcome, _board), _, events) =
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
        ],
    )?;

    Ok(())
}
#[tokio::test]
async fn test_reject_challenge() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    call::storage_deposit(&contract, &player_a, None, None).await?;
    call::storage_deposit(&contract, &player_b, None, None).await?;

    let (_res, events) = call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let challenge_ids = view::get_challenges(&contract, player_a.id(), true).await?;
    assert_eq!(challenge_ids, vec![challenge_id.clone()]);
    let challenge_ids = view::get_challenges(&contract, player_b.id(), false).await?;
    assert_eq!(challenge_ids, vec![challenge_id.clone()]);
    assert_event_emits(
        events,
        vec![ChessEvent::Challenge(Challenge::new(
            player_a.id().parse()?,
            player_b.id().parse()?,
            None,
        ))],
    )?;

    let (_res, events) = call::reject_challenge(&contract, &player_b, &challenge_id, false).await?;
    assert_event_emits(
        events,
        vec![ChessEvent::RejectChallenge {
            challenge_id: create_challenge_id(player_a.id(), player_b.id()),
        }],
    )?;

    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(game_ids.is_empty());
    let challenge_ids = view::get_challenges(&contract, player_a.id(), true).await?;
    assert!(challenge_ids.is_empty());
    let challenge_ids = view::get_challenges(&contract, player_b.id(), false).await?;
    assert!(challenge_ids.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_accept_reject_challenge_check_sender() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let player_c = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &player_c, None, None)
    )?;

    let (_res, events) = call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let challenge_ids = view::get_challenges(&contract, player_a.id(), true).await?;
    assert_eq!(challenge_ids, vec![challenge_id.clone()]);
    let challenge_ids = view::get_challenges(&contract, player_a.id(), false).await?;
    assert!(challenge_ids.is_empty());
    let challenge_ids = view::get_challenges(&contract, player_b.id(), true).await?;
    assert!(challenge_ids.is_empty());
    let challenge_ids = view::get_challenges(&contract, player_b.id(), false).await?;
    assert_eq!(challenge_ids, vec![challenge_id.clone()]);
    let challenge = view::get_challenge(&contract, &challenge_id).await?;
    let expected_challenge = Challenge::new(player_a.id().parse()?, player_b.id().parse()?, None);
    assert_eq!(&challenge, &expected_challenge);
    assert_event_emits(events, vec![ChessEvent::Challenge(expected_challenge)])?;

    let res = call::accept_challenge(&contract, &player_a, &challenge_id).await;
    assert!(res.is_err());
    let res = call::reject_challenge(&contract, &player_a, &challenge_id, false).await;
    assert!(res.is_err());
    let res = call::reject_challenge(&contract, &player_b, &challenge_id, true).await;
    assert!(res.is_err());
    let res = call::accept_challenge(&contract, &player_c, &challenge_id).await;
    assert!(res.is_err());
    let res = call::reject_challenge(&contract, &player_c, &challenge_id, false).await;
    assert!(res.is_err());
    let res = call::reject_challenge(&contract, &player_c, &challenge_id, true).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_challenge_check_duplicate() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    call::challenge(&contract, &player_a, player_b.id()).await?;

    let res = call::challenge(&contract, &player_a, player_b.id()).await;
    assert!(res.is_err());
    let res = call::challenge(&contract, &player_b, player_a.id()).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_resign() -> anyhow::Result<()> {
    let (worker, _, contract, _, iah_contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::add_human(&iah_contract, &player_a, player_a.id()),
        call::add_human(&iah_contract, &player_b, player_b.id())
    )?;
    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );

    let (res, events) = call::resign(&contract, &player_a, &game_id).await?;
    assert_eq!(res, GameOutcome::Victory(Color::Black));
    let expected_board = [
        "RNBQKBNR".to_string(),
        "PPPPPPPP".to_string(),
        "        ".to_string(),
        "        ".to_string(),
        "        ".to_string(),
        "        ".to_string(),
        "pppppppp".to_string(),
        "rnbqkbnr".to_string(),
    ];
    assert_event_emits(
        events,
        vec![
            ChessEvent::ResignGame {
                game_id: game_id.clone(),
                resigner: player_a.id().parse()?,
            },
            ChessEvent::FinishGame {
                game_id: game_id.clone(),
                outcome: GameOutcome::Victory(Color::Black),
                board: expected_board,
            },
        ],
    )?;
    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(game_ids.is_empty());
    let elo = view::get_elo(&contract, player_a.id()).await?.unwrap();
    assert_eq!(elo, 984.);
    let elo = view::get_elo(&contract, player_b.id()).await?.unwrap();
    assert_eq!(elo, 1016.);
    let games = view::recent_finished_games(&contract).await?;
    assert_eq!(games, vec![game_id.clone()]);
    let games = view::finished_games(&contract, player_a.id()).await?;
    assert_eq!(games, vec![game_id.clone()]);
    let games = view::finished_games(&contract, player_b.id()).await?;
    assert_eq!(games, vec![game_id.clone()]);

    Ok(())
}

#[tokio::test]
async fn test_cancel_success() -> anyhow::Result<()> {
    let (worker, _, contract, _, iah_contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::add_human(&iah_contract, &player_a, player_a.id()),
        call::add_human(&iah_contract, &player_b, player_b.id())
    )?;
    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );

    worker.fast_forward(100).await?;

    let (_res, events) = call::cancel(&contract, &player_b, &game_id).await?;
    assert_event_emits(
        events,
        vec![ChessEvent::CancelGame {
            game_id: game_id.clone(),
            cancelled_by: player_b.id().parse()?,
        }],
    )?;
    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(game_ids.is_empty());
    let elo = view::get_elo(&contract, player_a.id()).await?.unwrap();
    assert_eq!(elo, 1000.);
    let elo = view::get_elo(&contract, player_b.id()).await?.unwrap();
    assert_eq!(elo, 1000.);
    let games = view::recent_finished_games(&contract).await?;
    assert!(games.is_empty());
    let games = view::finished_games(&contract, player_a.id()).await?;
    assert!(games.is_empty());
    let games = view::finished_games(&contract, player_b.id()).await?;
    assert!(games.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_cancel_not_enough_blocks() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );

    let res = call::cancel(&contract, &player_a, &game_id).await;
    assert!(res.is_err());
    let res = call::cancel(&contract, &player_b, &game_id).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_cancel_update_last_block_height() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );

    worker.fast_forward(50).await?;
    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    worker.fast_forward(50).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;

    let res = call::cancel(&contract, &player_a, &game_id).await;
    assert!(res.is_err());
    let res = call::cancel(&contract, &player_b, &game_id).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_cancel_check_opponent() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );

    worker.fast_forward(100).await?;

    let res = call::cancel(&contract, &player_a, &game_id).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_no_self_challenge() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;

    call::storage_deposit(&contract, &player_a, None, None).await?;

    let res = call::challenge(&contract, &player_a, player_a.id()).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_max_open_games() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    let res = call::create_ai_game(&contract, &player_a, Difficulty::Easy).await;
    assert!(res.is_err());

    let res = call::accept_challenge(&contract, &player_b, &challenge_id).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_max_open_challenges() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    call::storage_deposit(&contract, &player_a, None, None).await?;
    call::storage_deposit(&contract, &player_b, None, None).await?;
    let mut tasks = vec![];
    for _ in 0..50 {
        tasks.push(worker.dev_create_account());
    }
    let players = try_join_all(tasks).await?;

    let tasks: Vec<_> = players
        .iter()
        .map(|player| call::storage_deposit(&contract, player, None, None))
        .collect();
    try_join_all(tasks).await?;

    for player in players {
        call::challenge(&contract, &player_a, player.id()).await?;
    }

    let res = call::challenge(&contract, &player_a, player_b.id()).await;
    assert!(res.is_err());
    let res = call::challenge(&contract, &player_b, player_a.id()).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_finish_game() -> anyhow::Result<()> {
    let (worker, _, contract, _, iah_contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::add_human(&iah_contract, &player_a, player_a.id()),
        call::add_human(&iah_contract, &player_b, player_b.id())
    )?;
    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
    let ((outcome, board), _, events) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;
    let expected_board = [
        "RNB K NR".to_string(),
        "PPPP PPP".to_string(),
        "        ".to_string(),
        "p B P   ".to_string(),
        "        ".to_string(),
        "        ".to_string(),
        " ppppQpp".to_string(),
        "rnbqkbnr".to_string(),
    ];
    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));
    assert_eq!(board, expected_board);
    assert_event_emits(
        events,
        vec![
            ChessEvent::PlayMove {
                game_id: game_id.clone(),
                color: Color::White,
                mv: "f3 to f7".to_string(),
            },
            ChessEvent::FinishGame {
                game_id: game_id.clone(),
                outcome: GameOutcome::Victory(Color::White),
                board: expected_board,
            },
        ],
    )?;

    let games = view::recent_finished_games(&contract).await?;
    assert_eq!(games, vec![game_id.clone()]);
    let games = view::finished_games(&contract, player_a.id()).await?;
    assert_eq!(games, vec![game_id.clone()]);
    let games = view::finished_games(&contract, player_b.id()).await?;
    assert_eq!(games, vec![game_id.clone()]);
    let games = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(games.is_empty());
    let games = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(games.is_empty());
    let elo = view::get_elo(&contract, player_a.id()).await?.unwrap();
    assert_eq!(elo, 1016.);
    let elo = view::get_elo(&contract, player_b.id()).await?.unwrap();
    assert_eq!(elo, 984.);

    Ok(())
}

#[tokio::test]
async fn test_notify() -> anyhow::Result<()> {
    let (worker, _, contract, social_contract, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    assert_notification(
        &contract,
        &social_contract,
        hashmap! {
            player_b.id().parse()? => vec![ChessNotification::Challenged {
                challenge_id: create_challenge_id(player_a.id(), player_b.id()),
                challenger_id: player_a.id().parse()?,
            }],
        },
    )
    .await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    call::reject_challenge(&contract, &player_b, &challenge_id, false).await?;
    assert_notification(
        &contract,
        &social_contract,
        hashmap! {
            player_a.id().parse()? => vec![ChessNotification::RejectedChallenge {
                challenged_id: player_b.id().parse()?,
            }],
        },
    )
    .await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    assert_notification(
        &contract,
        &social_contract,
        hashmap! {
            player_a.id().parse()? => vec![ChessNotification::AcceptedChallenge {
                game_id: game_id.clone(),
                challenged_id: player_b.id().parse()?,
            }]
        },
    )
    .await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    assert_notification(
        &contract,
        &social_contract,
        hashmap! {
            player_b.id().parse()? => vec![ChessNotification::YourTurn {
                game_id: game_id.clone(),
            }]
        },
    )
    .await?;

    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    assert_notification(
        &contract,
        &social_contract,
        hashmap! {
            player_a.id().parse()? => vec![ChessNotification::YourTurn {
                game_id: game_id.clone(),
            }]
        },
    )
    .await?;

    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;
    assert_notification(
        &contract,
        &social_contract,
        hashmap! {
            player_a.id().parse()? => vec![ChessNotification::Outcome {
                game_id: game_id.clone(),
                outcome: GameOutcome::Victory(Color::White),
            }],
            player_b.id().parse()? => vec![ChessNotification::Outcome {
                game_id: game_id.clone(),
                outcome: GameOutcome::Victory(Color::White),
            }]
        },
    )
    .await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    call::resign(&contract, &player_a, &game_id).await?;
    assert_notification(
        &contract,
        &social_contract,
        hashmap! {
            player_a.id().parse()? => vec![ChessNotification::Outcome {
                game_id: game_id.clone(),
                outcome: GameOutcome::Victory(Color::Black),
            }],
            player_b.id().parse()? => vec![ChessNotification::Outcome {
                game_id: game_id.clone(),
                outcome: GameOutcome::Victory(Color::Black),
            }]
        },
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_no_elo_if_not_human() -> anyhow::Result<()> {
    let (worker, _, contract, _, iah_contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );
    call::resign(&contract, &player_a, &game_id).await?;

    let is_human = view::is_human(&contract, player_a.id()).await?;
    assert!(!is_human);
    let is_human = view::is_human(&contract, player_b.id()).await?;
    assert!(!is_human);
    let elo = view::get_elo(&contract, player_a.id()).await?;
    assert!(elo.is_none());
    let elo = view::get_elo(&contract, player_b.id()).await?;
    assert!(elo.is_none());

    call::add_human(&iah_contract, &player_a, player_a.id()).await?;
    call::update_is_human(&contract, &player_a, player_a.id()).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );
    call::resign(&contract, &player_a, &game_id).await?;

    let is_human = view::is_human(&contract, player_a.id()).await?;
    assert!(is_human);
    let is_human = view::is_human(&contract, player_b.id()).await?;
    assert!(!is_human);
    let elo = view::get_elo(&contract, player_a.id()).await?.unwrap();
    assert_eq!(elo, 1_000.);
    let elo = view::get_elo(&contract, player_b.id()).await?;
    assert!(elo.is_none());

    call::add_human(&iah_contract, &player_b, player_b.id()).await?;
    call::update_is_human(&contract, &player_b, player_b.id()).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone().parse()?,
        Some(player_b.id().clone().parse()?),
    );
    call::resign(&contract, &player_a, &game_id).await?;

    let is_human = view::is_human(&contract, player_a.id()).await?;
    assert!(is_human);
    let is_human = view::is_human(&contract, player_b.id()).await?;
    assert!(is_human);
    let elo = view::get_elo(&contract, player_a.id()).await?.unwrap();
    assert_eq!(elo, 984.);
    let elo = view::get_elo(&contract, player_b.id()).await?.unwrap();
    assert_eq!(elo, 1016.);

    Ok(())
}
