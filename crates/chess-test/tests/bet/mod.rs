use crate::{bet, util::*};
use chess_engine::Color;
use chess_lib::{create_challenge_id, BetMsg, ChessEvent, GameId, GameOutcome};
use near_sdk::json_types::U128;
use near_workspaces::{types::NearToken, Account, AccountId, Contract};
use std::collections::HashMap;

#[tokio::test]
async fn test_bet_basic() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
    )?;
    tokio::try_join!(
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
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
        call::mint_tokens(&test_token, better_b.id(), bet_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_token_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    let (_, events) =
        bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    assert_event_emits(
        events,
        vec![ChessEvent::PlaceBet {
            bettor: better_a.id().clone(),
            players: (player_a.id().clone(), player_b.id().clone()),
            token_id: test_token.id().clone(),
            amount: U128(bet_amount),
            winner: player_a.id().clone(),
        }],
    )?;

    let (_, events) =
        bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;
    assert_event_emits(
        events,
        vec![ChessEvent::PlaceBet {
            bettor: better_b.id().clone(),
            players: (player_b.id().clone(), player_a.id().clone()),
            token_id: test_token.id().clone(),
            amount: U128(bet_amount),
            winner: player_b.id().clone(),
        }],
    )?;

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await?;
    assert!(!bet_info.is_locked);
    let token_bets = bet_info.bets.get(test_token.id()).unwrap();
    let bet_map: HashMap<_, _> = token_bets.iter().map(|(id, b)| (id.clone(), b)).collect();
    assert_eq!(bet_map.len(), 2);
    assert_eq!(bet_map[better_a.id()].amount.0, bet_amount);
    assert_eq!(bet_map[better_a.id()].winner, player_a.id().clone());
    assert_eq!(bet_map[better_b.id()].amount.0, bet_amount);
    assert_eq!(bet_map[better_b.id()].winner, player_b.id().clone());

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await?;
    assert!(bet_info.is_locked);

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
    let resolve_events: Vec<_> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                chess_common::ContractEvent::ChessGame(chess_common::ChessEvent {
                    event_kind: chess_common::ChessEventKind::ResolveBets(_),
                    ..
                })
            )
        })
        .collect();
    assert_eq!(
        resolve_events.len(),
        1,
        "Expected exactly 1 ResolveBets event"
    );

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;

    let balance = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance.0, bet_amount * 2);
    let balance = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance.0, 0);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_incomplete_bet() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
    )?;
    tokio::try_join!(
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
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
        call::mint_tokens(&test_token, better_b.id(), bet_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_token_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    call::accept_challenge(&contract, &player_b, &challenge_id).await?;

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await?;
    assert!(bet_info.bets.is_empty());
    assert!(bet_info.is_locked);

    Ok(())
}

#[tokio::test]
async fn test_bet_increase() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
    )?;
    tokio::try_join!(
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
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
        call::mint_tokens(&test_token, better_b.id(), 4 * bet_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_token_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await?;
    assert!(!bet_info.is_locked);
    let token_bets = bet_info.bets.get(test_token.id()).unwrap();
    let bet_map: HashMap<_, _> = token_bets.iter().map(|(id, b)| (id.clone(), b)).collect();
    assert_eq!(bet_map.len(), 2);
    assert_eq!(bet_map[better_a.id()].amount.0, bet_amount);
    assert_eq!(bet_map[better_a.id()].winner, player_a.id().clone());
    assert_eq!(bet_map[better_b.id()].amount.0, bet_amount * 4);
    assert_eq!(bet_map[better_b.id()].winner, player_b.id().clone());

    play_game(&contract, &player_a, &player_b).await?;

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;

    let balance = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance.0, bet_amount * 2);
    let balance = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance.0, bet_amount * 3);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_bet_weighted_win_imbalanced() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;
    let better_c = worker.dev_create_account().await?;
    let better_d = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
        call::storage_deposit(&contract, &better_c, None, None),
        call::storage_deposit(&contract, &better_d, None, None),
    )?;
    tokio::try_join!(
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
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_c,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_d,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), 1_000_000),
        call::mint_tokens(&test_token, better_b.id(), 1_000_000),
        call::mint_tokens(&test_token, better_c.id(), 1_000_000),
        call::mint_tokens(&test_token, better_d.id(), 1_000_000),
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_token_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    bet!(&better_a, test_token.id(), contract.id(), 500_000, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), 500_000, player_a => player_b).await?;
    bet!(&better_c, test_token.id(), contract.id(), 1_000_000, player_a => player_b).await?;
    bet!(&better_d, test_token.id(), contract.id(), 1_000_000, player_b => player_a).await?;

    play_game(&contract, &player_a, &player_b).await?;

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;
    call::withdraw_token(&contract, &better_c, test_token.id()).await?;
    call::withdraw_token(&contract, &better_d, test_token.id()).await?;

    let balance = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance.0, 1_250_000);
    let balance = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance.0, 1_250_000);
    let balance = view::ft_balance_of(&test_token, better_c.id()).await?;
    assert_eq!(balance.0, 1_500_000);
    let balance = view::ft_balance_of(&test_token, better_d.id()).await?;
    assert_eq!(balance.0, 0);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_bet_weighted_win_imbalanced_reverse() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;
    let better_c = worker.dev_create_account().await?;
    let better_d = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
        call::storage_deposit(&contract, &better_c, None, None),
        call::storage_deposit(&contract, &better_d, None, None),
    )?;
    tokio::try_join!(
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
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_c,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_d,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), 1_000_000),
        call::mint_tokens(&test_token, better_b.id(), 1_000_000),
        call::mint_tokens(&test_token, better_c.id(), 1_000_000),
        call::mint_tokens(&test_token, better_d.id(), 1_000_000),
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_token_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    bet!(&better_a, test_token.id(), contract.id(), 500_000, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), 500_000, player_a => player_b).await?;
    bet!(&better_c, test_token.id(), contract.id(), 1_000_000, player_a => player_b).await?;
    bet!(&better_d, test_token.id(), contract.id(), 1_000_000, player_b => player_a).await?;

    play_game(&contract, &player_b, &player_a).await?;

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;
    call::withdraw_token(&contract, &better_c, test_token.id()).await?;
    call::withdraw_token(&contract, &better_d, test_token.id()).await?;

    let balance = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance.0, 750_000);
    let balance = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance.0, 750_000);
    let balance = view::ft_balance_of(&test_token, better_c.id()).await?;
    assert_eq!(balance.0, 500_000);
    let balance = view::ft_balance_of(&test_token, better_d.id()).await?;
    assert_eq!(balance.0, 2_000_000);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_bet_weighted_win_with_refund() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;
    let better_c = worker.dev_create_account().await?;
    let better_d = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
        call::storage_deposit(&contract, &better_c, None, None),
        call::storage_deposit(&contract, &better_d, None, None),
    )?;
    tokio::try_join!(
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
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_c,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_d,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), 1_000_000),
        call::mint_tokens(&test_token, better_b.id(), 1_000_000),
        call::mint_tokens(&test_token, better_c.id(), 1_000_000),
        call::mint_tokens(&test_token, better_d.id(), 1_000_000),
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_token_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    bet!(&better_a, test_token.id(), contract.id(), 100_000, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), 200_000, player_a => player_b).await?;
    bet!(&better_c, test_token.id(), contract.id(), 300_000, player_a => player_b).await?;
    bet!(&better_d, test_token.id(), contract.id(), 1_000_000, player_b => player_a).await?;

    play_game(&contract, &player_a, &player_b).await?;

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;
    call::withdraw_token(&contract, &better_c, test_token.id()).await?;
    call::withdraw_token(&contract, &better_d, test_token.id()).await?;

    let balance = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance.0, 1_100_000);
    let balance = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance.0, 1_200_000);
    let balance = view::ft_balance_of(&test_token, better_c.id()).await?;
    assert_eq!(balance.0, 1_300_000);
    let balance = view::ft_balance_of(&test_token, better_d.id()).await?;
    assert_eq!(balance.0, 400_000);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_bet_weighted_win_with_refund_reverse() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;
    let better_c = worker.dev_create_account().await?;
    let better_d = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
        call::storage_deposit(&contract, &better_c, None, None),
        call::storage_deposit(&contract, &better_d, None, None),
    )?;
    tokio::try_join!(
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
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_c,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_d,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), 1_000_000),
        call::mint_tokens(&test_token, better_b.id(), 1_000_000),
        call::mint_tokens(&test_token, better_c.id(), 1_000_000),
        call::mint_tokens(&test_token, better_d.id(), 1_000_000),
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_token_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    bet!(&better_a, test_token.id(), contract.id(), 100_000, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), 200_000, player_a => player_b).await?;
    bet!(&better_c, test_token.id(), contract.id(), 300_000, player_a => player_b).await?;
    bet!(&better_d, test_token.id(), contract.id(), 1_000_000, player_b => player_a).await?;

    play_game(&contract, &player_b, &player_a).await?;

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;
    call::withdraw_token(&contract, &better_c, test_token.id()).await?;
    call::withdraw_token(&contract, &better_d, test_token.id()).await?;

    let balance = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance.0, 900_000);
    let balance = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance.0, 800_000);
    let balance = view::ft_balance_of(&test_token, better_c.id()).await?;
    assert_eq!(balance.0, 700_000);
    let balance = view::ft_balance_of(&test_token, better_d.id()).await?;
    assert_eq!(balance.0, 1_600_000);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

async fn play_game(contract: &Contract, winner: &Account, looser: &Account) -> anyhow::Result<()> {
    call::challenge(contract, winner, looser.id()).await?;
    let challenge_id = create_challenge_id(winner.id(), looser.id());

    let (game_id, _) = call::accept_challenge(contract, looser, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(block_height, winner.id().clone(), Some(looser.id().clone()));

    call::play_move(contract, winner, &game_id, "e2e4".to_string()).await?;
    call::play_move(contract, looser, &game_id, "a7a6".to_string()).await?;
    call::play_move(contract, winner, &game_id, "d1f3".to_string()).await?;
    call::play_move(contract, looser, &game_id, "a6a5".to_string()).await?;
    call::play_move(contract, winner, &game_id, "f1c4".to_string()).await?;
    call::play_move(contract, looser, &game_id, "a5a4".to_string()).await?;
    call::play_move(contract, winner, &game_id, "f3f7".to_string()).await?;

    Ok(())
}

#[tokio::test]
async fn test_bet_events_lifecycle() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
    )?;
    tokio::try_join!(
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
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
        call::mint_tokens(&test_token, better_b.id(), bet_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    // PlaceBet events
    let (_, events) =
        bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    assert_event_emits(
        events,
        vec![ChessEvent::PlaceBet {
            bettor: better_a.id().clone(),
            players: (player_a.id().clone(), player_b.id().clone()),
            token_id: test_token.id().clone(),
            amount: U128(bet_amount),
            winner: player_a.id().clone(),
        }],
    )?;

    let (_, events) =
        bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;
    assert_event_emits(
        events,
        vec![ChessEvent::PlaceBet {
            bettor: better_b.id().clone(),
            players: (player_b.id().clone(), player_a.id().clone()),
            token_id: test_token.id().clone(),
            amount: U128(bet_amount),
            winner: player_b.id().clone(),
        }],
    )?;

    // Challenge + Accept (LockBets event)
    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());

    let (game_id, events) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    let lock_bets_events: Vec<_> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                chess_common::ContractEvent::ChessGame(chess_common::ChessEvent {
                    event_kind: chess_common::ChessEventKind::LockBets(_),
                    ..
                })
            )
        })
        .collect();
    assert_eq!(
        lock_bets_events.len(),
        1,
        "Expected exactly 1 LockBets event"
    );

    // Play game to completion - ResolveBets event
    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
    let ((outcome, _), _, events) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;
    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));

    let resolve_bets_events: Vec<_> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                chess_common::ContractEvent::ChessGame(chess_common::ChessEvent {
                    event_kind: chess_common::ChessEventKind::ResolveBets(_),
                    ..
                })
            )
        })
        .collect();
    assert_eq!(
        resolve_bets_events.len(),
        1,
        "Expected exactly 1 ResolveBets event"
    );

    Ok(())
}

#[tokio::test]
async fn test_bet_winner_must_be_a_player() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100))
        ),
        call::storage_deposit(
            &test_token,
            &better,
            None,
            Some(NearToken::from_millinear(100))
        ),
    )?;
    call::mint_tokens(&test_token, better.id(), bet_amount).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    call::bet(
        &better,
        test_token.id(),
        contract.id(),
        bet_amount.into(),
        BetMsg {
            players: (player_a.id().clone(), player_b.id().clone()),
            winner: better.id().clone(),
        },
    )
    .await?;

    let balance = view::ft_balance_of(&test_token, better.id()).await?;
    assert_eq!(
        balance.0, bet_amount,
        "Tokens should be refunded after bet with invalid winner"
    );

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await;
    assert!(bet_info.is_err(), "No bet should have been recorded");

    Ok(())
}

#[tokio::test]
async fn test_bet_zero_amount_fails() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100))
        ),
        call::storage_deposit(
            &test_token,
            &better,
            None,
            Some(NearToken::from_millinear(100))
        ),
    )?;
    call::mint_tokens(&test_token, better.id(), 10_000_000).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    let res = call::bet(
        &better,
        test_token.id(),
        contract.id(),
        0.into(),
        BetMsg {
            players: (player_a.id().clone(), player_b.id().clone()),
            winner: player_a.id().clone(),
        },
    )
    .await;
    assert!(res.is_err(), "Betting with zero amount should fail");

    Ok(())
}

#[tokio::test]
async fn test_bet_unregistered_bettor_fails() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let unregistered = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100))
        ),
        call::storage_deposit(
            &test_token,
            &unregistered,
            None,
            Some(NearToken::from_millinear(100))
        ),
    )?;
    call::mint_tokens(&test_token, unregistered.id(), bet_amount).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    call::bet(
        &unregistered,
        test_token.id(),
        contract.id(),
        bet_amount.into(),
        BetMsg {
            players: (player_a.id().clone(), player_b.id().clone()),
            winner: player_a.id().clone(),
        },
    )
    .await?;

    let balance = view::ft_balance_of(&test_token, unregistered.id()).await?;
    assert_eq!(
        balance.0, bet_amount,
        "Tokens should be refunded after bet from unregistered account"
    );

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await;
    assert!(
        bet_info.is_err(),
        "No bet should have been recorded for unregistered bettor"
    );

    Ok(())
}

#[tokio::test]
async fn test_set_fees_exceed_100_percent_fails() -> anyhow::Result<()> {
    let (_, _, contract) = initialize_contracts(None).await?;

    let res = call::set_fees(&contract, contract.as_account(), 10_001).await;
    assert!(res.is_err(), "Setting fees exceeding 100% should fail");

    let current_fees = view::get_fees(&contract).await?;
    assert_eq!(current_fees, 0, "Fees should not have changed");

    Ok(())
}

#[tokio::test]
async fn test_max_open_bets_per_bettor() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 1_000_000;

    let better = worker.dev_create_account().await?;
    call::storage_deposit(&contract, &better, None, None).await?;
    call::storage_deposit(
        &test_token,
        contract.as_account(),
        None,
        Some(NearToken::from_millinear(100)),
    )
    .await?;
    call::storage_deposit(
        &test_token,
        &better,
        None,
        Some(NearToken::from_millinear(100)),
    )
    .await?;
    call::mint_tokens(&test_token, better.id(), bet_amount * 20).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    let mut players = vec![];
    for _ in 0..12 {
        let player_a = worker.dev_create_account().await?;
        let player_b = worker.dev_create_account().await?;
        call::storage_deposit(&contract, &player_a, None, None).await?;
        call::storage_deposit(&contract, &player_b, None, None).await?;
        players.push((player_a, player_b));
    }

    for (player_a, player_b) in players.iter().take(10) {
        call::bet(
            &better,
            test_token.id(),
            contract.id(),
            bet_amount.into(),
            BetMsg {
                players: (player_a.id().clone(), player_b.id().clone()),
                winner: player_a.id().clone(),
            },
        )
        .await?;
    }

    let (player_a, player_b) = &players[10];
    call::bet(
        &better,
        test_token.id(),
        contract.id(),
        bet_amount.into(),
        BetMsg {
            players: (player_a.id().clone(), player_b.id().clone()),
            winner: player_a.id().clone(),
        },
    )
    .await?;

    let balance = view::ft_balance_of(&test_token, better.id()).await?;
    assert_eq!(
        balance.0,
        bet_amount * 10,
        "Tokens should be refunded for bet exceeding max"
    );

    Ok(())
}

#[tokio::test]
async fn test_player_cannot_bet_on_own_game() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
    )?;
    tokio::try_join!(
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
        call::mint_tokens(&test_token, player_a.id(), bet_amount),
        call::mint_tokens(&test_token, player_b.id(), bet_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    let res =
        bet!(&player_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await;
    assert!(res.is_err());

    let res =
        bet!(&player_b, test_token.id(), contract.id(), bet_amount, player_a => player_b).await;
    assert!(res.is_err());

    Ok(())
}

fn sorted_players<'a>(a: &'a Account, b: &'a Account) -> (AccountId, AccountId) {
    if a.id() < b.id() {
        (a.id().clone(), b.id().clone())
    } else {
        (b.id().clone(), a.id().clone())
    }
}

#[tokio::test]
async fn test_cancel_bet_before_game() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better,
            None,
            Some(NearToken::from_millinear(100)),
        ),
    )?;
    call::mint_tokens(&test_token, better.id(), bet_amount).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    bet!(&better, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;

    let balance = view::ft_balance_of(&test_token, better.id()).await?;
    assert_eq!(balance.0, 0);

    let players = sorted_players(&player_a, &player_b);
    call::cancel_bet(&contract, &better, players.clone(), test_token.id()).await?;

    call::withdraw_token(&contract, &better, test_token.id()).await?;
    let balance = view::ft_balance_of(&test_token, better.id()).await?;
    assert_eq!(balance.0, bet_amount);

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await;
    assert!(bet_info.is_err());

    Ok(())
}

#[tokio::test]
async fn test_cancel_bet_locked_fails() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better,
            None,
            Some(NearToken::from_millinear(100)),
        ),
    )?;
    call::mint_tokens(&test_token, better.id(), bet_amount).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    bet!(&better, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    call::accept_challenge(&contract, &player_b, &challenge_id).await?;

    let players = sorted_players(&player_a, &player_b);
    let res = call::cancel_bet(&contract, &better, players, test_token.id()).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_cancel_game_refunds_bettors() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        ),
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
        call::mint_tokens(&test_token, better_b.id(), bet_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;

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

    call::cancel(&contract, &player_b, &game_id).await?;

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;

    let balance_a = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance_a.0, bet_amount);
    let balance_b = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance_b.0, bet_amount);

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await;
    assert!(bet_info.is_err());

    Ok(())
}

#[tokio::test]
async fn test_max_bets_per_game() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 1_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    call::storage_deposit(&contract, &player_a, None, None).await?;
    call::storage_deposit(&contract, &player_b, None, None).await?;
    call::storage_deposit(
        &test_token,
        contract.as_account(),
        None,
        Some(NearToken::from_millinear(100)),
    )
    .await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    let mut bettors = vec![];
    for _ in 0..6 {
        let bettor = worker.dev_create_account().await?;
        call::storage_deposit(&contract, &bettor, None, None).await?;
        call::storage_deposit(
            &test_token,
            &bettor,
            None,
            Some(NearToken::from_millinear(100)),
        )
        .await?;
        call::mint_tokens(&test_token, bettor.id(), bet_amount).await?;
        bettors.push(bettor);
    }

    for bettor in bettors.iter().take(5) {
        call::bet(
            bettor,
            test_token.id(),
            contract.id(),
            bet_amount.into(),
            BetMsg {
                players: (player_a.id().clone(), player_b.id().clone()),
                winner: player_a.id().clone(),
            },
        )
        .await?;
    }

    call::bet(
        &bettors[5],
        test_token.id(),
        contract.id(),
        bet_amount.into(),
        BetMsg {
            players: (player_a.id().clone(), player_b.id().clone()),
            winner: player_a.id().clone(),
        },
    )
    .await?;

    let balance = view::ft_balance_of(&test_token, bettors[5].id()).await?;
    assert_eq!(
        balance.0, bet_amount,
        "6th bettor tokens should be refunded"
    );

    Ok(())
}

#[tokio::test]
async fn test_stalemate_bet_refund() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
        call::mint_tokens(&test_token, better_b.id(), bet_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    play_stalemate_game(&contract, &player_a, &player_b, &game_id).await?;

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;

    let balance = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance.0, bet_amount);
    let balance = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance.0, bet_amount);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_bet_payout_with_fees() -> anyhow::Result<()> {
    let (worker, owner, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 1_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &owner,
            None,
            Some(NearToken::from_millinear(100)),
        ),
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
        call::mint_tokens(&test_token, better_b.id(), bet_amount)
    )?;

    call::set_fees(&contract, &owner, 1000).await?;
    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, &owner, &whitelist).await?;

    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;

    play_game(&contract, &player_a, &player_b).await?;

    let tokens = view::get_tokens(&contract, better_a.id()).await?;
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, test_token.id().clone());
    assert_eq!(tokens[0].1 .0, 1_900_000);
    let amount = view::get_token_amount(&contract, better_a.id(), test_token.id()).await?;
    assert_eq!(amount.0, 1_900_000);

    let tokens = view::get_tokens(&contract, better_b.id()).await?;
    assert!(tokens.is_empty());
    let amount = view::get_token_amount(&contract, better_b.id(), test_token.id()).await?;
    assert_eq!(amount.0, 0);

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;

    let balance = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance.0, 1_900_000);
    let balance = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance.0, 0);

    let treasury_tokens = view::get_treasury_tokens(&contract).await?;
    assert_eq!(treasury_tokens.len(), 1);
    assert_eq!(treasury_tokens[0].0, test_token.id().clone());
    assert_eq!(treasury_tokens[0].1 .0, 100_000);

    Ok(())
}

#[tokio::test]
async fn test_resign_with_active_bets() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 10_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
        call::mint_tokens(&test_token, better_b.id(), bet_amount)
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let block_height = game_id.0;
    let game_id = GameId(
        block_height,
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    let (outcome, _) = call::resign(&contract, &player_b, &game_id).await?;
    assert_eq!(outcome, GameOutcome::Victory(Color::White));

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;

    let balance = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance.0, bet_amount * 2);
    let balance = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance.0, 0);
    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_wager_and_bets_combined() -> anyhow::Result<()> {
    let (worker, owner, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 6).await?;
    let wager_amount = 10_000_000;
    let bet_amount = 1_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
    )?;
    tokio::try_join!(
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
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100)),
        )
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount),
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
        call::mint_tokens(&test_token, better_b.id(), bet_amount)
    )?;

    call::set_fees(&contract, &owner, 1000).await?;
    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, &owner, &whitelist).await?;

    call::challenge_with_wager(
        &player_a,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        chess_lib::ChallengeMsg {
            challenged_id: player_b.id().clone(),
        },
    )
    .await?;

    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;

    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (_res, events) = call::accept_challenge_with_wager(
        &player_b,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        chess_lib::AcceptChallengeMsg { challenge_id },
    )
    .await?;
    let game_id = events
        .iter()
        .find_map(|event| {
            if let chess_common::ContractEvent::ChessGame(chess_common::ChessEvent {
                event_kind:
                    chess_common::ChessEventKind::AcceptChallenge(chess_common::AcceptChallenge {
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
        .clone();

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
    let ((outcome, _), _, _) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;
    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));

    let balance = view::ft_balance_of(&test_token, player_a.id()).await?;
    assert_eq!(balance.0, 18_000_000);
    let balance = view::ft_balance_of(&test_token, player_b.id()).await?;
    assert_eq!(balance.0, 0);

    call::withdraw_token(&contract, &better_a, test_token.id()).await?;
    call::withdraw_token(&contract, &better_b, test_token.id()).await?;

    let balance = view::ft_balance_of(&test_token, better_a.id()).await?;
    assert_eq!(balance.0, 1_900_000);
    let balance = view::ft_balance_of(&test_token, better_b.id()).await?;
    assert_eq!(balance.0, 0);

    let balance = view::ft_balance_of(&test_token, contract.id()).await?;
    assert_eq!(balance.0, 2_100_000);

    let treasury_tokens = view::get_treasury_tokens(&contract).await?;
    assert_eq!(treasury_tokens.len(), 1);
    assert_eq!(treasury_tokens[0].1 .0, 2_100_000);

    Ok(())
}

#[tokio::test]
async fn test_bet_multiple_tokens() -> anyhow::Result<()> {
    let (worker, owner, contract) = initialize_contracts(None).await?;
    let token_a = initialize_token(&worker, "TokenA", "TKA", None, 6).await?;
    let token_b = initialize_token(&worker, "TokenB", "TKB", None, 6).await?;
    let bet_amount = 1_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;
    let better_c = worker.dev_create_account().await?;
    let better_d = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
        call::storage_deposit(&contract, &better_c, None, None),
        call::storage_deposit(&contract, &better_d, None, None),
    )?;

    for token in [&token_a, &token_b] {
        tokio::try_join!(
            call::storage_deposit(
                token,
                contract.as_account(),
                None,
                Some(NearToken::from_millinear(100))
            ),
            call::storage_deposit(token, &better_a, None, Some(NearToken::from_millinear(100))),
            call::storage_deposit(token, &better_b, None, Some(NearToken::from_millinear(100))),
            call::storage_deposit(token, &better_c, None, Some(NearToken::from_millinear(100))),
            call::storage_deposit(token, &better_d, None, Some(NearToken::from_millinear(100))),
        )?;
        tokio::try_join!(
            call::mint_tokens(token, better_a.id(), bet_amount),
            call::mint_tokens(token, better_b.id(), bet_amount),
            call::mint_tokens(token, better_c.id(), bet_amount),
            call::mint_tokens(token, better_d.id(), bet_amount),
        )?;
    }

    call::set_fees(&contract, &owner, 1000).await?;
    let whitelist = vec![token_a.id().clone(), token_b.id().clone()];
    call::set_token_whitelist(&contract, &owner, &whitelist).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;

    bet!(&better_a, token_a.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_b, token_a.id(), contract.id(), bet_amount, player_b => player_a).await?;
    bet!(&better_c, token_b.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_d, token_b.id(), contract.id(), bet_amount, player_b => player_a).await?;

    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
    let ((outcome, _), _, _) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;
    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));

    call::withdraw_token(&contract, &better_a, token_a.id()).await?;
    call::withdraw_token(&contract, &better_b, token_a.id()).await?;
    let balance = view::ft_balance_of(&token_a, better_a.id()).await?;
    assert_eq!(balance.0, 1_900_000);
    let balance = view::ft_balance_of(&token_a, better_b.id()).await?;
    assert_eq!(balance.0, 0);

    call::withdraw_token(&contract, &better_c, token_b.id()).await?;
    call::withdraw_token(&contract, &better_d, token_b.id()).await?;
    let balance = view::ft_balance_of(&token_b, better_c.id()).await?;
    assert_eq!(balance.0, 1_900_000);
    let balance = view::ft_balance_of(&token_b, better_d.id()).await?;
    assert_eq!(balance.0, 0);

    let balance = view::ft_balance_of(&token_a, contract.id()).await?;
    assert_eq!(balance.0, 100_000);
    let balance = view::ft_balance_of(&token_b, contract.id()).await?;
    assert_eq!(balance.0, 100_000);

    let treasury_tokens = view::get_treasury_tokens(&contract).await?;
    assert_eq!(treasury_tokens.len(), 2);
    for (tid, amt) in &treasury_tokens {
        if tid == token_a.id() || tid == token_b.id() {
            assert_eq!(amt.0, 100_000);
        } else {
            panic!("unexpected treasury token: {}", tid);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_cancel_bet_non_owner() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 1_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100))
        ),
        call::storage_deposit(
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100))
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100))
        ),
    )?;
    call::mint_tokens(&test_token, better_a.id(), bet_amount).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    call::accept_challenge(&contract, &player_b, &challenge_id).await?;

    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;

    let res = better_b
        .call(contract.id(), "cancel_bet")
        .args_json(((player_a.id(), player_b.id()), test_token.id()))
        .max_gas()
        .transact()
        .await?;
    let failures = res.receipt_failures();
    assert!(!failures.is_empty());

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await?;
    assert!(!bet_info.is_locked);
    assert_eq!(bet_info.bets.len(), 1);
    let token_bets = bet_info.bets.get(test_token.id()).unwrap();
    assert_eq!(token_bets.len(), 1);
    assert_eq!(token_bets[0].0, better_a.id().clone());
    assert_eq!(token_bets[0].1.amount.0, bet_amount);

    Ok(())
}

#[tokio::test]
async fn test_cancel_bet_wrong_token() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let token_a = initialize_token(&worker, "TokenA", "TKA", None, 6).await?;
    let token_b = initialize_token(&worker, "TokenB", "TKB", None, 6).await?;
    let bet_amount = 1_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &token_a,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100))
        ),
        call::storage_deposit(
            &token_a,
            &better,
            None,
            Some(NearToken::from_millinear(100))
        ),
    )?;
    call::mint_tokens(&token_a, better.id(), bet_amount).await?;

    let whitelist = vec![token_a.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    call::accept_challenge(&contract, &player_b, &challenge_id).await?;

    bet!(&better, token_a.id(), contract.id(), bet_amount, player_a => player_b).await?;

    let res = better
        .call(contract.id(), "cancel_bet")
        .args_json(((player_a.id(), player_b.id()), token_b.id()))
        .max_gas()
        .transact()
        .await?;
    let failures = res.receipt_failures();
    assert!(!failures.is_empty());

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await?;
    assert!(!bet_info.is_locked);
    let token_bets = bet_info.bets.get(token_a.id()).unwrap();
    assert_eq!(token_bets.len(), 1);
    assert_eq!(token_bets[0].1.amount.0, bet_amount);

    Ok(())
}

#[tokio::test]
async fn test_bet_same_players() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 1_000_000;

    let player = worker.dev_create_account().await?;
    let better = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player, None, None),
        call::storage_deposit(&contract, &better, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100))
        ),
        call::storage_deposit(
            &test_token,
            &better,
            None,
            Some(NearToken::from_millinear(100))
        ),
    )?;
    call::mint_tokens(&test_token, better.id(), bet_amount).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    let (res, _) = call::bet(
        &better,
        test_token.id(),
        contract.id(),
        bet_amount.into(),
        BetMsg {
            players: (player.id().clone(), player.id().clone()),
            winner: player.id().clone(),
        },
    )
    .await?;
    assert!(!res.receipt_failures().is_empty());

    let balance = view::ft_balance_of(&test_token, better.id()).await?;
    assert_eq!(balance.0, bet_amount);

    Ok(())
}

#[tokio::test]
async fn test_bet_between_challenge_and_accept() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 1_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better_a = worker.dev_create_account().await?;
    let better_b = worker.dev_create_account().await?;
    let better_late = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
        call::storage_deposit(&contract, &better_b, None, None),
        call::storage_deposit(&contract, &better_late, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100))
        ),
        call::storage_deposit(
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100))
        ),
        call::storage_deposit(
            &test_token,
            &better_b,
            None,
            Some(NearToken::from_millinear(100))
        ),
        call::storage_deposit(
            &test_token,
            &better_late,
            None,
            Some(NearToken::from_millinear(100))
        ),
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
        call::mint_tokens(&test_token, better_b.id(), bet_amount),
        call::mint_tokens(&test_token, better_late.id(), bet_amount),
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;

    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await?;
    assert!(!bet_info.is_locked);

    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (_game_id, events) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;

    let lock_events: Vec<_> = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                chess_common::ContractEvent::ChessGame(chess_common::ChessEvent {
                    event_kind: chess_common::ChessEventKind::LockBets(_),
                    ..
                })
            )
        })
        .collect();
    assert_eq!(lock_events.len(), 1, "Expected exactly 1 LockBets event");

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await?;
    assert!(bet_info.is_locked);

    let (res, _) = call::bet(
        &better_late,
        test_token.id(),
        contract.id(),
        bet_amount.into(),
        BetMsg {
            players: (player_a.id().clone(), player_b.id().clone()),
            winner: player_a.id().clone(),
        },
    )
    .await?;
    assert!(!res.receipt_failures().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_bet_sorted_insertion() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "TEST", "TEST", None, 24).await?;
    let bet_amount = 1_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    let parent = worker.dev_create_account().await?;
    let better_z = parent
        .create_subaccount("z_bettor")
        .initial_balance(NearToken::from_near(5))
        .transact()
        .await?
        .into_result()?;
    let better_m = parent
        .create_subaccount("m_bettor")
        .initial_balance(NearToken::from_near(5))
        .transact()
        .await?
        .into_result()?;
    let better_a = parent
        .create_subaccount("a_bettor")
        .initial_balance(NearToken::from_near(5))
        .transact()
        .await?
        .into_result()?;
    assert!(better_z.id() > better_m.id());
    assert!(better_m.id() > better_a.id());

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better_z, None, None),
        call::storage_deposit(&contract, &better_m, None, None),
        call::storage_deposit(&contract, &better_a, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_z,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_m,
            None,
            Some(NearToken::from_millinear(100)),
        ),
        call::storage_deposit(
            &test_token,
            &better_a,
            None,
            Some(NearToken::from_millinear(100)),
        ),
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, better_z.id(), bet_amount * 2),
        call::mint_tokens(&test_token, better_m.id(), bet_amount),
        call::mint_tokens(&test_token, better_a.id(), bet_amount),
    )?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    // Insert in reverse alphabetical order: z first, then m, then a.
    // Old code (push) produces: [(z, ...), (m, ...), (a, ...)] — unsorted!
    bet!(&better_z, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_m, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;

    // Test 1: Top-up better_z's bet.
    // Old code: binary_search fails to find z → pushes duplicate → 4 entries.
    // Fixed code: binary_search finds z → increments amount → still 3 entries.
    bet!(&better_z, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await?;
    let token_bets = bet_info.bets.get(test_token.id()).unwrap();
    assert_eq!(token_bets.len(), 3, "no duplicate after top-up");
    let z_bet = token_bets
        .iter()
        .find(|(id, _)| id == better_z.id())
        .unwrap();
    assert_eq!(
        z_bet.1.amount.0,
        bet_amount * 2,
        "top-up should increase amount"
    );

    // Test 2: Cancel better_z's bet.
    // Old code: binary_search fails → BetNotFound.
    // Fixed code: binary_search finds z → successfully cancels.
    let players = sorted_players(&player_a, &player_b);
    call::cancel_bet(&contract, &better_z, players, test_token.id()).await?;

    call::withdraw_token(&contract, &better_z, test_token.id()).await?;
    let balance = view::ft_balance_of(&test_token, better_z.id()).await?;
    assert_eq!(
        balance.0,
        bet_amount * 2,
        "refund after cancel should include top-up"
    );

    Ok(())
}

#[tokio::test]
async fn test_bettor_active_bets_multi_token_cancel() -> anyhow::Result<()> {
    let (worker, owner, contract) = initialize_contracts(None).await?;
    let token_a = initialize_token(&worker, "TokenA", "TKA", None, 6).await?;
    let token_b = initialize_token(&worker, "TokenB", "TKB", None, 6).await?;
    let bet_amount = 1_000_000;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better, None, None),
    )?;

    for token in [&token_a, &token_b] {
        tokio::try_join!(
            call::storage_deposit(
                token,
                contract.as_account(),
                None,
                Some(NearToken::from_millinear(100))
            ),
            call::storage_deposit(token, &better, None, Some(NearToken::from_millinear(100))),
        )?;
        call::mint_tokens(token, better.id(), bet_amount * 20).await?;
    }

    let whitelist = vec![token_a.id().clone(), token_b.id().clone()];
    call::set_token_whitelist(&contract, &owner, &whitelist).await?;

    // Phase 1: Bet on game A with 2 tokens.
    // bettor_active_bets = 1 (incremented once per BetId, not per-token).
    bet!(&better, token_a.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better, token_b.id(), contract.id(), bet_amount, player_a => player_b).await?;

    let players_ab = sorted_players(&player_a, &player_b);
    let bet_info = view::get_bet_info(&contract, (&players_ab.0, &players_ab.1)).await?;
    assert_eq!(bet_info.bets.len(), 2, "should have 2 token entries");

    // Phase 2: Cancel ONE token's bet. The bettor still has a bet in the
    // other token, so bettor_active_bets should NOT decrement.
    // BUG: the current code decrements unconditionally, so counter goes to 0.
    call::cancel_bet(&contract, &better, players_ab.clone(), token_a.id()).await?;

    let bet_info = view::get_bet_info(&contract, (&players_ab.0, &players_ab.1)).await?;
    assert_eq!(bet_info.bets.len(), 1, "should still have 1 token entry");

    // Phase 3: The bettor still has an active bet on game A (via token_b).
    // With the bug (counter = 0), they can place bets on 10 MORE games for a
    // total of 11 active BetIds, exceeding MAX_OPEN_BETS.
    // With the fix (counter = 1), they should only be able to place 9 more.
    //
    // We place 9 bets — these should succeed with both buggy and fixed code.
    let mut game_players: Vec<(Account, Account)> = vec![];
    for _ in 0..9 {
        let pa = worker.dev_create_account().await?;
        let pb = worker.dev_create_account().await?;
        call::storage_deposit(&contract, &pa, None, None).await?;
        call::storage_deposit(&contract, &pb, None, None).await?;
        game_players.push((pa, pb));
    }

    for (pa, pb) in &game_players {
        bet!(&better, token_a.id(), contract.id(), bet_amount, pa => pb).await?;
    }

    // Phase 4: The 10th NEW bet (11th BetId total: game A + 9 new + this one)
    // should FAIL because bettor_active_bets should be at MAX_OPEN_BETS = 10.
    // With the bug (counter = 9 after 9 new bets), this SUCCEEDS — test fails.
    // With the fix (counter = 10 = 1 game A + 9 new), this correctly fails.
    let player_extra_a = worker.dev_create_account().await?;
    let player_extra_b = worker.dev_create_account().await?;
    call::storage_deposit(&contract, &player_extra_a, None, None).await?;
    call::storage_deposit(&contract, &player_extra_b, None, None).await?;

    let ft_balance_before = view::ft_balance_of(&token_a, better.id()).await?;
    let _ = bet!(
        &better,
        token_a.id(),
        contract.id(),
        bet_amount,
        player_extra_a => player_extra_b
    )
    .await;
    let ft_balance_after = view::ft_balance_of(&token_a, better.id()).await?;

    assert_eq!(
        ft_balance_before, ft_balance_after,
        "10th bet should be refunded — MAX_OPEN_BETS reached"
    );

    Ok(())
}
