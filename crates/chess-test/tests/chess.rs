mod util;

use chess_engine::Color;
use chess_lib::{
    create_challenge_id, AcceptChallengeMsg, Challenge, ChallengeMsg, ChessEvent,
    ChessNotification, ChessNotificationItem, Difficulty, GameId, GameOutcome, Notification,
    Player,
};
use futures::future::try_join_all;
use tokio::fs;
use util::*;

#[tokio::test]
async fn test_migrate() -> anyhow::Result<()> {
    let (worker, _, contract, _) = initialize_contracts(Some("../../res/chess_old.wasm")).await?;

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

    call::create_ai_game_old(&contract, &player_c, Difficulty::Easy).await?;

    contract
        .as_account()
        .deploy(&fs::read("../../res/chess.wasm").await?)
        .await?
        .into_result()?;
    call::migrate(&contract, contract.as_account()).await?;

    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert_eq!(game_ids, vec![game_id.clone()]);
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert_eq!(game_ids, vec![game_id.clone()]);
    let ai_game_ids = view::get_game_ids(&contract, player_c.id()).await?;
    assert!(!ai_game_ids.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_ai_game() -> anyhow::Result<()> {
    let (worker, _, contract, _) = initialize_contracts(None).await?;

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

    let (_res, events) = call::resign(&contract, &player_a, &game_id).await?;
    assert_event_emits(
        events,
        vec![ChessEvent::ResignGame {
            game_id,
            resigner: player_a.id().parse()?,
        }],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_accept_challenge() -> anyhow::Result<()> {
    let (worker, _, contract, _) = initialize_contracts(None).await?;

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
        ],
    )?;

    let (_res, events) = call::resign(&contract, &player_a, &game_id).await?;
    assert_event_emits(
        events,
        vec![ChessEvent::ResignGame {
            game_id,
            resigner: player_a.id().parse()?,
        }],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_accept_challenge_with_wager() -> anyhow::Result<()> {
    let (worker, _, contract, _) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "wrapped Near", "wNEAR", None, 24).await?;
    let wager_amount = 10_000_000_000_000_000_000_000_000; // 10 NEAR

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(100_000_000_000_000_000_000_000),
        ),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(100_000_000_000_000_000_000_000),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(100_000_000_000_000_000_000_000),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount)
    )?;

    let (_res, events) = call::challenge_with_wager(
        &player_a,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        ChallengeMsg {
            challenged_id: player_b.id().parse()?,
        },
    )
    .await?;
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
    let expected_challenge = Challenge::new(
        player_a.id().parse()?,
        player_b.id().parse()?,
        Some((test_token.id().parse()?, wager_amount.into())),
    );
    assert_eq!(&challenge, &expected_challenge);
    assert_event_emits(events, vec![ChessEvent::Challenge(expected_challenge)])?;

    let (_res, events) = call::accept_challenge_with_wager(
        &player_b,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        AcceptChallengeMsg { challenge_id },
    )
    .await?;
    let game_id = events
        .iter()
        .find_map(|event| {
            if let event::ContractEvent::ChessGame(event::ChessEvent {
                event_kind:
                    event::ChessEventKind::AcceptChallenge(event::AcceptChallengeEventData {
                        game_id,
                        ..
                    }),
                ..
            }) = event
            {
                Some(game_id)
            } else {
                None
            }
        })
        .unwrap();
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
    assert_eq!(
        game_ids,
        vec![GameId(
            block_height,
            player_a.id().clone().parse()?,
            Some(player_b.id().clone().parse()?)
        )]
    );
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert_eq!(
        game_ids,
        vec![GameId(
            block_height,
            player_a.id().clone().parse()?,
            Some(player_b.id().clone().parse()?)
        )]
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
        ],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_reject_challenge() -> anyhow::Result<()> {
    let (worker, _, contract, _) = initialize_contracts(None).await?;

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
    let (worker, _, contract, _) = initialize_contracts(None).await?;

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
    let (worker, _, contract, _) = initialize_contracts(None).await?;

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
async fn test_no_self_challenge() -> anyhow::Result<()> {
    let (worker, _, contract, _) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;

    call::storage_deposit(&contract, &player_a, None, None).await?;

    let res = call::challenge(&contract, &player_a, player_a.id()).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_max_open_games() -> anyhow::Result<()> {
    let (worker, _, contract, _) = initialize_contracts(None).await?;

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
    let (worker, _, contract, _) = initialize_contracts(None).await?;

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
    let (worker, _, contract, _) = initialize_contracts(None).await?;

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

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
    let ((outcome, board), events) =
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
    let elo = view::get_elo(&contract, player_a.id()).await?;
    assert_eq!(elo, 1016.);
    let elo = view::get_elo(&contract, player_b.id()).await?;
    assert_eq!(elo, 984.);

    Ok(())
}

#[tokio::test]
async fn test_notify() -> anyhow::Result<()> {
    let (worker, _, contract, social_contract) = initialize_contracts(None).await?;

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
        vec![Notification {
            key: player_b.id().parse()?,
            value: ChessNotification {
                _type: "chess-game".to_string(),
                item: ChessNotificationItem::Challenged {
                    challenger_id: player_a.id().parse()?,
                },
            },
        }],
    )
    .await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    call::reject_challenge(&contract, &player_b, &challenge_id, false).await?;
    assert_notification(
        &contract,
        &social_contract,
        vec![Notification {
            key: player_a.id().parse()?,
            value: ChessNotification {
                _type: "chess-game".to_string(),
                item: ChessNotificationItem::RejectedChallenge {
                    challenged_id: player_b.id().parse()?,
                },
            },
        }],
    )
    .await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    assert_notification(
        &contract,
        &social_contract,
        vec![Notification {
            key: player_a.id().parse()?,
            value: ChessNotification {
                _type: "chess-game".to_string(),
                item: ChessNotificationItem::AcceptedChallenge {
                    challenged_id: player_b.id().parse()?,
                },
            },
        }],
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
        vec![Notification {
            key: player_b.id().parse()?,
            value: ChessNotification {
                _type: "chess-game".to_string(),
                item: ChessNotificationItem::YourTurn {
                    game_id: game_id.clone(),
                },
            },
        }],
    )
    .await?;

    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    assert_notification(
        &contract,
        &social_contract,
        vec![Notification {
            key: player_a.id().parse()?,
            value: ChessNotification {
                _type: "chess-game".to_string(),
                item: ChessNotificationItem::YourTurn {
                    game_id: game_id.clone(),
                },
            },
        }],
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
        vec![
            Notification {
                key: player_a.id().parse()?,
                value: ChessNotification {
                    _type: "chess-game".to_string(),
                    item: ChessNotificationItem::Outcome {
                        game_id: game_id.clone(),
                        outcome: GameOutcome::Victory(Color::White),
                    },
                },
            },
            Notification {
                key: player_b.id().parse()?,
                value: ChessNotification {
                    _type: "chess-game".to_string(),
                    item: ChessNotificationItem::Outcome {
                        game_id,
                        outcome: GameOutcome::Victory(Color::White),
                    },
                },
            },
        ],
    )
    .await?;

    Ok(())
}
