use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    near_bindgen,
    store::UnorderedSet,
    AccountId, PanicOnDefault,
};

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    hoomans: UnorderedSet<AccountId>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            hoomans: UnorderedSet::new(b"h".to_vec()),
        }
    }

    pub fn add_human(&mut self, account: AccountId) {
        self.hoomans.insert(account);
    }

    pub fn is_human(&self, account: AccountId) -> Vec<(AccountId, Vec<u64>)> {
        if self.hoomans.contains(&account) {
            vec![("fractal.i-am-human.near".parse().unwrap(), vec![42])]
        } else {
            vec![]
        }
    }
}
