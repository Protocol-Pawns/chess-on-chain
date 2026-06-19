use crate::util::*;
use chess_common::{ChessEvent as ChessEventCommon, ChessEventKind, ContractEvent};
use chess_lib::{ChessEvent, GameId, MatchmakingMsg, Player};
use near_workspaces::{network::Sandbox, types::NearToken, Account, Contract, Worker};

fn extract_game_id_from_events(events: &[ContractEvent]) -> GameId {
    events
        .iter()
        .find_map(|event| match event {
            ContractEvent::ChessGame(ChessEventCommon {
                event_kind: ChessEventKind::CreateGame(data),
                ..
            }) => Some(data.game_id.clone()),
            _ => None,
        })
        .unwrap()
}

fn default_board() -> [String; 8] {
    [
        "RNBQKBNR".to_string(),
        "PPPPPPPP".to_string(),
        "        ".to_string(),
        "        ".to_string(),
        "        ".to_string(),
        "        ".to_string(),
        "pppppppp".to_string(),
        "rnbqkbnr".to_string(),
    ]
}

async fn setup_players(
    worker: &Worker<Sandbox>,
    contract: &Contract,
    count: usize,
) -> anyhow::Result<Vec<Account>> {
    let mut accounts = Vec::with_capacity(count);
    for _ in 0..count {
        accounts.push(worker.dev_create_account().await?);
    }
    for a in &accounts {
        call::storage_deposit(contract, a, None, None).await?;
    }
    Ok(accounts)
}

#[tokio::test]
async fn test_matchmaking_queue_then_match() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let [player_a, player_b] = setup_players(&worker, &contract, 2)
        .await?
        .try_into()
        .ok()
        .unwrap();

    // A joins with a wide range -> queued (None).
    let (res, _events) = call::join_matchmaking(&contract, &player_a, 0.0, 2_000.0).await?;
    assert!(res.is_none());
    let entry = view::is_queued(&contract, player_a.id()).await?;
    assert!(entry.is_some());

    // B joins with an overlapping range -> matched immediately (Some).
    let (res, events) = call::join_matchmaking(&contract, &player_b, 0.0, 2_000.0).await?;
    let game_id = res.expect("should have matched");
    assert_event_emits(
        events,
        vec![ChessEvent::CreateGame {
            game_id: game_id.clone(),
            white: Player::Human(player_a.id().clone()),
            black: Player::Human(player_b.id().clone()),
            board: default_board(),
        }],
    )?;

    // Neither player is queued anymore.
    let entry = view::is_queued(&contract, player_a.id()).await?;
    assert!(entry.is_none());
    let entry = view::is_queued(&contract, player_b.id()).await?;
    assert!(entry.is_none());

    // Both players have the game and correct colors (queued=White, joiner=Black).
    let info = view::get_game_info(&contract, &game_id).await?;
    assert_eq!(info.white, Player::Human(player_a.id().clone()));
    assert_eq!(info.black, Player::Human(player_b.id().clone()));
    let a_games = view::get_game_ids(&contract, player_a.id()).await?;
    assert!(a_games.contains(&game_id));
    let b_games = view::get_game_ids(&contract, player_b.id()).await?;
    assert!(b_games.contains(&game_id));

    Ok(())
}

#[tokio::test]
async fn test_matchmaking_elo_no_overlap() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let [player_a, player_b] = setup_players(&worker, &contract, 2)
        .await?
        .try_into()
        .ok()
        .unwrap();

    // A only wants high-rated opponents (1500+); both start at 1000.
    let (res, _events) = call::join_matchmaking(&contract, &player_a, 1_500.0, 3_000.0).await?;
    assert!(res.is_none());

    // B (elo 1000) is outside A's window -> no match, B is queued separately.
    let (res, _events) = call::join_matchmaking(&contract, &player_b, 0.0, 1_400.0).await?;
    assert!(res.is_none());
    let entry = view::is_queued(&contract, player_a.id()).await?;
    assert!(entry.is_some());
    let entry = view::is_queued(&contract, player_b.id()).await?;
    assert!(entry.is_some());

    let queue = view::get_matchmaking_queue(&contract, None, None).await?;
    assert_eq!(queue.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_matchmaking_already_queued() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let [player_a] = setup_players(&worker, &contract, 1)
        .await?
        .try_into()
        .ok()
        .unwrap();

    let (res, _events) = call::join_matchmaking(&contract, &player_a, 0.0, 2_000.0).await?;
    assert!(res.is_none());

    // Joining again must fail.
    let res = call::join_matchmaking(&contract, &player_a, 0.0, 2_000.0).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_cancel_matchmaking() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let [player_a] = setup_players(&worker, &contract, 1)
        .await?
        .try_into()
        .ok()
        .unwrap();

    let (res, _events) = call::join_matchmaking(&contract, &player_a, 0.0, 2_000.0).await?;
    assert!(res.is_none());

    call::cancel_matchmaking(&contract, &player_a).await?;
    let entry = view::is_queued(&contract, player_a.id()).await?;
    assert!(entry.is_none());

    // Cancelling again must fail.
    let res = call::cancel_matchmaking(&contract, &player_a).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_matchmaking_does_not_count_as_challenge() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let [player_a, player_b] = setup_players(&worker, &contract, 2)
        .await?
        .try_into()
        .ok()
        .unwrap();

    call::join_matchmaking(&contract, &player_a, 0.0, 2_000.0).await?;
    call::join_matchmaking(&contract, &player_b, 0.0, 2_000.0).await?;

    // A matchmaking match must not increment challenges_sent.
    let a = view::get_account(&contract, player_a.id()).await?;
    assert_eq!(a.challenges_sent, 0);
    let b = view::get_account(&contract, player_b.id()).await?;
    assert_eq!(b.challenges_sent, 0);

    Ok(())
}

#[tokio::test]
async fn test_matchmaking_wager_match() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "wrapped Near", "wNEAR", None, 24).await?;
    let wager_amount = 10_000_000_000_000_000_000_000_000u128; // 10 NEAR
    let [player_a, player_b] = setup_players(&worker, &contract, 2)
        .await?
        .try_into()
        .ok()
        .unwrap();

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
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), wager_amount),
        call::mint_tokens(&test_token, player_b.id(), wager_amount),
    )?;
    call::set_token_whitelist(&contract, contract.as_account(), &[test_token.id().clone()]).await?;

    // A joins with a wager -> queued.
    let (_res, _events) = call::join_matchmaking_with_wager(
        &player_a,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        MatchmakingMsg {
            min_elo: 0.0,
            max_elo: 2_000.0,
        },
    )
    .await?;
    let entry = view::is_queued(&contract, player_a.id()).await?;
    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.wager.as_ref().unwrap().0, *test_token.id());
    assert_eq!(entry.wager.as_ref().unwrap().1 .0, wager_amount);

    // B joins with the identical wager -> matched.
    let (_res, events) = call::join_matchmaking_with_wager(
        &player_b,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        MatchmakingMsg {
            min_elo: 0.0,
            max_elo: 2_000.0,
        },
    )
    .await?;
    let game_id = extract_game_id_from_events(&events);
    let entry = view::is_queued(&contract, player_b.id()).await?;
    assert!(entry.is_none());

    let info = view::get_game_info(&contract, &game_id).await?;
    assert_eq!(info.white, Player::Human(player_a.id().clone()));
    assert_eq!(info.black, Player::Human(player_b.id().clone()));

    Ok(())
}

#[tokio::test]
async fn test_matchmaking_wager_mismatch_no_match() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "wrapped Near", "wNEAR", None, 24).await?;
    let [player_a, player_b] = setup_players(&worker, &contract, 2)
        .await?
        .try_into()
        .ok()
        .unwrap();

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
    )?;
    tokio::try_join!(
        call::mint_tokens(&test_token, player_a.id(), 100),
        call::mint_tokens(&test_token, player_b.id(), 100),
    )?;
    call::set_token_whitelist(&contract, contract.as_account(), &[test_token.id().clone()]).await?;

    // A queues with amount 50.
    call::join_matchmaking_with_wager(
        &player_a,
        test_token.id(),
        contract.id(),
        50u128.into(),
        MatchmakingMsg {
            min_elo: 0.0,
            max_elo: 2_000.0,
        },
    )
    .await?;

    // B joins with a different amount (100) -> no match, B queued.
    let (_res, _events) = call::join_matchmaking_with_wager(
        &player_b,
        test_token.id(),
        contract.id(),
        100u128.into(),
        MatchmakingMsg {
            min_elo: 0.0,
            max_elo: 2_000.0,
        },
    )
    .await?;
    let entry = view::is_queued(&contract, player_b.id()).await?;
    assert!(entry.is_some());
    let entry = view::is_queued(&contract, player_a.id()).await?;
    assert!(entry.is_some(), "A should still be queued");

    Ok(())
}

#[tokio::test]
async fn test_cancel_matchmaking_wager_refund() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "wrapped Near", "wNEAR", None, 24).await?;
    let wager_amount = 5_000_000_000_000_000_000_000_000u128; // 5 NEAR
    let [player_a] = setup_players(&worker, &contract, 1)
        .await?
        .try_into()
        .ok()
        .unwrap();

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
    )?;
    call::mint_tokens(&test_token, player_a.id(), wager_amount).await?;
    call::set_token_whitelist(&contract, contract.as_account(), &[test_token.id().clone()]).await?;

    // Token balance before = wager_amount.
    let before = view::ft_balance_of(&test_token, player_a.id()).await?.0;

    call::join_matchmaking_with_wager(
        &player_a,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        MatchmakingMsg {
            min_elo: 0.0,
            max_elo: 2_000.0,
        },
    )
    .await?;

    // Tokens were transferred to the contract.
    let mid = view::ft_balance_of(&test_token, player_a.id()).await?.0;
    assert_eq!(mid, before - wager_amount);

    // Cancel -> wager refunded via ft_transfer.
    call::cancel_matchmaking(&contract, &player_a).await?;
    let after = view::ft_balance_of(&test_token, player_a.id()).await?.0;
    assert_eq!(after, before);
    let entry = view::is_queued(&contract, player_a.id()).await?;
    assert!(entry.is_none());

    Ok(())
}

#[tokio::test]
async fn test_matchmaking_expired_entry_purged_and_refunded() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;
    let test_token = initialize_token(&worker, "wrapped Near", "wNEAR", None, 24).await?;
    let wager_amount = 5_000_000_000_000_000_000_000_000u128; // 5 NEAR
    let [player_a, player_b] = setup_players(&worker, &contract, 2)
        .await?
        .try_into()
        .ok()
        .unwrap();

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
    )?;
    call::mint_tokens(&test_token, player_a.id(), wager_amount).await?;
    call::set_token_whitelist(&contract, contract.as_account(), &[test_token.id().clone()]).await?;

    // A joins with a wager -> queued.
    call::join_matchmaking_with_wager(
        &player_a,
        test_token.id(),
        contract.id(),
        wager_amount.into(),
        MatchmakingMsg {
            min_elo: 0.0,
            max_elo: 2_000.0,
        },
    )
    .await?;

    let entry = view::is_queued(&contract, player_a.id()).await?;
    assert!(entry.is_some(), "A should be queued");

    // FT balance dropped — wager is in escrow.
    let bal_after_join = view::ft_balance_of(&test_token, player_a.id()).await?.0;
    assert_eq!(bal_after_join, 0, "wager tokens should be held by contract");

    // Wait for the entry to expire (sandbox timestamps are wall-clock based).
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // B joins — this triggers lazy purge of A's expired entry.
    // B has no wager so cannot match A anyway, but A is expired regardless.
    let (res, _events) = call::join_matchmaking(&contract, &player_b, 0.0, 2_000.0).await?;
    assert!(
        res.is_none(),
        "B should be queued (no match with expired A)"
    );

    // A's expired entry was purged.
    let entry = view::is_queued(&contract, player_a.id()).await?;
    assert!(entry.is_none(), "A should have been purged from the queue");

    // B is queued.
    let entry = view::is_queued(&contract, player_b.id()).await?;
    assert!(entry.is_some(), "B should be queued");

    // A's wager was credited back to their contract-escrow balance.
    let escrow = view::get_tokens(&contract, player_a.id()).await?;
    let refunded = escrow
        .iter()
        .find(|(id, _)| id == test_token.id())
        .map(|(_, amt)| amt.0)
        .unwrap_or(0);
    assert_eq!(
        refunded, wager_amount,
        "expired wager should be credited back to A's escrow balance"
    );

    // A withdraws — FT balance restored.
    call::withdraw_token(&contract, &player_a, test_token.id()).await?;
    let bal_after_withdraw = view::ft_balance_of(&test_token, player_a.id()).await?.0;
    assert_eq!(
        bal_after_withdraw, wager_amount,
        "A should have the wager back in their FT balance after withdrawal"
    );

    Ok(())
}
