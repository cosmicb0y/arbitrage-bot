//! Credentials management using .env file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

/// Exchange credentials.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExchangeCredentials {
    pub api_key: String,
    pub secret_key: String,
}

/// All exchange credentials.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Credentials {
    pub binance: ExchangeCredentials,
    pub coinbase: ExchangeCredentials,
    pub upbit: ExchangeCredentials,
}

impl Credentials {
    /// Check if any credentials are configured.
    pub fn has_any(&self) -> bool {
        !self.binance.api_key.is_empty()
            || !self.coinbase.api_key.is_empty()
            || !self.upbit.api_key.is_empty()
    }

    /// Get configured exchanges.
    pub fn configured_exchanges(&self) -> Vec<String> {
        let mut exchanges = Vec::new();
        if !self.binance.api_key.is_empty() {
            exchanges.push("Binance".to_string());
        }
        if !self.coinbase.api_key.is_empty() {
            exchanges.push("Coinbase".to_string());
        }
        if !self.upbit.api_key.is_empty() {
            exchanges.push("Upbit".to_string());
        }
        exchanges
    }
}

/// Get the .env file path (project root).
fn get_env_path() -> PathBuf {
    // Try to find project root by looking for Cargo.toml
    let mut path = std::env::current_dir().unwrap_or_default();

    // If we're in src-tauri, go up to project root
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

    path.join(".env")
}

/// Load credentials from .env file.
pub fn load_credentials() -> Credentials {
    let env_path = get_env_path();
    info!("Loading credentials from: {:?}", env_path);

    // Try to load .env file
    if let Err(e) = dotenvy::from_path(&env_path) {
        warn!("Could not load .env file: {}", e);
    }

    Credentials {
        binance: ExchangeCredentials {
            api_key: std::env::var("BINANCE_API_KEY").unwrap_or_default(),
            secret_key: std::env::var("BINANCE_SECRET_KEY").unwrap_or_default(),
        },
        coinbase: ExchangeCredentials {
            api_key: std::env::var("COINBASE_API_KEY").unwrap_or_default(),
            secret_key: std::env::var("COINBASE_SECRET_KEY").unwrap_or_default(),
        },
        upbit: ExchangeCredentials {
            api_key: std::env::var("UPBIT_ACCESS_KEY").unwrap_or_default(),
            secret_key: std::env::var("UPBIT_SECRET_KEY").unwrap_or_default(),
        },
    }
}

/// Save credentials to .env file.
pub fn save_credentials(creds: &Credentials) -> Result<(), String> {
    let env_path = get_env_path();
    info!("Saving credentials to: {:?}", env_path);

    // Read existing .env or create new content
    let mut env_vars: HashMap<String, String> = HashMap::new();

    // Load existing vars if file exists
    if env_path.exists() {
        if let Ok(content) = fs::read_to_string(&env_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    env_vars.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }
    }

    // Update with new credentials
    env_vars.insert("BINANCE_API_KEY".to_string(), creds.binance.api_key.clone());
    env_vars.insert("BINANCE_SECRET_KEY".to_string(), creds.binance.secret_key.clone());
    env_vars.insert("COINBASE_API_KEY".to_string(), creds.coinbase.api_key.clone());
    env_vars.insert("COINBASE_SECRET_KEY".to_string(), creds.coinbase.secret_key.clone());
    env_vars.insert("UPBIT_ACCESS_KEY".to_string(), creds.upbit.api_key.clone());
    env_vars.insert("UPBIT_SECRET_KEY".to_string(), creds.upbit.secret_key.clone());

    // Write to file
    let content = format!(
        "# Arbitrage Bot - Exchange API Credentials\n\
         # DO NOT COMMIT THIS FILE\n\n\
         # Binance\n\
         BINANCE_API_KEY={}\n\
         BINANCE_SECRET_KEY={}\n\n\
         # Coinbase\n\
         COINBASE_API_KEY={}\n\
         COINBASE_SECRET_KEY={}\n\n\
         # Upbit\n\
         UPBIT_ACCESS_KEY={}\n\
         UPBIT_SECRET_KEY={}\n",
        creds.binance.api_key,
        creds.binance.secret_key,
        creds.coinbase.api_key,
        creds.coinbase.secret_key,
        creds.upbit.api_key,
        creds.upbit.secret_key,
    );

    fs::write(&env_path, content).map_err(|e| format!("Failed to write .env: {}", e))?;

    // Update environment variables in current process
    std::env::set_var("BINANCE_API_KEY", &creds.binance.api_key);
    std::env::set_var("BINANCE_SECRET_KEY", &creds.binance.secret_key);
    std::env::set_var("COINBASE_API_KEY", &creds.coinbase.api_key);
    std::env::set_var("COINBASE_SECRET_KEY", &creds.coinbase.secret_key);
    std::env::set_var("UPBIT_ACCESS_KEY", &creds.upbit.api_key);
    std::env::set_var("UPBIT_SECRET_KEY", &creds.upbit.secret_key);

    info!("Credentials saved successfully");
    Ok(())
}

/// Get masked credentials for display (hide secret keys).
pub fn get_masked_credentials() -> Credentials {
    let creds = load_credentials();

    fn mask(s: &str) -> String {
        if s.is_empty() {
            String::new()
        } else if s.len() <= 8 {
            "*".repeat(s.len())
        } else {
            format!("{}...{}", &s[..4], &s[s.len()-4..])
        }
    }

    Credentials {
        binance: ExchangeCredentials {
            api_key: mask(&creds.binance.api_key),
            secret_key: mask(&creds.binance.secret_key),
        },
        coinbase: ExchangeCredentials {
            api_key: mask(&creds.coinbase.api_key),
            secret_key: mask(&creds.coinbase.secret_key),
        },
        upbit: ExchangeCredentials {
            api_key: mask(&creds.upbit.api_key),
            secret_key: mask(&creds.upbit.secret_key),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_has_any() {
        let empty = Credentials::default();
        assert!(!empty.has_any());

        let with_binance = Credentials {
            binance: ExchangeCredentials {
                api_key: "test".to_string(),
                secret_key: "secret".to_string(),
            },
            ..Default::default()
        };
        assert!(with_binance.has_any());
    }

    #[test]
    fn test_configured_exchanges() {
        let creds = Credentials {
            binance: ExchangeCredentials {
                api_key: "key".to_string(),
                secret_key: "secret".to_string(),
            },
            upbit: ExchangeCredentials {
                api_key: "key".to_string(),
                secret_key: "secret".to_string(),
            },
            ..Default::default()
        };

        let exchanges = creds.configured_exchanges();
        assert_eq!(exchanges.len(), 2);
        assert!(exchanges.contains(&"Binance".to_string()));
        assert!(exchanges.contains(&"Upbit".to_string()));
    }
}
