use crate::{Chess, ChessExt};
use near_contract_standards::fungible_token::{
    metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider},
    FungibleTokenCore,
};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    json_types::U128,
    near_bindgen,
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    AccountId, PromiseOrValue,
};

#[derive(
    Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Deserialize, Serialize, JsonSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
#[schemars(crate = "near_sdk::schemars")]
pub enum Quest {
    DailyPlayMove,
    WeeklyWinHuman,
}

#[cfg(not(feature = "integration-test"))]
const DAYLIGHT_DIFF: u64 = 1_000 * 60 * 60 * 8;
#[cfg(not(feature = "integration-test"))]
const DAILY_COOLDOWN: u64 = 1_000 * 60 * 60 * 24 - DAYLIGHT_DIFF;
#[cfg(not(feature = "integration-test"))]
const WEEKLY_COOLDOWN: u64 = 1_000 * 60 * 60 * 24 * 7 - DAYLIGHT_DIFF;

impl Quest {
    pub fn get_points(&self, on_cooldown: bool) -> u128 {
        match (self, on_cooldown) {
            (Quest::DailyPlayMove, true) => 1_000,
            (Quest::DailyPlayMove, false) => 100_000,
            (Quest::WeeklyWinHuman, true) => 200_000,
            (Quest::WeeklyWinHuman, false) => 2_000_000,
        }
    }

    pub fn get_cooldown(&self) -> u64 {
        #[cfg(not(feature = "integration-test"))]
        match self {
            Quest::DailyPlayMove => DAILY_COOLDOWN,
            Quest::WeeklyWinHuman => WEEKLY_COOLDOWN,
        }
        #[cfg(feature = "integration-test")]
        match self {
            Quest::DailyPlayMove => 1_000 * 18,
            Quest::WeeklyWinHuman => 1_000 * 18 * 7,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Quest::DailyPlayMove => "DailyPlayMove",
            Quest::WeeklyWinHuman => "WeeklyWinHuman",
        }
    }
}

#[derive(
    Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Deserialize, Serialize, JsonSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
#[schemars(crate = "near_sdk::schemars")]
pub enum Achievement {
    FirstWinHuman,
    FirstWinAiEasy,
    FirstWinAiMedium,
    FirstWinAiHard,
}

impl Achievement {
    pub fn get_points(&self) -> u128 {
        match self {
            Achievement::FirstWinHuman => 8_000_000,
            Achievement::FirstWinAiEasy => 1_000_000,
            Achievement::FirstWinAiMedium => 2_500_000,
            Achievement::FirstWinAiHard => 5_000_000,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Achievement::FirstWinHuman => "FirstWinHuman",
            Achievement::FirstWinAiEasy => "FirstWinAiEasy",
            Achievement::FirstWinAiMedium => "FirstWinAiMedium",
            Achievement::FirstWinAiHard => "FirstWinAiHard",
        }
    }
}

#[near_bindgen]
impl FungibleTokenMetadataProvider for Chess {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            name: "Protocol Pawns Points".to_string(),
            symbol: "PPP".to_string(),
            icon: Some(DATA_IMAGE_WEBP_ICON.to_string()),
            reference: None,
            reference_hash: None,
            decimals: 6,
        }
    }
}

#[near_bindgen]
impl FungibleTokenCore for Chess {
    #[allow(unused)]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        panic!("Protocol Pawns Points are not transferable");
    }

    #[allow(unused)]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        panic!("Protocol Pawns Points are not transferable");
    }

    fn ft_total_supply(&self) -> U128 {
        0.into()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.get_points(account_id).unwrap()
    }
}

const DATA_IMAGE_WEBP_ICON: &str = "data:image/webp;base64,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";
