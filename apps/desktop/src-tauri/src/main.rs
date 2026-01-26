//! Arbitrage Bot - Desktop Application
//!
//! Tauri-based desktop GUI that connects to CLI server for real-time data.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod credentials;
mod exchange_client;
mod state;
mod symbol_mapping;
mod wts;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

fn init_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber).ok();
}

fn main() {
    init_logging();
    info!("Starting Arbitrage Bot Desktop");

    let app_state = Arc::new(AppState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state.clone())
        .setup(move |app| {
            // Spawn CLI server connection
            let app_handle = app.handle().clone();
            let state = app.state::<Arc<AppState>>().inner().clone();
            tauri::async_runtime::spawn(async move {
                state::run_server_connection(state, app_handle).await;
            });

            info!("Arbitrage Bot Desktop initialized");
            info!("Connecting to CLI server at ws://127.0.0.1:9001/ws");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_prices,
            commands::get_opportunities,
            commands::get_stats,
            commands::start_bot,
            commands::stop_bot,
            commands::get_config,
            commands::update_config,
            commands::execute_opportunity,
            commands::set_server_url,
            commands::is_connected,
            commands::get_exchange_rate,
            commands::get_common_markets,
            commands::get_wallet_status,
            commands::get_credentials,
            commands::save_credentials,
            commands::get_wallet_info,
            commands::get_all_wallets,
            commands::get_symbol_mappings,
            commands::upsert_symbol_mapping,
            commands::remove_symbol_mapping,
            commands::save_symbol_mappings,
            wts::wts_open_window,
            wts::wts_check_connection,
            wts::wts_get_balance,
            wts::wts_get_markets,
            wts::wts_place_order,
            wts::wts_get_deposit_address,
            wts::wts_generate_deposit_address,
            wts::wts_get_deposit_chance,
            wts::wts_withdraw,
            wts::wts_get_withdraw_chance,
            wts::wts_get_withdraw_addresses,
            wts::wts_get_withdraw,
            wts::wts_generate_ws_token,
            wts::wts_start_myorder_ws,
            wts::wts_stop_myorder_ws,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
