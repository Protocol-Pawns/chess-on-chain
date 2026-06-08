mod bet;
mod points;
mod util;
mod wager;

#[cfg(feature = "gas-test")]
mod gas;

use base64::Engine;
use chess_common::ContractEvent;
use chess_engine::Color;
use chess_lib::{
    create_challenge_id, Challenge, ChessEvent, Difficulty, GameId, GameInfo, GameOutcome, Player,
    MAX_OPEN_CHALLENGES, MAX_OPEN_GAMES,
};
use futures::future::try_join_all;
use near_workspaces::types::{KeyType, SecretKey};
use tokio::fs;
use util::*;

#[tokio::test]
async fn test_migrate() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;

    let key = SecretKey::from_random(KeyType::ED25519);
    let nada_bot_contract = worker
        .create_tla_and_deploy(
            "nada-bot.registrar".parse()?,
            key,
            &fs::read("../../res/nada_bot_stub.wasm").await?,
        )
        .await?
        .into_result()?;
    nada_bot_contract
        .call("new")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let key = SecretKey::from_random(KeyType::ED25519);
    let contract = worker
        .create_tla_and_deploy(
            "chess.registrar".parse()?,
            key,
            &fs::read("../../res/chess_mainnet.wasm").await?,
        )
        .await?
        .into_result()?;
    contract
        .call("new")
        .args_json(("social.near", nada_bot_contract.id()))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let player_c = worker.dev_create_account().await?;

    tokio::try_join!(
        call::add_human(&nada_bot_contract, &player_a, player_a.id()),
        call::add_human(&nada_bot_contract, &player_b, player_b.id()),
        call::add_human(&nada_bot_contract, &player_c, player_c.id()),
    )?;
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
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    call::create_ai_game(&contract, &player_c, Difficulty::Easy).await?;

    contract
        .as_account()
        .deploy(&fs::read("../../res/chess_testing.wasm").await?)
        .await?
        .into_result()?;
    call::migrate(&contract, contract.as_account(), contract.id()).await?;

    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert_eq!(game_ids, vec![game_id.clone()]);
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert_eq!(game_ids, vec![game_id.clone()]);
    let ai_game_ids = view::get_game_ids(&contract, player_c.id()).await?;
    assert!(!ai_game_ids.is_empty());
    let game_info = view::get_game_info(&contract, &game_id).await?;
    let mut actual = serde_json::to_value(game_info)?;
    actual["last_block_height"].take();
    let mut expected = serde_json::to_value(GameInfo {
        white: Player::Human(player_a.id().clone()),
        black: Player::Human(player_b.id().clone()),
        turn_color: Color::White,
        last_block_height: block_height,
        has_bets: false,
    })?;
    expected["last_block_height"].take();
    assert_eq!(actual, expected);

    Ok(())
}

#[tokio::test]
async fn test_pausing() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    let res = call::pause(&contract, &player_a).await;
    assert!(res.is_err());

    call::pause(&contract, contract.as_account()).await?;

    let res = call::storage_deposit(&contract, &player_a, None, None).await;
    assert!(res.is_err());
    let res = call::challenge(&contract, &player_a, player_b.id()).await;
    assert!(res.is_err());
    let res = call::accept_challenge(&contract, &player_a, &"id".to_string()).await;
    assert!(res.is_err());
    let res = call::create_ai_game(&contract, &player_a, Difficulty::Easy).await;
    assert!(res.is_err());
    let res = call::play_move(
        &contract,
        &player_a,
        &GameId(0, "a.near".parse()?, None),
        "a2a4".to_string(),
    )
    .await;
    assert!(res.is_err());

    call::resume(&contract, contract.as_account()).await?;
    let res = call::storage_deposit(&contract, &player_a, None, None).await;
    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_ai_game() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;

    call::storage_deposit(&contract, &player_a, None, None).await?;
    let (game_id, events) = call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    let block_height = game_id.0;
    let game_id = GameId(block_height, player_a.id().clone(), None);
    assert_event_emits(
        events,
        vec![ChessEvent::CreateGame {
            game_id: game_id.clone(),
            white: Player::Human(player_a.id().clone()),
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
                outcome: None,
            },
            ChessEvent::PlayMove {
                game_id: game_id.clone(),
                color: Color::Black,
                mv: "g8 to f6".to_string(),
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
                outcome: None,
            },
        ],
    )?;

    let (res, events) = call::resign(&contract, &player_a, &game_id).await?;
    assert_eq!(res, GameOutcome::Victory(Color::Black));
    assert_event_emits(
        events,
        vec![ChessEvent::ResignGame {
            game_id: game_id.clone(),
            resigner: Color::White,
            outcome: GameOutcome::Victory(Color::Black),
        }],
    )?;
    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_accept_challenge() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

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
    let expected_challenge = Challenge::new(player_a.id().clone(), player_b.id().clone(), None);
    assert_eq!(&challenge, &expected_challenge);
    assert_event_emits(events, vec![ChessEvent::Challenge(expected_challenge)])?;

    let (game_id, events) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone(),
        Some(player_b.id().clone()),
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
                white: Player::Human(player_a.id().clone()),
                black: Player::Human(player_b.id().clone()),
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
        vec![ChessEvent::PlayMove {
            game_id: game_id.clone(),
            color: Color::White,
            mv: "e2 to e4".to_string(),
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
            outcome: None,
        }],
    )?;

    Ok(())
}
#[tokio::test]
async fn test_reject_challenge() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

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
            player_a.id().clone(),
            player_b.id().clone(),
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
    let (worker, _, contract) = initialize_contracts(None).await?;

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
    let expected_challenge = Challenge::new(player_a.id().clone(), player_b.id().clone(), None);
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
    let (worker, _, contract) = initialize_contracts(None).await?;

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
    let (worker, _, contract) = initialize_contracts(None).await?;

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
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    let (res, events) = call::resign(&contract, &player_a, &game_id).await?;
    assert_eq!(res, GameOutcome::Victory(Color::Black));
    assert_event_emits(
        events,
        vec![ChessEvent::ResignGame {
            game_id: game_id.clone(),
            resigner: Color::White,
            outcome: GameOutcome::Victory(Color::Black),
        }],
    )?;
    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(game_ids.is_empty());
    let account = view::get_account(&contract, player_a.id()).await?;
    assert_eq!(account.elo.unwrap(), 984.);
    let account = view::get_account(&contract, player_b.id()).await?;
    assert_eq!(account.elo.unwrap(), 1016.);

    Ok(())
}

#[tokio::test]
async fn test_cancel_success() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

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
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    worker.fast_forward(100).await?;

    let (_res, events) = call::cancel(&contract, &player_b, &game_id).await?;
    assert_event_emits(
        events,
        vec![ChessEvent::CancelGame {
            game_id: game_id.clone(),
            cancelled_by: player_b.id().clone(),
        }],
    )?;
    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(game_ids.is_empty());
    let account = view::get_account(&contract, player_a.id()).await?;
    assert_eq!(account.elo.unwrap(), 1000.);
    let account = view::get_account(&contract, player_b.id()).await?;
    assert_eq!(account.elo.unwrap(), 1000.);

    Ok(())
}

#[tokio::test]
async fn test_cancel_not_enough_blocks() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

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
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    let res = call::cancel(&contract, &player_a, &game_id).await;
    assert!(res.is_err());
    let res = call::cancel(&contract, &player_b, &game_id).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_cancel_update_last_block_height() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

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
        player_a.id().clone(),
        Some(player_b.id().clone()),
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
    let (worker, _, contract) = initialize_contracts(None).await?;

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
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    worker.fast_forward(100).await?;

    let res = call::cancel(&contract, &player_a, &game_id).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_no_self_challenge() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;

    call::storage_deposit(&contract, &player_a, None, None).await?;

    let res = call::challenge(&contract, &player_a, player_a.id()).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_max_open_games() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    for _ in 0..MAX_OPEN_GAMES {
        call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    }
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
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    call::storage_deposit(&contract, &player_a, None, None).await?;
    call::storage_deposit(&contract, &player_b, None, None).await?;
    let mut tasks = vec![];
    for _ in 0..MAX_OPEN_CHALLENGES {
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
    let (worker, _, contract) = initialize_contracts(None).await?;

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
        player_a.id().clone(),
        Some(player_b.id().clone()),
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
        vec![ChessEvent::PlayMove {
            game_id: game_id.clone(),
            color: Color::White,
            mv: "f3 to f7".to_string(),
            board: expected_board,
            outcome: Some(GameOutcome::Victory(Color::White)),
        }],
    )?;

    let games = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(games.is_empty());
    let games = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(games.is_empty());
    let account = view::get_account(&contract, player_a.id()).await?;
    assert_eq!(account.elo.unwrap(), 1016.);
    let account = view::get_account(&contract, player_b.id()).await?;
    assert_eq!(account.elo.unwrap(), 984.);

    Ok(())
}

#[tokio::test]
async fn test_set_is_agent_toggle() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let player_a = worker.dev_create_account().await?;

    call::storage_deposit(&contract, &player_a, None, None).await?;
    let account = view::get_account(&contract, player_a.id()).await?;
    assert!(!account.is_agent);

    call::set_is_agent(&contract, &player_a, true).await?;
    let account = view::get_account(&contract, player_a.id()).await?;
    assert!(account.is_agent);

    call::set_is_agent(&contract, &player_a, false).await?;
    let account = view::get_account(&contract, player_a.id()).await?;
    assert!(!account.is_agent);

    Ok(())
}

#[tokio::test]
async fn test_set_is_agent_requires_one_yocto() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let player_a = worker.dev_create_account().await?;

    call::storage_deposit(&contract, &player_a, None, None).await?;
    let res = call::set_is_agent_no_deposit(&contract, &player_a, true).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_mainnet_migration() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;

    let mainnet_wasm = fs::read("../../res/chess_mainnet.wasm").await?;
    let new_wasm = fs::read("../../res/chess_testing.wasm").await?;
    let state_json = fs::read_to_string("../../res/mainnet_state.json").await?;
    let state_entries: Vec<(Vec<u8>, Vec<u8>)> =
        serde_json::from_str::<Vec<serde_json::Value>>(&state_json)?
            .into_iter()
            .map(|entry| {
                let k = base64::engine::general_purpose::STANDARD
                    .decode(entry["key"].as_str().unwrap())
                    .unwrap();
                let v = base64::engine::general_purpose::STANDARD
                    .decode(entry["value"].as_str().unwrap())
                    .unwrap();
                (k, v)
            })
            .collect();

    let contract_id: near_sdk::AccountId = "app.chess-game.near".parse()?;

    let key = SecretKey::from_random(KeyType::ED25519);
    worker
        .patch(&contract_id)
        .account(
            near_workspaces::AccountDetailsPatch::default()
                .balance(near_workspaces::types::NearToken::from_near(100)),
        )
        .access_key(key.public_key(), near_workspaces::AccessKey::full_access())
        .code(&mainnet_wasm)
        .states(
            state_entries
                .iter()
                .map(|(k, v)| (k.as_slice(), v.as_slice())),
        )
        .transact()
        .await?;

    let contract = near_workspaces::Contract::from_secret_key(contract_id.clone(), key, &worker);

    let account_ids: Vec<String> = contract
        .call("get_accounts")
        .args_json((None::<usize>, None::<usize>))
        .max_gas()
        .view()
        .await?
        .json()?;

    contract
        .as_account()
        .deploy(&new_wasm)
        .await?
        .into_result()?;

    call::migrate(&contract, contract.as_account(), contract.id()).await?;

    let account = view::get_account(&contract, &"marior.near".parse()?).await?;
    assert!(account.elo.is_some());
    assert!(!account.is_agent);

    let account_ids_after: Vec<String> = contract
        .call("get_accounts")
        .args_json((None::<usize>, None::<usize>))
        .max_gas()
        .view()
        .await?
        .json()?;
    assert_eq!(account_ids, account_ids_after);

    Ok(())
}

fn count_piece(board: &[String; 8], piece: char) -> usize {
    board
        .iter()
        .map(|row| row.chars().filter(|&c| c == piece).count())
        .sum()
}

fn assert_valid_board(board: &[String; 8]) {
    let white_kings = count_piece(board, 'K');
    let black_kings = count_piece(board, 'k');
    assert_eq!(
        white_kings, 1,
        "board must have exactly 1 white king: {:?}",
        board
    );
    assert_eq!(
        black_kings, 1,
        "board must have exactly 1 black king: {:?}",
        board
    );
}

fn generate_white_move_attempts(board: &[String; 8]) -> Vec<String> {
    let mut moves = Vec::new();
    let priority_pieces: &[char] = &['P', 'N', 'B', 'R', 'Q', 'K'];
    let cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
    for &piece_ch in priority_pieces {
        for row in 0..8u8 {
            for col in 0..8u8 {
                let ch = board[row as usize].as_bytes()[col as usize] as char;
                if ch != piece_ch {
                    continue;
                }
                let deltas: &[(i8, i8)] = if piece_ch == 'P' {
                    &[(0, 1), (0, 2), (-1, 1), (1, 1)]
                } else if piece_ch == 'N' {
                    &[
                        (-2, -1),
                        (-2, 1),
                        (-1, -2),
                        (-1, 2),
                        (1, -2),
                        (1, 2),
                        (2, -1),
                        (2, 1),
                    ]
                } else {
                    &[]
                };
                for &(dc, dr) in deltas {
                    let nc = col as i8 + dc;
                    let nr = row as i8 + dr;
                    if nc >= 0 && nc < 8 && nr >= 0 && nr < 8 {
                        let target = board[nr as usize].as_bytes()[nc as usize] as char;
                        if target == ' ' || target.is_ascii_lowercase() {
                            let from = format!("{}{}", cols[col as usize], row + 1);
                            let to = format!("{}{}", cols[nc as usize], nr + 1);
                            moves.push(format!("{}{}", from, to));
                        }
                    }
                }
            }
        }
    }
    moves
}

#[tokio::test]
async fn test_ai_move_board_state_consistency() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let player_a = worker.dev_create_account().await?;
    call::storage_deposit(&contract, &player_a, None, None).await?;

    let (game_id, _) = call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    let block_height = game_id.0;
    let game_id = GameId(block_height, player_a.id().clone(), None);

    let mut tested_moves = 0u32;
    let max_moves = 10;

    while tested_moves < max_moves {
        let current_board = view::get_board(&contract, &game_id).await?;
        let candidates = generate_white_move_attempts(&current_board);
        if candidates.is_empty() {
            break;
        }

        let mut played_this_turn = false;
        for mv in candidates {
            let result = call::play_move(&contract, &player_a, &game_id, mv.clone()).await;
            let ((outcome, returned_board), _, events) = match result {
                Ok(r) => r,
                Err(_) => continue,
            };
            played_this_turn = true;
            tested_moves += 1;

            for event in &events {
                if let ContractEvent::ChessGame(ce) = event {
                    if let chess_common::ChessEventKind::PlayMove(pm) = &ce.event_kind {
                        assert_valid_board(&pm.board);
                    }
                }
            }

            let ai_event_board = events.iter().rev().find_map(|e| {
                if let ContractEvent::ChessGame(ce) = e {
                    if let chess_common::ChessEventKind::PlayMove(pm) = &ce.event_kind {
                        if pm.color == Color::Black {
                            return Some(pm.board.clone());
                        }
                    }
                }
                None
            });
            if let Some(event_board) = ai_event_board {
                assert_valid_board(&event_board);
                assert_eq!(
                    returned_board, event_board,
                    "returned board must match AI event board"
                );
            }

            if outcome.is_some() {
                return Ok(());
            }
            break;
        }

        if !played_this_turn {
            break;
        }
    }

    assert!(
        tested_moves >= 3,
        "should have tested at least 3 moves, got {}",
        tested_moves
    );

    Ok(())
}
