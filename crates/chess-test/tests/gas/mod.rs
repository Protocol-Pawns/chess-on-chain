use crate::util::*;
use chess_engine::Color;
use chess_lib::{BetMsg, GameId, GameOutcome, MAX_BETS_PER_GAME};
use futures::future::try_join_all;
use near_workspaces::types::NearToken;

const BATCH_SIZE: usize = 25;

#[tokio::test]
async fn test_gas_max_bets_per_game() -> anyhow::Result<()> {
    let (worker, _, contract) = initialize_contracts(Some("../../res/chess.wasm")).await?;
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

    call::challenge(&contract, &player_a, player_b.id()).await?;

    let mut bettors = Vec::with_capacity(MAX_BETS_PER_GAME as usize);
    for _ in 0..MAX_BETS_PER_GAME {
        bettors.push(worker.dev_create_account().await?);
    }

    for chunk in bettors.chunks(BATCH_SIZE) {
        let tasks: Vec<_> = chunk
            .iter()
            .map(|b| call::storage_deposit(&contract, b, None, None))
            .collect();
        try_join_all(tasks).await?;
    }

    for chunk in bettors.chunks(BATCH_SIZE) {
        let tasks: Vec<_> = chunk
            .iter()
            .map(|b| async {
                call::storage_deposit(&test_token, b, None, Some(NearToken::from_millinear(100)))
                    .await?;
                call::mint_tokens(&test_token, b.id(), bet_amount).await?;
                Ok::<_, anyhow::Error>(())
            })
            .collect();
        try_join_all(tasks).await?;
    }

    call::bet(
        &bettors[0],
        test_token.id(),
        contract.id(),
        bet_amount.into(),
        BetMsg {
            players: (player_a.id().clone(), player_b.id().clone()),
            winner: player_b.id().clone(),
        },
    )
    .await?;

    for bettor in &bettors[1..] {
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

    let challenge_id = chess_lib::create_challenge_id(player_a.id(), player_b.id());
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
    let ((outcome, _board), _, _) =
        call::play_move(&contract, &player_a, &game_id, "f3f7".to_string()).await?;
    assert_eq!(outcome.unwrap(), GameOutcome::Victory(Color::White));

    Ok(())
}
