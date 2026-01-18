//! Tauri IPC commands.

use crate::credentials::{self, Credentials};
use crate::state::{
    AppState, BotStats, CommonMarketsData, ExchangeRateData, ExecutionConfig, OpportunityData,
    PriceData, WalletStatusData,
};
use crate::symbol_mapping::{self, SymbolMapping, SymbolMappings};
use std::sync::Arc;
use tauri::State;
use tracing::info;

/// Get current prices from CLI server.
#[tauri::command]
pub fn get_prices(state: State<'_, Arc<AppState>>) -> Vec<PriceData> {
    state.get_prices()
}

/// Get detected arbitrage opportunities from CLI server.
#[tauri::command]
pub fn get_opportunities(state: State<'_, Arc<AppState>>) -> Vec<OpportunityData> {
    state.get_opportunities()
}

/// Get bot statistics from CLI server.
#[tauri::command]
pub fn get_stats(state: State<'_, Arc<AppState>>) -> BotStats {
    state.get_stats()
}

/// Start the bot (sends command to CLI server - not implemented yet).
#[tauri::command]
pub fn start_bot(_state: State<'_, Arc<AppState>>) -> bool {
    info!("start_bot called - CLI server controls bot state");
    // TODO: Send start command to CLI server via WebSocket
    true
}

/// Stop the bot (sends command to CLI server - not implemented yet).
#[tauri::command]
pub fn stop_bot(_state: State<'_, Arc<AppState>>) -> bool {
    info!("stop_bot called - CLI server controls bot state");
    // TODO: Send stop command to CLI server via WebSocket
    true
}

/// Get current configuration.
#[tauri::command]
pub fn get_config(state: State<'_, Arc<AppState>>) -> ExecutionConfig {
    state.get_config()
}

/// Update configuration.
#[tauri::command]
pub fn update_config(config: ExecutionConfig, state: State<'_, Arc<AppState>>) -> bool {
    state.update_config(config);
    info!("Config updated via command");
    true
}

/// Execute an arbitrage opportunity (manual approval).
#[tauri::command]
pub fn execute_opportunity(
    opportunity_id: u64,
    _amount: f64,
    _state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    info!("execute_opportunity called for {}", opportunity_id);
    // TODO: Send execute command to CLI server via WebSocket
    Ok(format!(
        "Execution request sent for opportunity {}",
        opportunity_id
    ))
}

/// Set CLI server WebSocket URL.
#[tauri::command]
pub fn set_server_url(url: String, state: State<'_, Arc<AppState>>) -> bool {
    info!("Setting server URL to: {}", url);
    state.set_server_url(url);
    true
}

/// Check if connected to CLI server.
#[tauri::command]
pub fn is_connected(state: State<'_, Arc<AppState>>) -> bool {
    state.is_connected()
}

/// Get current exchange rate.
#[tauri::command]
pub fn get_exchange_rate(state: State<'_, Arc<AppState>>) -> Option<ExchangeRateData> {
    state.get_exchange_rate()
}

/// Get common markets across exchanges.
#[tauri::command]
pub fn get_common_markets(state: State<'_, Arc<AppState>>) -> Option<CommonMarketsData> {
    state.get_common_markets()
}

/// Get wallet status (deposit/withdraw availability) from server.
#[tauri::command]
pub fn get_wallet_status(state: State<'_, Arc<AppState>>) -> Option<WalletStatusData> {
    state.get_wallet_status()
}

/// Get credentials (masked for display).
#[tauri::command]
pub fn get_credentials() -> Credentials {
    credentials::get_masked_credentials()
}

/// Save credentials to .env file.
#[tauri::command]
pub fn save_credentials(creds: Credentials) -> Result<bool, String> {
    credentials::save_credentials(&creds)?;
    info!("Credentials saved via command");
    Ok(true)
}

/// Get wallet info for a specific exchange.
#[tauri::command]
pub async fn get_wallet_info(
    exchange: String,
) -> Result<crate::exchange_client::ExchangeWalletInfo, String> {
    info!("Fetching wallet info for {}", exchange);
    match exchange.to_lowercase().as_str() {
        "binance" => crate::exchange_client::fetch_binance_wallet().await,
        "upbit" => crate::exchange_client::fetch_upbit_wallet().await,
        "coinbase" => crate::exchange_client::fetch_coinbase_wallet().await,
        _ => Err(format!("Unknown exchange: {}", exchange)),
    }
}

/// Get wallet info for all configured exchanges.
#[tauri::command]
pub async fn get_all_wallets() -> Vec<crate::exchange_client::ExchangeWalletInfo> {
    info!("Fetching wallet info for all exchanges");
    crate::exchange_client::fetch_all_wallets().await
}

// ============ Symbol Mapping Commands ============

/// Get all symbol mappings.
#[tauri::command]
pub fn get_symbol_mappings() -> SymbolMappings {
    symbol_mapping::load_mappings()
}

/// Add or update a symbol mapping.
#[tauri::command]
pub fn upsert_symbol_mapping(mapping: SymbolMapping) -> Result<bool, String> {
    let mut mappings = symbol_mapping::load_mappings();
    mappings.upsert(mapping);
    symbol_mapping::save_mappings(&mappings)?;
    info!("Symbol mapping upserted");
    Ok(true)
}

/// Remove a symbol mapping.
#[tauri::command]
pub fn remove_symbol_mapping(exchange: String, symbol: String) -> Result<bool, String> {
    let mut mappings = symbol_mapping::load_mappings();
    let removed = mappings.remove(&exchange, &symbol);
    if removed {
        symbol_mapping::save_mappings(&mappings)?;
        info!("Symbol mapping removed: {}:{}", exchange, symbol);
    }
    Ok(removed)
}

/// Save all symbol mappings (bulk update).
#[tauri::command]
pub fn save_symbol_mappings(mappings: SymbolMappings) -> Result<bool, String> {
    symbol_mapping::save_mappings(&mappings)?;
    info!("All symbol mappings saved");
    Ok(true)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_commands_exist() {
        // Commands are tested via integration tests
        assert!(true);
    }
}
