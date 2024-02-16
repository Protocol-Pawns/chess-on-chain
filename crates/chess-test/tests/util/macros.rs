#[macro_export]
macro_rules! bet {
    ( $better:expr, $token_id:expr, $contract_id:expr, $amount:expr, $winner:expr => $looser:expr ) => {
        call::bet(
            $better,
            $token_id,
            $contract_id,
            $amount.into(),
            BetMsg {
                players: ($winner.id().clone(), $looser.id().clone()),
                winner: $winner.id().clone(),
            },
        )
    };
}
