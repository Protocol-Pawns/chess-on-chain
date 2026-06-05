use crate::{bet, util::*};
use chess_engine::Color;
use chess_lib::{create_challenge_id, BetInfo, BetMsg, BetView, ChessEvent, Fees, GameId, GameOutcome};
use maplit::hashmap;
use near_sdk::json_types::U128;
use near_workspaces::{types::NearToken, Account, AccountId, Contract};

#[tokio::test]
async fn test_bet_basic() -> anyhow::Result<()> {
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
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
    let actual = serde_json::to_value(bet_info)?;
    let expected = serde_json::to_value(BetInfo {
        is_locked: false,
        bets: hashmap! {
            test_token.id().clone() => vec![(
                better_a.id().clone(),
                BetView { amount: U128(bet_amount), winner: player_a.id().clone() }
            ), (
                better_b.id().clone(),
                BetView { amount: U128(bet_amount), winner: player_b.id().clone() }
            )]
        },
    })?;
    assert_eq!(actual, expected);

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
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
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
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
    let actual_whitelist = view::get_token_whitelist(&contract).await?;
    assert_eq!(whitelist, actual_whitelist);

    bet!(&better_a, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;
    bet!(&better_b, test_token.id(), contract.id(), bet_amount, player_b => player_a).await?;

    let bet_info = view::get_bet_info(&contract, (player_a.id(), player_b.id())).await?;
    let actual = serde_json::to_value(bet_info)?;
    let expected = serde_json::to_value(BetInfo {
        is_locked: false,
        bets: hashmap! {
            test_token.id().clone() => vec![(
                better_a.id().clone(),
                BetView { amount: U128(bet_amount), winner: player_a.id().clone() }
            ), (
                better_b.id().clone(),
                BetView { amount: U128(bet_amount * 4), winner: player_b.id().clone() }
            )]
        },
    })?;
    assert_eq!(actual, expected);

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
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
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
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
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
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
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
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;
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
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;

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

async fn setup_bet_test() -> anyhow::Result<(Contract, AccountId, AccountId, Account, Account, AccountId)> {
    let (worker, _, contract, _) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let token_id = test_token.id().clone();

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let better = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &better, None, None),
    )?;
    tokio::try_join!(
        call::storage_deposit(&test_token, contract.as_account(), None, Some(NearToken::from_millinear(100))),
        call::storage_deposit(&test_token, &player_a, None, Some(NearToken::from_millinear(100))),
        call::storage_deposit(&test_token, &player_b, None, Some(NearToken::from_millinear(100))),
        call::storage_deposit(&test_token, &better, None, Some(NearToken::from_millinear(100))),
    )?;
    call::mint_tokens(&test_token, better.id(), 10_000_000).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;

    Ok((contract, player_a.id().clone(), player_b.id().clone(), player_a, player_b, token_id))
}

#[tokio::test]
async fn test_bet_winner_must_be_a_player() -> anyhow::Result<()> {
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
        call::storage_deposit(&test_token, contract.as_account(), None, Some(NearToken::from_millinear(100))),
        call::storage_deposit(&test_token, &better, None, Some(NearToken::from_millinear(100))),
    )?;
    call::mint_tokens(&test_token, better.id(), bet_amount).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;

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
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
        call::storage_deposit(&test_token, contract.as_account(), None, Some(NearToken::from_millinear(100))),
        call::storage_deposit(&test_token, &better, None, Some(NearToken::from_millinear(100))),
    )?;
    call::mint_tokens(&test_token, better.id(), 10_000_000).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;

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
    let (worker, _, contract, _) = initialize_contracts(None).await?;
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
        call::storage_deposit(&test_token, contract.as_account(), None, Some(NearToken::from_millinear(100))),
        call::storage_deposit(&test_token, &unregistered, None, Some(NearToken::from_millinear(100))),
    )?;
    call::mint_tokens(&test_token, unregistered.id(), bet_amount).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;

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
    assert!(bet_info.is_err(), "No bet should have been recorded for unregistered bettor");

    Ok(())
}

#[tokio::test]
async fn test_set_fees_exceed_100_percent_fails() -> anyhow::Result<()> {
    let (_, _, contract, _) = initialize_contracts(None).await?;

    let res = call::set_fees(
        &contract,
        contract.as_account(),
        &Fees {
            treasury: 5000,
            royalties: vec![
                ("account1.near".parse()?, 3000),
                ("account2.near".parse()?, 3000),
            ],
        },
    )
    .await;
    assert!(res.is_err(), "Setting fees totaling >100% should fail");

    let current_fees = view::get_fees(&contract).await?;
    assert_eq!(current_fees.treasury, 0, "Fees should not have changed");
    assert!(current_fees.royalties.is_empty(), "Royalties should not have changed");

    Ok(())
}

#[tokio::test]
async fn test_max_open_bets_per_bettor() -> anyhow::Result<()> {
    let (worker, _, contract, _) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "SHITZU", "SHITZU", None, 24).await?;
    let bet_amount = 1_000_000;

    let better = worker.dev_create_account().await?;
    call::storage_deposit(&contract, &better, None, None).await?;
    call::storage_deposit(&test_token, contract.as_account(), None, Some(NearToken::from_millinear(100))).await?;
    call::storage_deposit(&test_token, &better, None, Some(NearToken::from_millinear(100))).await?;
    call::mint_tokens(&test_token, better.id(), bet_amount * 20).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_wager_whitelist(&contract, contract.as_account(), &whitelist).await?;

    let mut players = vec![];
    for i in 0..12 {
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
