use crate::bet;
use crate::util::*;
use chess_engine::Color;
use chess_lib::{create_challenge_id, Achievement, BetMsg, Difficulty, GameId, GameOutcome, Quest};
use near_contract_standards::fungible_token::events::FtMint;
use near_workspaces::types::NearToken;

#[tokio::test]
async fn test_daily_play_move() -> anyhow::Result<()> {
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

    let (_, _, events) =
        call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    let points = view::ft_balance_of(&contract, player_a.id()).await?;
    assert_eq!(
        points.0,
        Quest::DailyPlayMove.get_points(false)
            + Achievement::FirstChallenge.get_points()
            + Quest::WeeklyChallenger.get_points(false)
    );
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(
        supply.0,
        Quest::DailyPlayMove.get_points(false)
            + Achievement::FirstChallenge.get_points()
            + Quest::WeeklyChallenger.get_points(false)
    );
    let cooldowns = view::get_quest_cooldowns(&contract, player_a.id()).await?;
    assert!(
        cooldowns.iter().any(|(_, q)| q == &Quest::DailyPlayMove),
        "should have DailyPlayMove cooldown"
    );
    assert!(
        cooldowns.iter().any(|(_, q)| q == &Quest::WeeklyChallenger),
        "should have WeeklyChallenger cooldown"
    );
    assert_ft_mint_events(
        events,
        vec![FtMint {
            owner_id: player_a.id(),
            amount: Quest::DailyPlayMove.get_points(false).into(),
            memo: Some("DailyPlayMove"),
        }],
    )?;

    let (_, _, events) =
        call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    let points = view::ft_balance_of(&contract, player_b.id()).await?;
    assert_eq!(points.0, Quest::DailyPlayMove.get_points(false));
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(
        supply.0,
        Quest::DailyPlayMove.get_points(false) * 2
            + Achievement::FirstChallenge.get_points()
            + Quest::WeeklyChallenger.get_points(false)
    );
    assert_ft_mint_events(
        events,
        vec![FtMint {
            owner_id: player_b.id(),
            amount: Quest::DailyPlayMove.get_points(false).into(),
            memo: Some("DailyPlayMove"),
        }],
    )?;

    let (_, _, events) =
        call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    let points = view::ft_balance_of(&contract, player_a.id()).await?;
    assert_eq!(
        points.0,
        Quest::DailyPlayMove.get_points(false)
            + Quest::DailyPlayMove.get_points(true)
            + Achievement::FirstChallenge.get_points()
            + Quest::WeeklyChallenger.get_points(false)
    );
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(
        supply.0,
        Quest::DailyPlayMove.get_points(false) * 2
            + Quest::DailyPlayMove.get_points(true)
            + Achievement::FirstChallenge.get_points()
            + Quest::WeeklyChallenger.get_points(false)
    );
    assert_ft_mint_events(
        events,
        vec![FtMint {
            owner_id: player_a.id(),
            amount: Quest::DailyPlayMove.get_points(true).into(),
            memo: Some("DailyPlayMove"),
        }],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_daily_game_on_checkmate() -> anyhow::Result<()> {
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
    let game_id = GameId(
        game_id.0,
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;

    let points_a_before = view::ft_balance_of(&contract, player_a.id()).await?.0;
    let points_b_before = view::ft_balance_of(&contract, player_b.id()).await?.0;
    let ((outcome, _), _, _) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;

    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));
    let points_a_after = view::ft_balance_of(&contract, player_a.id()).await?.0;
    let points_b_after = view::ft_balance_of(&contract, player_b.id()).await?.0;

    assert!(
        points_a_after - points_a_before >= Quest::DailyGame.get_points(false),
        "player_a should get DailyGame on checkmate"
    );
    assert!(
        points_b_after - points_b_before >= Quest::DailyGame.get_points(false),
        "player_b should get DailyGame on checkmate"
    );

    Ok(())
}

#[tokio::test]
async fn test_daily_game_no_resign_early() -> anyhow::Result<()> {
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
    let game_id = GameId(
        game_id.0,
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;

    let (outcome, _) = call::resign(&contract, &player_b, &game_id).await?;
    assert_eq!(outcome, GameOutcome::Victory(Color::White));

    let cooldowns_a = view::get_quest_cooldowns(&contract, player_a.id()).await?;
    let cooldowns_b = view::get_quest_cooldowns(&contract, player_b.id()).await?;
    assert!(
        !cooldowns_a.iter().any(|(_, q)| q == &Quest::DailyGame),
        "player_a should NOT have DailyGame cooldown on early resign"
    );
    assert!(
        !cooldowns_b.iter().any(|(_, q)| q == &Quest::DailyGame),
        "player_b should NOT have DailyGame cooldown on early resign"
    );

    Ok(())
}

#[tokio::test]
async fn test_daily_game_resign_late() -> anyhow::Result<()> {
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
    let game_id = GameId(
        game_id.0,
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "e1e2".to_string()).await?;

    let points_a_before = view::ft_balance_of(&contract, player_a.id()).await?.0;
    let points_b_before = view::ft_balance_of(&contract, player_b.id()).await?.0;
    let (outcome, _) = call::resign(&contract, &player_b, &game_id).await?;
    assert_eq!(outcome, GameOutcome::Victory(Color::White));
    let points_a_after = view::ft_balance_of(&contract, player_a.id()).await?.0;
    let points_b_after = view::ft_balance_of(&contract, player_b.id()).await?.0;

    assert!(
        points_a_after - points_a_before >= Quest::DailyGame.get_points(false),
        "player_a SHOULD get DailyGame on late resign (>= 5 moves)"
    );
    assert!(
        points_b_after - points_b_before >= Quest::DailyGame.get_points(false),
        "player_b SHOULD get DailyGame on late resign (>= 5 moves)"
    );

    Ok(())
}

#[tokio::test]
async fn test_first_win_human() -> anyhow::Result<()> {
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
    let ((outcome, _), block_hash, events) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;

    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));

    let points_a = view::ft_balance_of(&contract, player_a.id()).await?;
    let challenge_pts =
        Achievement::FirstChallenge.get_points() + Quest::WeeklyChallenger.get_points(false);
    assert_eq!(
        points_a.0,
        Quest::DailyPlayMove.get_points(false)
            + Quest::DailyPlayMove.get_points(true) * 3
            + challenge_pts
            + Achievement::FirstWin.get_points()
            + Quest::WeeklyWin.get_points(false)
            + Quest::DailyGame.get_points(false)
    );
    let points_b = view::ft_balance_of(&contract, player_b.id()).await?;
    assert_eq!(
        points_b.0,
        Quest::DailyPlayMove.get_points(false)
            + Quest::DailyPlayMove.get_points(true) * 2
            + Quest::DailyGame.get_points(false)
    );
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(supply.0, points_a.0 + points_b.0);

    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    let block = worker.view_block().block_hash(block_hash).await?;
    let ts = block.timestamp() / 1_000_000;
    assert!(achievements.contains(&(ts, Achievement::FirstWin)));
    assert!(achievements
        .iter()
        .any(|(_, a)| a == &Achievement::FirstChallenge));
    assert_ft_mint_events(
        events,
        vec![
            FtMint {
                owner_id: player_a.id(),
                amount: Quest::DailyPlayMove.get_points(true).into(),
                memo: Some("DailyPlayMove"),
            },
            FtMint {
                owner_id: player_a.id(),
                amount: Achievement::FirstWin.get_points().into(),
                memo: Some("FirstWin"),
            },
            FtMint {
                owner_id: player_a.id(),
                amount: Quest::WeeklyWin.get_points(false).into(),
                memo: Some("WeeklyWin"),
            },
            FtMint {
                owner_id: player_a.id(),
                amount: Quest::DailyGame.get_points(false).into(),
                memo: Some("DailyGame"),
            },
            FtMint {
                owner_id: player_b.id(),
                amount: Quest::DailyGame.get_points(false).into(),
                memo: Some("DailyGame"),
            },
        ],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_first_win_ai() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;

    tokio::try_join!(call::storage_deposit(&contract, &player_a, None, None),)?;

    let (game_id, _) = call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    let block_height = game_id.0;
    let game_id = GameId(block_height, player_a.id().clone(), None);

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    let ((outcome, _), block_hash, events) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;

    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));
    let points = view::ft_balance_of(&contract, player_a.id()).await?;
    let expected = Quest::DailyPlayMove.get_points(false)
        + Quest::DailyPlayMove.get_points(true) * 3
        + Achievement::FirstWinAiEasy.get_points()
        + Quest::DailyGame.get_points(false);
    assert_eq!(points.0, expected);
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(supply.0, expected);
    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    let block = worker.view_block().block_hash(block_hash).await?;
    assert_eq!(
        achievements,
        vec![(block.timestamp() / 1_000_000, Achievement::FirstWinAiEasy)]
    );
    assert_ft_mint_events(
        events,
        vec![
            FtMint {
                owner_id: player_a.id(),
                amount: Quest::DailyPlayMove.get_points(true).into(),
                memo: Some("DailyPlayMove"),
            },
            FtMint {
                owner_id: player_a.id(),
                amount: Achievement::FirstWinAiEasy.get_points().into(),
                memo: Some("FirstWinAiEasy"),
            },
            FtMint {
                owner_id: player_a.id(),
                amount: Quest::DailyGame.get_points(false).into(),
                memo: Some("DailyGame"),
            },
        ],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_achievement_only_once() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;

    tokio::try_join!(call::storage_deposit(&contract, &player_a, None, None),)?;

    let (game_id, _) = call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    let block_height = game_id.0;
    let game_id = GameId(block_height, player_a.id().clone(), None);

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    let ((outcome, _), block_hash, _) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;

    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));
    let block = worker.view_block().block_hash(block_hash).await?;

    worker.fast_forward(100).await?;

    let (game_id, _) = call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    let block_height = game_id.0;
    let game_id = GameId(block_height, player_a.id().clone(), None);

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    let ((outcome, _), _, events) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;

    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));
    let points = view::ft_balance_of(&contract, player_a.id()).await?;
    let expected = Quest::DailyPlayMove.get_points(false) * 2
        + Quest::DailyPlayMove.get_points(true) * 6
        + Achievement::FirstWinAiEasy.get_points()
        + Quest::DailyGame.get_points(false) * 2;
    assert_eq!(points.0, expected);
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(supply.0, expected);
    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    assert_eq!(
        achievements,
        vec![(block.timestamp() / 1_000_000, Achievement::FirstWinAiEasy)]
    );
    assert_ft_mint_events(
        events,
        vec![
            FtMint {
                owner_id: player_a.id(),
                amount: Quest::DailyPlayMove.get_points(true).into(),
                memo: Some("DailyPlayMove"),
            },
            FtMint {
                owner_id: player_a.id(),
                amount: Quest::DailyGame.get_points(false).into(),
                memo: Some("DailyGame"),
            },
        ],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_multiple_achievements() -> anyhow::Result<()> {
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
    let ((outcome, _), block_hash, _) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;

    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));
    let block = worker.view_block().block_hash(block_hash).await?;
    let pvp_achievement = (block.timestamp() / 1_000_000, Achievement::FirstWin);

    worker.fast_forward(100).await?;

    let (game_id, _) = call::create_ai_game(&contract, &player_a, Difficulty::Easy).await?;
    let block_height = game_id.0;
    let game_id = GameId(block_height, player_a.id().clone(), None);

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    let ((outcome, _), block_hash, _) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;

    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));
    let block = worker.view_block().block_hash(block_hash).await?;
    let ai_achievement = (block.timestamp() / 1_000_000, Achievement::FirstWinAiEasy);
    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    assert!(achievements.contains(&pvp_achievement));
    assert!(achievements.contains(&ai_achievement));

    let points_a = view::ft_balance_of(&contract, player_a.id()).await?.0;
    let points_b = view::ft_balance_of(&contract, player_b.id()).await?.0;
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(supply.0, points_a + points_b);

    Ok(())
}

#[tokio::test]
async fn test_first_challenge_achievement() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    let (_, events) = call::challenge(&contract, &player_a, player_b.id()).await?;

    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    assert_eq!(achievements.len(), 1);
    assert_eq!(achievements[0].1, Achievement::FirstChallenge);

    let cooldowns = view::get_quest_cooldowns(&contract, player_a.id()).await?;
    assert!(cooldowns.iter().any(|(_, q)| q == &Quest::WeeklyChallenger));

    let points = view::ft_balance_of(&contract, player_a.id()).await?;
    assert_eq!(
        points.0,
        Achievement::FirstChallenge.get_points() + Quest::WeeklyChallenger.get_points(false)
    );
    assert_ft_mint_events(
        events,
        vec![
            FtMint {
                owner_id: player_a.id(),
                amount: Quest::WeeklyChallenger.get_points(false).into(),
                memo: Some("WeeklyChallenger"),
            },
            FtMint {
                owner_id: player_a.id(),
                amount: Achievement::FirstChallenge.get_points().into(),
                memo: Some("FirstChallenge"),
            },
        ],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_win_streak_tracking() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    assert!(achievements.is_empty());

    for _ in 0..3 {
        worker.fast_forward(100).await?;
        call::challenge(&contract, &player_a, player_b.id()).await?;
        let challenge_id = create_challenge_id(player_a.id(), player_b.id());
        let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
        let game_id = GameId(
            game_id.0,
            player_a.id().clone(),
            Some(player_b.id().clone()),
        );

        call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
        call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
        call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
        call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
        call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
        call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
        let ((outcome, _), _, _) =
            call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;
        assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));
    }

    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    assert!(achievements
        .iter()
        .any(|(_, a)| a == &Achievement::FirstWin));
    assert!(achievements
        .iter()
        .any(|(_, a)| a == &Achievement::WinStreak3));

    Ok(())
}

#[tokio::test]
async fn test_elo_achievement() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    let account = view::get_account(&contract, player_a.id()).await?;
    assert_eq!(account.elo, Some(1000.0));

    for _ in 0..10 {
        worker.fast_forward(100).await?;
        call::challenge(&contract, &player_a, player_b.id()).await?;
        let challenge_id = create_challenge_id(player_a.id(), player_b.id());
        let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
        let game_id = GameId(
            game_id.0,
            player_a.id().clone(),
            Some(player_b.id().clone()),
        );

        call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
        call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
        call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
        call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
        call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
        call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
        let ((outcome, _), _, _) =
            call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;
        assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));
    }

    let account = view::get_account(&contract, player_a.id()).await?;
    assert!(account.elo.unwrap() >= 1100.0);

    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    assert!(achievements.iter().any(|(_, a)| a == &Achievement::Elo1100));

    Ok(())
}

#[tokio::test]
async fn test_player_points_immediate() -> anyhow::Result<()> {
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
    let game_id = GameId(
        game_id.0,
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;

    let account_before = view::get_account(&contract, player_a.id()).await?;
    let points_before = account_before.points.0;

    call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;

    let account_after = view::get_account(&contract, player_a.id()).await?;
    assert!(account_after.points.0 > points_before);
    assert_eq!(account_after.pending_points.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_bettor_points_deferred() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;
    let bettor = worker.dev_create_account().await?;

    let test_token = initialize_token(&worker, "USDC", "USDC", None, 24).await?;
    let bet_amount: u128 = 10_000_000_000_000_000_000_000;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None),
        call::storage_deposit(&contract, &bettor, None, None)
    )?;
    tokio::try_join!(
        call::storage_deposit(&test_token, &bettor, None, None),
        call::storage_deposit(
            &test_token,
            contract.as_account(),
            None,
            Some(NearToken::from_millinear(100))
        ),
    )?;

    call::mint_tokens(&test_token, bettor.id(), bet_amount).await?;

    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    call::accept_challenge(&contract, &player_b, &challenge_id).await?;

    let whitelist = vec![test_token.id().clone()];
    call::set_token_whitelist(&contract, contract.as_account(), &whitelist).await?;

    bet!(&bettor, test_token.id(), contract.id(), bet_amount, player_a => player_b).await?;

    let account = view::get_account(&contract, bettor.id()).await?;
    assert!(
        account.pending_points.0 > 0,
        "bettor should have pending points after placing bet"
    );
    assert_eq!(
        account.points.0, 0,
        "bettor should have no claimed points yet"
    );

    let ft_balance = view::ft_balance_of(&contract, bettor.id()).await?;
    assert_eq!(
        ft_balance.0, 0,
        "ft_balance_of should not include deferred points"
    );

    call::claim_points(&contract, &bettor).await?;

    let account = view::get_account(&contract, bettor.id()).await?;
    assert!(account.points.0 > 0, "points should be claimed now");
    assert_eq!(
        account.pending_points.0, 0,
        "pending should be zero after claim"
    );

    let ft_balance = view::ft_balance_of(&contract, bettor.id()).await?;
    assert_eq!(
        ft_balance.0, account.points.0,
        "ft_balance_of should match claimed points"
    );

    let points_after_first_claim = account.points.0;
    call::claim_points(&contract, &bettor).await?;
    let account = view::get_account(&contract, bettor.id()).await?;
    assert_eq!(
        account.points.0, points_after_first_claim,
        "double claim should be no-op"
    );

    Ok(())
}

#[tokio::test]
async fn test_ft_balance_of_unregistered_account() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let stranger = worker.dev_create_account().await?;

    let balance = view::ft_balance_of(&contract, stranger.id()).await?;
    assert_eq!(
        balance.0, 0,
        "ft_balance_of should return 0 for unregistered accounts"
    );

    Ok(())
}

#[tokio::test]
async fn test_weekly_win_cooldown_not_wiped_by_daily_cleanup() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &player_a, None, None),
        call::storage_deposit(&contract, &player_b, None, None)
    )?;

    // Game 1: Player A wins -> WeeklyWin cooldown starts
    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let game_id = GameId(
        game_id.0,
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a7a6".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "d1f3".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
    call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;

    let cooldowns = view::get_quest_cooldowns(&contract, player_a.id()).await?;
    assert!(
        cooldowns.iter().any(|(_, q)| q == &Quest::WeeklyWin),
        "WeeklyWin should be on cooldown after win"
    );

    // Game 2: Start new game, play a move -> DailyPlayMove cooldown starts
    call::challenge(&contract, &player_a, player_b.id()).await?;
    let challenge_id = create_challenge_id(player_a.id(), player_b.id());
    let (game_id, _) = call::accept_challenge(&contract, &player_b, &challenge_id).await?;
    let game_id = GameId(
        game_id.0,
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    // call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;

    // let cooldowns = view::get_quest_cooldowns(&contract, player_a.id()).await?;
    // assert!(
    //     cooldowns.iter().any(|(_, q)| q == &Quest::WeeklyWin),
    //     "WeeklyWin still on cooldown after second game"
    // );
    // assert!(
    //     cooldowns.iter().any(|(_, q)| q == &Quest::DailyPlayMove),
    //     "DailyPlayMove on cooldown after play_move"
    // );

    // Fast forward past DailyPlayMove cooldown (18s) but not WeeklyChallenger (126s) or WeeklyWin (126s)
    worker.fast_forward(100).await?;

    // A's play_move triggers cleanup on A's cooldowns
    call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;

    // THE BUG (before fix): WeeklyChallenger (126s cooldown, valid) gets wiped
    // because DailyPlayMove (18s) behind it expired and cleanup removed 0..=index
    let cooldowns = view::get_quest_cooldowns(&contract, player_a.id()).await?;
    assert!(
        cooldowns.iter().any(|(_, q)| q == &Quest::WeeklyChallenger),
        "WeeklyChallenger should still be on cooldown (126s) - not wiped by DailyPlayMove cleanup. Got: {:?}",
        cooldowns
    );

    Ok(())
}
