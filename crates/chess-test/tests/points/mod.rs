use crate::util::*;
use chess_engine::Color;
use chess_lib::{create_challenge_id, Achievement, Difficulty, GameId, GameOutcome, Quest};
use near_contract_standards::fungible_token::events::FtMint;

#[tokio::test]
async fn test_daily_play_move() -> anyhow::Result<()> {
    let (worker, _, contract, _, nada_bot_contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::add_human(&nada_bot_contract, &player_a, player_a.id()),
        call::add_human(&nada_bot_contract, &player_b, player_b.id())
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
        player_a.id().clone(),
        Some(player_b.id().clone()),
    );

    let (_, block_hash, events) =
        call::play_move(&contract, &player_a, &game_id, "e2e4".to_string()).await?;
    let points = view::ft_balance_of(&contract, player_a.id()).await?;
    assert_eq!(points.0, Quest::DailyPlayMove.get_points(false));
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(supply.0, Quest::DailyPlayMove.get_points(false));
    let cooldowns = view::get_quest_cooldowns(&contract, player_a.id()).await?;
    let block = worker.view_block().block_hash(block_hash).await?;
    assert_eq!(
        cooldowns,
        vec![(block.timestamp() / 1_000_000, Quest::DailyPlayMove)]
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
    assert_eq!(supply.0, 2 * Quest::DailyPlayMove.get_points(false));
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
        Quest::DailyPlayMove.get_points(false) + Quest::DailyPlayMove.get_points(true)
    );
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(
        supply.0,
        2 * Quest::DailyPlayMove.get_points(false) + Quest::DailyPlayMove.get_points(true)
    );
    assert_ft_mint_events(
        events,
        vec![FtMint {
            owner_id: player_a.id(),
            amount: Quest::DailyPlayMove.get_points(true).into(),
            memo: Some("DailyPlayMove"),
        }],
    )?;

    let (_, _, events) =
        call::play_move(&contract, &player_b, &game_id, "a6a5".to_string()).await?;
    let points = view::ft_balance_of(&contract, player_b.id()).await?;
    assert_eq!(
        points.0,
        Quest::DailyPlayMove.get_points(false) + Quest::DailyPlayMove.get_points(true)
    );
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(
        supply.0,
        2 * Quest::DailyPlayMove.get_points(false) + 2 * Quest::DailyPlayMove.get_points(true)
    );
    assert_ft_mint_events(
        events,
        vec![FtMint {
            owner_id: player_b.id(),
            amount: Quest::DailyPlayMove.get_points(true).into(),
            memo: Some("DailyPlayMove"),
        }],
    )?;

    worker.fast_forward(100).await?;

    let (_, _, events) =
        call::play_move(&contract, &player_a, &game_id, "f1c4".to_string()).await?;
    let points = view::ft_balance_of(&contract, player_a.id()).await?;
    assert_eq!(
        points.0,
        2 * Quest::DailyPlayMove.get_points(false) + Quest::DailyPlayMove.get_points(true)
    );
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(
        supply.0,
        3 * Quest::DailyPlayMove.get_points(false) + 2 * Quest::DailyPlayMove.get_points(true)
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
        call::play_move(&contract, &player_b, &game_id, "a5a4".to_string()).await?;
    let points = view::ft_balance_of(&contract, player_b.id()).await?;
    assert_eq!(
        points.0,
        2 * Quest::DailyPlayMove.get_points(false) + Quest::DailyPlayMove.get_points(true)
    );
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(
        supply.0,
        4 * Quest::DailyPlayMove.get_points(false) + 2 * Quest::DailyPlayMove.get_points(true)
    );
    assert_ft_mint_events(
        events,
        vec![FtMint {
            owner_id: player_b.id(),
            amount: Quest::DailyPlayMove.get_points(false).into(),
            memo: Some("DailyPlayMove"),
        }],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_first_win_human() -> anyhow::Result<()> {
    let (worker, _, contract, _, nada_bot_contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::add_human(&nada_bot_contract, &player_a, player_a.id()),
        call::add_human(&nada_bot_contract, &player_b, player_b.id())
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
    let points = view::ft_balance_of(&contract, player_a.id()).await?;
    assert_eq!(
        points.0,
        Quest::DailyPlayMove.get_points(false)
            + 3 * Quest::DailyPlayMove.get_points(true)
            + Achievement::FirstWinHuman.get_points()
    );
    let points = view::ft_balance_of(&contract, player_b.id()).await?;
    assert_eq!(
        points.0,
        Quest::DailyPlayMove.get_points(false) + 2 * Quest::DailyPlayMove.get_points(true)
    );
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(
        supply.0,
        2 * Quest::DailyPlayMove.get_points(false)
            + 5 * Quest::DailyPlayMove.get_points(true)
            + Achievement::FirstWinHuman.get_points()
    );
    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    let block = worker.view_block().block_hash(block_hash).await?;
    assert_eq!(
        achievements,
        vec![(block.timestamp() / 1_000_000, Achievement::FirstWinHuman)]
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
                amount: Achievement::FirstWinHuman.get_points().into(),
                memo: Some("FirstWinHuman"),
            },
        ],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_first_win_not_human() -> anyhow::Result<()> {
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
    let points = view::ft_balance_of(&contract, player_a.id()).await?;
    assert_eq!(
        points.0,
        Quest::DailyPlayMove.get_points(false) + 3 * Quest::DailyPlayMove.get_points(true)
    );
    let points = view::ft_balance_of(&contract, player_b.id()).await?;
    assert_eq!(
        points.0,
        Quest::DailyPlayMove.get_points(false) + 2 * Quest::DailyPlayMove.get_points(true)
    );
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(
        supply.0,
        2 * Quest::DailyPlayMove.get_points(false) + 5 * Quest::DailyPlayMove.get_points(true)
    );
    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    worker.view_block().block_hash(block_hash).await?;
    assert!(achievements.is_empty(),);
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
async fn test_first_win_ai() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

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
        + 3 * Quest::DailyPlayMove.get_points(true)
        + Achievement::FirstWinAiEasy.get_points();
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
        ],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_achievement_only_once() -> anyhow::Result<()> {
    let (worker, _, contract, _, _) = initialize_contracts(None).await?;

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
    let expected = 2 * Quest::DailyPlayMove.get_points(false)
        + 6 * Quest::DailyPlayMove.get_points(true)
        + Achievement::FirstWinAiEasy.get_points();
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
        vec![FtMint {
            owner_id: player_a.id(),
            amount: Quest::DailyPlayMove.get_points(true).into(),
            memo: Some("DailyPlayMove"),
        }],
    )?;

    Ok(())
}

#[tokio::test]
async fn test_multiple_achievements() -> anyhow::Result<()> {
    let (worker, _, contract, _, nada_bot_contract) = initialize_contracts(None).await?;

    let player_a = worker.dev_create_account().await?;
    let player_b = worker.dev_create_account().await?;

    tokio::try_join!(
        call::add_human(&nada_bot_contract, &player_a, player_a.id()),
        call::add_human(&nada_bot_contract, &player_b, player_b.id())
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
    let human_achievement = (block.timestamp() / 1_000_000, Achievement::FirstWinHuman);

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
    let points = view::ft_balance_of(&contract, player_a.id()).await?;
    assert_eq!(
        points.0,
        2 * Quest::DailyPlayMove.get_points(false)
            + 6 * Quest::DailyPlayMove.get_points(true)
            + Achievement::FirstWinHuman.get_points()
            + Achievement::FirstWinAiEasy.get_points()
    );
    let supply = view::ft_total_supply(&contract).await?;
    assert_eq!(
        supply.0,
        3 * Quest::DailyPlayMove.get_points(false)
            + 8 * Quest::DailyPlayMove.get_points(true)
            + Achievement::FirstWinHuman.get_points()
            + Achievement::FirstWinAiEasy.get_points()
    );
    let achievements = view::get_achievements(&contract, player_a.id()).await?;
    assert_eq!(achievements, vec![human_achievement, ai_achievement]);

    Ok(())
}
