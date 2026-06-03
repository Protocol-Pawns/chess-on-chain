use anyhow::Result;
use base64::Engine;
use near_workspaces::AccountId;

const RPC_URL: &str = "https://rpc.shitzuapes.xyz";
const CONTRACT_ID: &str = "app.chess-game.near";
const WASM_OUT: &str = "res/chess_mainnet.wasm";
const STATE_OUT: &str = "res/mainnet_state.json";

#[tokio::main]
async fn main() -> Result<()> {
    let contract_id: AccountId = CONTRACT_ID.parse()?;

    let worker = near_workspaces::mainnet_archival()
        .rpc_addr(RPC_URL)
        .await?;

    println!("Downloading WASM from {CONTRACT_ID} via {RPC_URL}...");
    let code = worker.view_code(&contract_id).await?;
    std::fs::write(WASM_OUT, &code)?;
    println!("WASM saved to {WASM_OUT} ({} bytes)", code.len());

    println!("Downloading state from {CONTRACT_ID}...");
    let state = worker.view_state(&contract_id).await?;
    println!("State entries: {}", state.len());

    let entries: Vec<serde_json::Value> = state
        .iter()
        .map(|(key, value)| {
            serde_json::json!({
                "key": base64::engine::general_purpose::STANDARD.encode(key),
                "value": base64::engine::general_purpose::STANDARD.encode(value),
            })
        })
        .collect();

    let json = serde_json::to_string(&entries)?;
    std::fs::write(STATE_OUT, json)?;
    println!("State saved to {STATE_OUT} ({} entries)", entries.len());

    Ok(())
}
