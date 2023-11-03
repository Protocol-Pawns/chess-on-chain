use crate::util::*;
use chess_lib::{
    create_challenge_id, AcceptChallengeMsg, Challenge, ChallengeMsg, ChessEvent, GameId, Player,
};

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
        call::mint_tokens(&test_token, player_b.id(), wager_amount + 1)
    )?;

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

// refund wager if cancelled game

// pay out wager on win

// treasury
