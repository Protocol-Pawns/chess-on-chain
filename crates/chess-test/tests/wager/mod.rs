use crate::util::*;
use chess_engine::Color;
use chess_lib::{
    create_challenge_id, AcceptChallengeMsg, Challenge, ChallengeMsg, ChessEvent, Fees, GameId,
    GameOutcome, Player,
};
use near_workspaces::types::NearToken;

#[tokio::test]
async fn test_accept_challenge_success() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;
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
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_wager_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

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
    let game_id = get_game_id(&events);
    let block_height = game_id.0;
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

    Ok(())
}

#[tokio::test]
async fn test_accept_challenge_not_enough_wager() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;
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
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_wager_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    call::challenge_with_wager(
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

    call::accept_challenge_with_wager(
        &player_b,
        test_token.id(),
        contract.id(),
        (wager_amount - 1).into(),
        AcceptChallengeMsg { challenge_id },
    )
    .await?;

    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(game_ids.is_empty());

    let balance = view::ft_balance_of(&test_token, player_b.id()).await?;
    assert_eq!(balance.0, wager_amount);

    Ok(())
}

#[tokio::test]
async fn test_accept_challenge_refund_too_much_wager() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;
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
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount + 1)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_wager_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    call::challenge_with_wager(
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

    let (_res, events) = call::accept_challenge_with_wager(
        &player_b,
        test_token.id(),
        contract.id(),
        (wager_amount + 1).into(),
        AcceptChallengeMsg { challenge_id },
    )
    .await?;
    let game_id = get_game_id(&events);
    let block_height = game_id.0;
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

    let balance = view::ft_balance_of(&test_token, player_b.id()).await?;
    assert_eq!(balance.0, 1);

    Ok(())
}

#[tokio::test]
async fn test_reject_challenge_refund_wager() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;
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
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_wager_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    call::challenge_with_wager(
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

    let balance = view::ft_balance_of(&test_token, player_a.id()).await?;
    assert_eq!(balance.0, wager_amount);
    let balance = view::ft_balance_of(&test_token, player_b.id()).await?;
    assert_eq!(balance.0, wager_amount);

    Ok(())
}

#[tokio::test]
async fn test_reject_wager_no_whitelist() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;
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
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount)
    )?;

    let (res, _) = call::challenge_with_wager(
        &player_a,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        ChallengeMsg {
            challenged_id: player_b.id().parse()?,
        },
    )
    .await?;
    assert!(!res.receipt_failures().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_reject_wager_wrong_token() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "wrapped Near", "wNEAR", None, 6).await?;
    let wrong_test_token =
        initialize_token(&worker, "HarryPotterObamaSonicInu", "BITCOIN", None, 6).await?;
    let wager_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &wrong_test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &wrong_test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &wrong_test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&wrong_test_token, player_b.id(), wager_amount)
    )?;

    let whitelist = vec![test_token.id().clone(), wrong_test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_wager_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    call::challenge_with_wager(
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

    let (res, _) = call::accept_challenge_with_wager(
        &player_b,
        wrong_test_token.id(),
        contract.id(),
        wager_amount.into(),
        AcceptChallengeMsg { challenge_id },
    )
    .await?;
    assert!(!res.receipt_failures().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_cancel_game_refund_wager() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;
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
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_wager_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    call::challenge_with_wager(
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

    let (_res, events) = call::accept_challenge_with_wager(
        &player_b,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        AcceptChallengeMsg { challenge_id },
    )
    .await?;
    let game_id = get_game_id(&events);

    let balance = view::ft_balance_of(&test_token, player_a.id()).await?;
    assert_eq!(balance.0, 0);
    let balance = view::ft_balance_of(&test_token, player_b.id()).await?;
    assert_eq!(balance.0, 0);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 2 * wager_amount);

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
    let balance = view::ft_balance_of(&test_token, player_a.id()).await?;
    assert_eq!(balance.0, wager_amount);
    let balance = view::ft_balance_of(&test_token, player_b.id()).await?;
    assert_eq!(balance.0, wager_amount);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_finish_game_payout_wager() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;
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
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_wager_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    call::challenge_with_wager(
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

    let (_res, events) = call::accept_challenge_with_wager(
        &player_b,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        AcceptChallengeMsg { challenge_id },
    )
    .await?;
    let game_id = get_game_id(&events);

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

    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(game_ids.is_empty());
    let balance = view::ft_balance_of(&test_token, player_a.id()).await?;
    assert_eq!(balance.0, 2 * wager_amount);
    let balance = view::ft_balance_of(&test_token, player_b.id()).await?;
    assert_eq!(balance.0, 0);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_finish_game_payout_fees() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "wrapped Near", "wNEAR", None, 6).await?;
    let wager_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let royalty_account_a = worker.dev_create_account().await?;
    let royalty_account_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        ),
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount)
    )?;

    let fees = Fees {
        treasury: 900,
        royalties: vec![
            (royalty_account_a.id().parse()?, 70),
            (royalty_account_b.id().parse()?, 30),
        ],
    };
    call::set_fees(&contract, contract.as_account(), &fees).await?;
    let actual_fees = view::get_fees(&contract).await?;
    assert_eq!(fees, actual_fees);

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_wager_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    let amount = NearToken::from_millinear(50);
    call::register_token(
        &contract,
        contract.as_account(),
        test_token.id(),
        amount.as_yoctonear().into(),
        NearToken::from_yoctonear(amount.as_yoctonear() * 3),
    )
    .await?;

    call::challenge_with_wager(
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

    let (_res, events) = call::accept_challenge_with_wager(
        &player_b,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        AcceptChallengeMsg { challenge_id },
    )
    .await?;
    let game_id = get_game_id(&events);

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

    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(game_ids.is_empty());
    let balance = view::ft_balance_of(&test_token, player_a.id()).await?;
    assert_eq!(balance.0, 19_000_000);
    let balance = view::ft_balance_of(&test_token, player_b.id()).await?;
    assert_eq!(balance.0, 0);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 900_000);
    let balance = view::ft_balance_of(&test_token, royalty_account_a.id()).await?;
    assert_eq!(balance.0, 70_000);
    let balance = view::ft_balance_of(&test_token, royalty_account_b.id()).await?;
    assert_eq!(balance.0, 30_000);

    Ok(())
}

#[tokio::test]
async fn test_resign_payout_wager() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;
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
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &player_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_wager_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    call::challenge_with_wager(
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

    let (_res, events) = call::accept_challenge_with_wager(
        &player_b,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        AcceptChallengeMsg { challenge_id },
    )
    .await?;
    let game_id = get_game_id(&events);

    let (res, events) = call::resign(&contract, &player_b, &game_id).await?;

    assert_eq!(res, GameOutcome::Victory(Color::White));
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
                resigner: player_b.id().parse()?,
            },
            ChessEvent::FinishGame {
                game_id: game_id.clone(),
                outcome: GameOutcome::Victory(Color::White),
                board: expected_board,
            },
        ],
    )?;

    let game_ids = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(game_ids.is_empty());
    let game_ids = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(game_ids.is_empty());
    let balance = view::ft_balance_of(&test_token, player_a.id()).await?;
    assert_eq!(balance.0, 2 * wager_amount);
    let balance = view::ft_balance_of(&test_token, player_b.id()).await?;
    assert_eq!(balance.0, 0);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

fn get_game_id(events: &[event::ContractEvent]) -> GameId {
    events
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
        .unwrap()
        .clone()
}

// TODO stalemate refund wager
