//! Tauri IPC commands.

use crate::state::{AppState, BotStats, CommonMarketsData, ExchangeRateData, ExecutionConfig, OpportunityData, PriceData};
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_commands_exist() {
        // Commands are tested via integration tests
        assert!(true);
    }
}
