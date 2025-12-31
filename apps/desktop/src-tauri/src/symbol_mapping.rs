//! Symbol mapping management.
//!
//! Handles cases where the same symbol represents different coins across exchanges.
//! For example, "GTC" might be Gitcoin on Binance but a different coin on another exchange.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

/// A single symbol mapping entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMapping {
    /// The exchange where this mapping applies
    pub exchange: String,
    /// The symbol as it appears on the exchange (e.g., "GTC")
    pub symbol: String,
    /// The canonical/unified name for this asset (e.g., "Gitcoin" or "GTC_BINANCE")
    pub canonical_name: String,
    /// Whether this symbol should be excluded from arbitrage (different coins)
    pub exclude: bool,
    /// Optional notes about why this mapping exists
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// All symbol mappings configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolMappings {
    /// List of all symbol mappings
    pub mappings: Vec<SymbolMapping>,
}

impl SymbolMappings {
    /// Get all mappings for a specific exchange.
    pub fn for_exchange(&self, exchange: &str) -> Vec<&SymbolMapping> {
        self.mappings
            .iter()
            .filter(|m| m.exchange.eq_ignore_ascii_case(exchange))
            .collect()
    }

    /// Get mapping for a specific exchange and symbol.
    pub fn get(&self, exchange: &str, symbol: &str) -> Option<&SymbolMapping> {
        self.mappings.iter().find(|m| {
            m.exchange.eq_ignore_ascii_case(exchange) && m.symbol.eq_ignore_ascii_case(symbol)
        })
    }

    /// Check if a symbol should be excluded from arbitrage for an exchange.
    pub fn is_excluded(&self, exchange: &str, symbol: &str) -> bool {
        self.get(exchange, symbol).map_or(false, |m| m.exclude)
    }

    /// Get the canonical name for a symbol on an exchange.
    /// Returns the original symbol if no mapping exists.
    pub fn canonical_name(&self, exchange: &str, symbol: &str) -> String {
        self.get(exchange, symbol)
            .map(|m| m.canonical_name.clone())
            .unwrap_or_else(|| symbol.to_string())
    }

    /// Add or update a mapping.
    pub fn upsert(&mut self, mapping: SymbolMapping) {
        if let Some(existing) = self.mappings.iter_mut().find(|m| {
            m.exchange.eq_ignore_ascii_case(&mapping.exchange)
                && m.symbol.eq_ignore_ascii_case(&mapping.symbol)
        }) {
            *existing = mapping;
        } else {
            self.mappings.push(mapping);
        }
    }

    /// Remove a mapping.
    pub fn remove(&mut self, exchange: &str, symbol: &str) -> bool {
        let len_before = self.mappings.len();
        self.mappings.retain(|m| {
            !(m.exchange.eq_ignore_ascii_case(exchange) && m.symbol.eq_ignore_ascii_case(symbol))
        });
        self.mappings.len() < len_before
    }
}

/// Get the symbol mappings file path.
fn get_mappings_path() -> PathBuf {
    // Try to find project root by looking for .env
    let mut path = std::env::current_dir().unwrap_or_default();

    for _ in 0..5 {
        if path.join(".env").exists() || path.join(".env.example").exists() {
            break;
        }
        if path.join("Cargo.toml").exists() && path.join("apps").exists() {
            break;
        }
        if let Some(parent) = path.parent() {
            path = parent.to_path_buf();
        } else {
            break;
        }
    }

    path.join("symbol_mappings.json")
}

/// Load symbol mappings from file.
pub fn load_mappings() -> SymbolMappings {
    let path = get_mappings_path();
    info!("Loading symbol mappings from: {:?}", path);

    if !path.exists() {
        info!("No symbol mappings file found, using defaults");
        return SymbolMappings::default();
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(mappings) => {
                info!("Loaded symbol mappings successfully");
                mappings
            }
            Err(e) => {
                warn!("Failed to parse symbol mappings: {}", e);
                SymbolMappings::default()
            }
        },
        Err(e) => {
            warn!("Failed to read symbol mappings file: {}", e);
            SymbolMappings::default()
        }
    }
}

/// Save symbol mappings to file.
pub fn save_mappings(mappings: &SymbolMappings) -> Result<(), String> {
    let path = get_mappings_path();
    info!("Saving symbol mappings to: {:?}", path);

    let content = serde_json::to_string_pretty(mappings)
        .map_err(|e| format!("Failed to serialize mappings: {}", e))?;

    fs::write(&path, content).map_err(|e| format!("Failed to write mappings file: {}", e))?;

    info!("Symbol mappings saved successfully");
    Ok(())
}

/// Build a lookup map for quick canonical name resolution.
/// Returns: HashMap<(exchange, symbol), canonical_name>
pub fn build_lookup_map(mappings: &SymbolMappings) -> HashMap<(String, String), String> {
    mappings
        .mappings
        .iter()
        .map(|m| {
            (
                (m.exchange.to_lowercase(), m.symbol.to_uppercase()),
                m.canonical_name.clone(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_mappings_get() {
        let mut mappings = SymbolMappings::default();
        mappings.mappings.push(SymbolMapping {
            exchange: "Binance".to_string(),
            symbol: "GTC".to_string(),
            canonical_name: "Gitcoin".to_string(),
            exclude: false,
            notes: Some("Gitcoin on Binance".to_string()),
        });

        assert!(mappings.get("Binance", "GTC").is_some());
        assert!(mappings.get("binance", "gtc").is_some()); // case insensitive
        assert!(mappings.get("Upbit", "GTC").is_none());
    }

    #[test]
    fn test_symbol_mappings_upsert() {
        let mut mappings = SymbolMappings::default();

        // Add new
        mappings.upsert(SymbolMapping {
            exchange: "Binance".to_string(),
            symbol: "GTC".to_string(),
            canonical_name: "Gitcoin".to_string(),
            exclude: false,
            notes: None,
        });
        assert_eq!(mappings.mappings.len(), 1);

        // Update existing
        mappings.upsert(SymbolMapping {
            exchange: "Binance".to_string(),
            symbol: "GTC".to_string(),
            canonical_name: "Gitcoin_v2".to_string(),
            exclude: true,
            notes: None,
        });
        assert_eq!(mappings.mappings.len(), 1);
        assert_eq!(mappings.get("Binance", "GTC").unwrap().canonical_name, "Gitcoin_v2");
        assert!(mappings.get("Binance", "GTC").unwrap().exclude);
    }

    #[test]
    fn test_symbol_mappings_remove() {
        let mut mappings = SymbolMappings::default();
        mappings.mappings.push(SymbolMapping {
            exchange: "Binance".to_string(),
            symbol: "GTC".to_string(),
            canonical_name: "Gitcoin".to_string(),
            exclude: false,
            notes: None,
        });

        assert!(mappings.remove("Binance", "GTC"));
        assert!(mappings.mappings.is_empty());
        assert!(!mappings.remove("Binance", "GTC")); // Already removed
    }

    #[test]
    fn test_canonical_name() {
        let mut mappings = SymbolMappings::default();
        mappings.mappings.push(SymbolMapping {
            exchange: "Binance".to_string(),
            symbol: "GTC".to_string(),
            canonical_name: "Gitcoin".to_string(),
            exclude: false,
            notes: None,
        });

        assert_eq!(mappings.canonical_name("Binance", "GTC"), "Gitcoin");
        assert_eq!(mappings.canonical_name("Binance", "BTC"), "BTC"); // No mapping, returns original
    }
}
