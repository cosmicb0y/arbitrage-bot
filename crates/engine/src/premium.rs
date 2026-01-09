//! Premium calculation across exchange pairs.
//!
//! Calculates and tracks arbitrage premiums between all exchange pairs.

use arbitrage_core::{Exchange, FixedPoint, QuoteCurrency};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// USD-like stablecoin type for premium calculation.
/// Represents stablecoins pegged to USD that can be compared directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UsdlikeQuote {
    USDT,
    USDC,
    BUSD,
}

impl UsdlikeQuote {
    /// Convert from QuoteCurrency if applicable.
    pub fn from_quote_currency(quote: QuoteCurrency) -> Option<Self> {
        match quote {
            QuoteCurrency::USDT => Some(UsdlikeQuote::USDT),
            QuoteCurrency::USDC => Some(UsdlikeQuote::USDC),
            QuoteCurrency::BUSD => Some(UsdlikeQuote::BUSD),
            _ => None,
        }
    }

    /// Convert to QuoteCurrency.
    pub fn to_quote_currency(self) -> QuoteCurrency {
        match self {
            UsdlikeQuote::USDT => QuoteCurrency::USDT,
            UsdlikeQuote::USDC => QuoteCurrency::USDC,
            UsdlikeQuote::BUSD => QuoteCurrency::BUSD,
        }
    }

    /// Get display name.
    pub fn as_str(self) -> &'static str {
        match self {
            UsdlikeQuote::USDT => "USDT",
            UsdlikeQuote::USDC => "USDC",
            UsdlikeQuote::BUSD => "BUSD",
        }
    }
}

impl std::fmt::Display for UsdlikeQuote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Price in a USD-like stablecoin with its quote type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UsdlikePrice {
    /// Price value
    pub price: FixedPoint,
    /// Which stablecoin was used
    pub quote: UsdlikeQuote,
}

impl UsdlikePrice {
    /// Create a new UsdlikePrice.
    pub fn new(price: FixedPoint, quote: UsdlikeQuote) -> Self {
        Self { price, quote }
    }
}

/// Prices converted to multiple denominations (USDlike, USD).
///
/// Each exchange stores prices in its native quote currency (KRW, USDT, USDC, USD).
/// This struct provides converted prices for accurate premium comparison:
/// - USDlike Premium: Compare same stablecoin prices (USDT vs USDT, USDC vs USDC)
/// - Kimchi Premium: Compare USD prices (KRW via forex rate)
///
/// For KRW markets, `usdlike` is None and will be computed at opportunity detection time
/// based on the overseas market's quote currency.
#[derive(Debug, Clone, Copy)]
pub struct DenominatedPrices {
    /// Price in USD-like stablecoin (USDT, USDC, or BUSD).
    /// For KRW markets, this is None until converted at detection time.
    /// For overseas markets, this is set based on the market's quote currency.
    pub usdlike: Option<UsdlikePrice>,
    /// Price in USD (KRW via forex rate, USDT * USDT_USD, etc.)
    pub usd: Option<FixedPoint>,
    /// Raw price in original currency
    pub raw: FixedPoint,
    /// Original quote currency
    pub original_quote: QuoteCurrency,
}

impl Default for DenominatedPrices {
    fn default() -> Self {
        Self {
            usdlike: None,
            usd: None,
            raw: FixedPoint(0),
            original_quote: QuoteCurrency::USD,
        }
    }
}

impl DenominatedPrices {
    /// Create from KRW price.
    /// USDlike is None - will be computed at opportunity detection time based on overseas quote.
    ///
    /// # Arguments
    /// * `krw` - Raw price in KRW
    /// * `usd_krw` - USD/KRW forex rate from 하나은행 (e.g., 1450.0)
    pub fn from_krw(krw: FixedPoint, usd_krw: f64) -> Self {
        let krw_f64 = krw.to_f64();
        Self {
            usdlike: None, // Computed at detection time based on overseas market quote
            usd: if usd_krw > 0.0 {
                Some(FixedPoint::from_f64(krw_f64 / usd_krw))
            } else {
                None
            },
            raw: krw,
            original_quote: QuoteCurrency::KRW,
        }
    }

    /// Create from KRW price with all conversion rates (legacy compatibility).
    /// This computes USDT-based usdlike for backward compatibility.
    ///
    /// # Arguments
    /// * `krw` - Raw price in KRW
    /// * `usdt_krw` - USDT/KRW rate from Korean exchange (e.g., 1430.0)
    /// * `usdc_krw` - USDC/KRW rate (unused in new system)
    /// * `usd_krw` - USD/KRW forex rate from 하나은행 (e.g., 1450.0)
    pub fn from_krw_with_rates(
        krw: FixedPoint,
        usdt_krw: f64,
        _usdc_krw: f64,
        usd_krw: f64,
    ) -> Self {
        let krw_f64 = krw.to_f64();
        Self {
            usdlike: if usdt_krw > 0.0 {
                Some(UsdlikePrice::new(
                    FixedPoint::from_f64(krw_f64 / usdt_krw),
                    UsdlikeQuote::USDT,
                ))
            } else {
                None
            },
            usd: if usd_krw > 0.0 {
                Some(FixedPoint::from_f64(krw_f64 / usd_krw))
            } else {
                None
            },
            raw: krw,
            original_quote: QuoteCurrency::KRW,
        }
    }

    /// Create from USD-like stablecoin price (USDT/USDC/BUSD market).
    ///
    /// # Arguments
    /// * `price` - Raw price in the stablecoin
    /// * `quote` - Which stablecoin (USDT, USDC, or BUSD)
    /// * `rate_to_usd` - Stablecoin to USD rate (e.g., USDT_USD = 0.9998)
    pub fn from_usdlike(price: FixedPoint, quote: UsdlikeQuote, rate_to_usd: f64) -> Self {
        let price_f64 = price.to_f64();
        Self {
            usdlike: Some(UsdlikePrice::new(price, quote)),
            usd: Some(FixedPoint::from_f64(price_f64 * rate_to_usd)),
            raw: price,
            original_quote: quote.to_quote_currency(),
        }
    }

    /// Create from USDT price (Binance, Bybit, Gate.io, etc.).
    ///
    /// # Arguments
    /// * `usdt` - Raw price in USDT
    /// * `usdt_usd` - USDT/USD rate (e.g., 0.9998)
    pub fn from_usdt(usdt: FixedPoint, usdt_usd: f64) -> Self {
        Self::from_usdlike(usdt, UsdlikeQuote::USDT, usdt_usd)
    }

    /// Create from USDC price.
    ///
    /// # Arguments
    /// * `usdc` - Raw price in USDC
    /// * `usdc_usd` - USDC/USD rate (e.g., 1.0001)
    pub fn from_usdc(usdc: FixedPoint, usdc_usd: f64) -> Self {
        Self::from_usdlike(usdc, UsdlikeQuote::USDC, usdc_usd)
    }

    /// Create from USD price (Coinbase, Kraken with USD pairs).
    /// Converts to USDT equivalent for USDlike comparison.
    ///
    /// # Arguments
    /// * `usd` - Raw price in USD
    /// * `usdt_usd` - USDT/USD rate (e.g., 0.9998)
    pub fn from_usd(usd: FixedPoint, usdt_usd: f64) -> Self {
        let usd_f64 = usd.to_f64();
        Self {
            usdlike: if usdt_usd > 0.0 {
                // Convert USD to USDT equivalent
                Some(UsdlikePrice::new(
                    FixedPoint::from_f64(usd_f64 / usdt_usd),
                    UsdlikeQuote::USDT,
                ))
            } else {
                None
            },
            usd: Some(usd),
            raw: usd,
            original_quote: QuoteCurrency::USD,
        }
    }

    /// Convert KRW price to USDlike at opportunity detection time.
    /// This is called when comparing KRW market with overseas market.
    ///
    /// # Arguments
    /// * `target_quote` - The overseas market's quote currency
    /// * `rates` - Conversion rates containing USDT/KRW, USDC/KRW, etc.
    pub fn to_usdlike(
        &self,
        target_quote: UsdlikeQuote,
        rates: &ConversionRates,
        exchange: Exchange,
    ) -> Option<UsdlikePrice> {
        // If usdlike is already set (even for KRW markets pre-converted to USDT), use it
        if let Some(existing) = self.usdlike {
            return Some(existing);
        }

        // KRW markets without pre-conversion need rate-based conversion
        if self.original_quote != QuoteCurrency::KRW {
            return None; // Non-KRW without usdlike shouldn't happen
        }

        // KRW price needs conversion based on target quote
        let krw_f64 = self.raw.to_f64();
        let rate = match target_quote {
            UsdlikeQuote::USDT => rates.usdt_krw_for(exchange),
            UsdlikeQuote::USDC => rates.usdc_krw_for(exchange),
            UsdlikeQuote::BUSD => return None, // No BUSD/KRW rate
        };

        if rate > 0.0 {
            Some(UsdlikePrice::new(
                FixedPoint::from_f64(krw_f64 / rate),
                target_quote,
            ))
        } else {
            None
        }
    }

    /// Get USDlike price if available.
    #[inline]
    pub fn usdlike_price(&self) -> Option<UsdlikePrice> {
        self.usdlike
    }

    /// Get USDT price if available (for backward compatibility).
    #[inline]
    pub fn usdt_price(&self) -> Option<FixedPoint> {
        self.usdlike.and_then(|p| {
            if p.quote == UsdlikeQuote::USDT {
                Some(p.price)
            } else {
                None
            }
        })
    }

    /// Get USDC price if available (for backward compatibility).
    #[inline]
    pub fn usdc_price(&self) -> Option<FixedPoint> {
        self.usdlike.and_then(|p| {
            if p.quote == UsdlikeQuote::USDC {
                Some(p.price)
            } else {
                None
            }
        })
    }

    /// Get USD price if available.
    #[inline]
    pub fn usd_price(&self) -> Option<FixedPoint> {
        self.usd
    }

    /// Get USDlike price value regardless of quote type (for backward compatibility).
    #[inline]
    pub fn usdt(&self) -> Option<FixedPoint> {
        self.usdlike.map(|p| p.price)
    }

    /// Alias for usdt() for backward compatibility.
    #[inline]
    pub fn usdc(&self) -> Option<FixedPoint> {
        self.usdlike.map(|p| p.price)
    }
}

/// Conversion rates needed for price denomination.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConversionRates {
    /// USDT/USD rate (stablecoin price in USD)
    pub usdt_usd: f64,
    /// USDC/USD rate (stablecoin price in USD)
    pub usdc_usd: f64,
    /// USD/KRW forex rate (하나은행)
    pub usd_krw: f64,
    /// USDT/KRW rate from Upbit
    pub upbit_usdt_krw: f64,
    /// USDC/KRW rate from Upbit
    pub upbit_usdc_krw: f64,
    /// USDT/KRW rate from Bithumb
    pub bithumb_usdt_krw: f64,
    /// USDC/KRW rate from Bithumb
    pub bithumb_usdc_krw: f64,
}

impl ConversionRates {
    /// Get USDT/KRW rate for a specific exchange.
    pub fn usdt_krw_for(&self, exchange: Exchange) -> f64 {
        match exchange {
            Exchange::Upbit => self.upbit_usdt_krw,
            Exchange::Bithumb => self.bithumb_usdt_krw,
            _ => 0.0,
        }
    }

    /// Get USDC/KRW rate for a specific exchange.
    pub fn usdc_krw_for(&self, exchange: Exchange) -> f64 {
        match exchange {
            Exchange::Upbit => self.upbit_usdc_krw,
            Exchange::Bithumb => self.bithumb_usdc_krw,
            _ => 0.0,
        }
    }
}

/// Premium calculation configuration.
#[derive(Debug, Clone)]
pub struct PremiumConfig {
    /// Minimum premium in basis points to consider profitable.
    pub min_premium_bps: i32,
    /// Maximum age of price data before considering it stale (ms).
    pub max_staleness_ms: u64,
    /// Trading fee in basis points per trade.
    pub trading_fee_bps: i32,
    /// Estimated gas cost in basis points.
    pub gas_cost_bps: i32,
}

impl Default for PremiumConfig {
    fn default() -> Self {
        Self {
            min_premium_bps: 30,     // 0.3%
            max_staleness_ms: 30000, // 30 seconds
            trading_fee_bps: 10,     // 0.1% per trade
            gas_cost_bps: 5,         // 0.05%
        }
    }
}

impl PremiumConfig {
    /// Check if a premium is profitable after costs.
    pub fn is_profitable(&self, premium_bps: i32) -> bool {
        premium_bps >= self.min_premium_bps
    }

    /// Calculate net profit after fees.
    pub fn net_profit_bps(&self, gross_premium_bps: i32) -> i32 {
        gross_premium_bps - (2 * self.trading_fee_bps) - self.gas_cost_bps
    }
}

/// Premium entry for a single exchange with multi-denomination prices.
#[derive(Debug, Clone, Copy)]
struct PriceEntry {
    /// Mid price in all denominations
    mid: DenominatedPrices,
    /// Best bid price (highest buy order) in all denominations
    bid: DenominatedPrices,
    /// Best ask price (lowest sell order) in all denominations
    ask: DenominatedPrices,
    /// Best bid size (quantity available at best bid)
    bid_size: FixedPoint,
    /// Best ask size (quantity available at best ask)
    ask_size: FixedPoint,
    /// Timestamp when this price was recorded (ms since epoch)
    timestamp_ms: u64,
}

impl PriceEntry {
    /// Check if this price entry is stale (older than max_staleness_ms).
    fn is_stale(&self, max_staleness_ms: u64) -> bool {
        if max_staleness_ms == 0 {
            return false; // Staleness check disabled
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        now.saturating_sub(self.timestamp_ms) > max_staleness_ms
    }
}

/// Premium matrix for calculating arbitrage between exchanges.
/// Now stores prices in multiple denominations for accurate premium comparison.
#[derive(Debug, Clone)]
pub struct PremiumMatrix {
    pair_id: u32,
    /// Key: exchange_id (one entry per exchange with all denominations)
    prices: HashMap<u16, PriceEntry>,
    /// Maximum staleness in milliseconds (0 = disabled)
    max_staleness_ms: u64,
}

impl PremiumMatrix {
    /// Create a new premium matrix for a trading pair.
    pub fn new(pair_id: u32) -> Self {
        Self {
            pair_id,
            prices: HashMap::new(),
            max_staleness_ms: 30000, // Default 30 seconds
        }
    }

    /// Create a new premium matrix with custom staleness threshold.
    pub fn with_staleness(pair_id: u32, max_staleness_ms: u64) -> Self {
        Self {
            pair_id,
            prices: HashMap::new(),
            max_staleness_ms,
        }
    }

    /// Set the maximum staleness threshold.
    pub fn set_max_staleness_ms(&mut self, max_staleness_ms: u64) {
        self.max_staleness_ms = max_staleness_ms;
    }

    /// Remove stale price entries. Returns the number of entries removed.
    pub fn expire_stale_prices(&mut self) -> usize {
        if self.max_staleness_ms == 0 {
            return 0;
        }
        let before = self.prices.len();
        self.prices.retain(|_, entry| !entry.is_stale(self.max_staleness_ms));
        before - self.prices.len()
    }

    /// Clear all prices for a specific exchange.
    /// Call this on reconnection to avoid using stale cached data.
    pub fn clear_exchange(&mut self, exchange: Exchange) {
        self.prices.remove(&(exchange as u16));
    }

    /// Clear all prices. Call this when major state reset is needed.
    pub fn clear_all(&mut self) {
        self.prices.clear();
    }

    /// Get the pair ID.
    pub fn pair_id(&self) -> u32 {
        self.pair_id
    }

    /// Check if the matrix has no prices.
    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }

    /// Get the number of exchanges with prices.
    pub fn exchange_count(&self) -> usize {
        self.prices.len()
    }

    /// Update price for an exchange with default quote currency (USD).
    /// Legacy method - prefer update_price_with_denominations for new code.
    pub fn update_price(&mut self, exchange: Exchange, price: FixedPoint) {
        // Create DenominatedPrices with USD/USDT (legacy behavior assumes 1:1)
        let denominated = DenominatedPrices {
            usdlike: Some(UsdlikePrice::new(price, UsdlikeQuote::USDT)),
            usd: Some(price),
            raw: price,
            original_quote: QuoteCurrency::USD,
        };
        self.update_price_with_denominations(
            exchange,
            denominated,
            denominated,
            denominated,
            FixedPoint::from_f64(0.0),
            FixedPoint::from_f64(0.0),
        );
    }

    /// Update price for an exchange with specified quote currency.
    /// Legacy method - prefer update_price_with_denominations for new code.
    pub fn update_price_with_quote(
        &mut self,
        exchange: Exchange,
        price: FixedPoint,
        quote: QuoteCurrency,
    ) {
        let usdlike = UsdlikeQuote::from_quote_currency(quote).map(|q| UsdlikePrice::new(price, q));
        let denominated = DenominatedPrices {
            usdlike,
            usd: Some(price),
            raw: price,
            original_quote: quote,
        };
        self.update_price_with_denominations(
            exchange,
            denominated,
            denominated,
            denominated,
            FixedPoint::from_f64(0.0),
            FixedPoint::from_f64(0.0),
        );
    }

    /// Update price for an exchange with bid/ask from orderbook (legacy).
    /// For USD quotes, treats prices as USDT equivalent (1:1) for USDlike comparison.
    /// For KRW quotes, assumes prices are already converted to USDT via USDT/KRW rate.
    pub fn update_price_with_bid_ask(
        &mut self,
        exchange: Exchange,
        price: FixedPoint,
        bid: FixedPoint,
        ask: FixedPoint,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
        quote: QuoteCurrency,
    ) {
        // For USD and KRW quotes, treat as USDT equivalent for USDlike comparison
        // KRW prices are already converted to USDT in main.rs via USDT/KRW rate
        let usdlike_quote = UsdlikeQuote::from_quote_currency(quote).or_else(|| {
            if quote == QuoteCurrency::USD || quote == QuoteCurrency::KRW {
                Some(UsdlikeQuote::USDT)
            } else {
                None
            }
        });

        let mid_denom = DenominatedPrices {
            usdlike: usdlike_quote.map(|q| UsdlikePrice::new(price, q)),
            usd: Some(price),
            raw: price,
            original_quote: quote,
        };
        let bid_denom = DenominatedPrices {
            usdlike: usdlike_quote.map(|q| UsdlikePrice::new(bid, q)),
            usd: Some(bid),
            raw: bid,
            original_quote: quote,
        };
        let ask_denom = DenominatedPrices {
            usdlike: usdlike_quote.map(|q| UsdlikePrice::new(ask, q)),
            usd: Some(ask),
            raw: ask,
            original_quote: quote,
        };
        self.update_price_with_denominations(
            exchange, mid_denom, bid_denom, ask_denom, bid_size, ask_size,
        );
    }

    /// Update price for an exchange with full multi-denomination support.
    /// This is the primary method for the new denomination system.
    pub fn update_price_with_denominations(
        &mut self,
        exchange: Exchange,
        mid: DenominatedPrices,
        bid: DenominatedPrices,
        ask: DenominatedPrices,
        bid_size: FixedPoint,
        ask_size: FixedPoint,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let key = exchange as u16;

        // Preserve previous depth values if new ones are zero
        let (final_bid_size, final_ask_size) = if let Some(existing) = self.prices.get(&key) {
            let new_bid_size = if bid_size.0 == 0 {
                existing.bid_size
            } else {
                bid_size
            };
            let new_ask_size = if ask_size.0 == 0 {
                existing.ask_size
            } else {
                ask_size
            };
            (new_bid_size, new_ask_size)
        } else {
            (bid_size, ask_size)
        };

        self.prices.insert(
            key,
            PriceEntry {
                mid,
                bid,
                ask,
                bid_size: final_bid_size,
                ask_size: final_ask_size,
                timestamp_ms: now,
            },
        );
    }

    /// Get raw price for an exchange (original quote currency).
    pub fn get_price(&self, exchange: Exchange) -> Option<FixedPoint> {
        self.prices.get(&(exchange as u16)).map(|e| e.mid.raw)
    }

    /// Get USDlike-denominated price for an exchange.
    pub fn get_usdlike_price(&self, exchange: Exchange) -> Option<UsdlikePrice> {
        self.prices
            .get(&(exchange as u16))
            .and_then(|e| e.mid.usdlike)
    }

    /// Get USDT-denominated price for an exchange (legacy compatibility).
    pub fn get_usdt_price(&self, exchange: Exchange) -> Option<FixedPoint> {
        self.prices
            .get(&(exchange as u16))
            .and_then(|e| e.mid.usdlike.map(|p| p.price))
    }

    /// Get USDC-denominated price for an exchange (legacy compatibility).
    pub fn get_usdc_price(&self, exchange: Exchange) -> Option<FixedPoint> {
        self.prices
            .get(&(exchange as u16))
            .and_then(|e| e.mid.usdlike.map(|p| p.price))
    }

    /// Get USD-denominated price for an exchange.
    pub fn get_usd_price(&self, exchange: Exchange) -> Option<FixedPoint> {
        self.prices.get(&(exchange as u16)).and_then(|e| e.mid.usd)
    }

    /// Get price for an exchange with specific quote currency (legacy).
    pub fn get_price_with_quote(
        &self,
        exchange: Exchange,
        _quote: QuoteCurrency,
    ) -> Option<FixedPoint> {
        self.get_price(exchange)
    }

    /// Get bid price for an exchange (raw).
    pub fn get_bid_with_quote(
        &self,
        exchange: Exchange,
        _quote: QuoteCurrency,
    ) -> Option<FixedPoint> {
        self.prices.get(&(exchange as u16)).map(|e| e.bid.raw)
    }

    /// Get ask price for an exchange (raw).
    pub fn get_ask_with_quote(
        &self,
        exchange: Exchange,
        _quote: QuoteCurrency,
    ) -> Option<FixedPoint> {
        self.prices.get(&(exchange as u16)).map(|e| e.ask.raw)
    }

    /// Get quote currency for an exchange.
    pub fn get_quote(&self, exchange: Exchange) -> Option<QuoteCurrency> {
        self.prices
            .get(&(exchange as u16))
            .map(|e| e.mid.original_quote)
    }

    // ============ Denomination-specific premium methods ============

    /// Calculate USDlike premium between buy and sell exchanges.
    /// Compares prices in the same stablecoin (USDT vs USDT or USDC vs USDC).
    /// For KRW markets, converts to the overseas market's quote currency.
    ///
    /// Returns (premium_bps, quote) where quote is the stablecoin used for comparison.
    pub fn usdlike_premium(
        &self,
        buy_exchange: Exchange,
        sell_exchange: Exchange,
        rates: &ConversionRates,
    ) -> Option<(i32, UsdlikeQuote)> {
        let buy_entry = self.prices.get(&(buy_exchange as u16))?;
        let sell_entry = self.prices.get(&(sell_exchange as u16))?;

        // Determine target quote from overseas market (non-KRW)
        let target_quote = if buy_entry.ask.original_quote == QuoteCurrency::KRW {
            sell_entry.bid.usdlike?.quote
        } else {
            buy_entry.ask.usdlike?.quote
        };

        // Convert KRW prices to target quote if needed
        let buy_usdlike = buy_entry
            .ask
            .to_usdlike(target_quote, rates, buy_exchange)?;
        let sell_usdlike = sell_entry
            .bid
            .to_usdlike(target_quote, rates, sell_exchange)?;

        let premium = FixedPoint::premium_bps(buy_usdlike.price, sell_usdlike.price);
        Some((premium, target_quote))
    }

    /// Calculate Tether premium between buy and sell exchanges (legacy compatibility).
    /// Uses usdlike_premium internally.
    pub fn tether_premium(&self, buy_exchange: Exchange, sell_exchange: Exchange) -> Option<i32> {
        let buy_entry = self.prices.get(&(buy_exchange as u16))?;
        let sell_entry = self.prices.get(&(sell_exchange as u16))?;

        let buy_ask_usdlike = buy_entry.ask.usdlike?;
        let sell_bid_usdlike = sell_entry.bid.usdlike?;

        Some(FixedPoint::premium_bps(
            buy_ask_usdlike.price,
            sell_bid_usdlike.price,
        ))
    }

    /// Calculate USDC premium between buy and sell exchanges (legacy compatibility).
    /// Uses usdlike_premium internally.
    pub fn usdc_premium(&self, buy_exchange: Exchange, sell_exchange: Exchange) -> Option<i32> {
        // Same as tether_premium since we now use unified usdlike
        self.tether_premium(buy_exchange, sell_exchange)
    }

    /// Calculate Kimchi premium between buy and sell exchanges.
    /// Compares USD prices (KRW via forex rate): (sell_bid_usd - buy_ask_usd) / buy_ask_usd * 10000
    pub fn kimchi_premium(&self, buy_exchange: Exchange, sell_exchange: Exchange) -> Option<i32> {
        let buy_entry = self.prices.get(&(buy_exchange as u16))?;
        let sell_entry = self.prices.get(&(sell_exchange as u16))?;

        let buy_ask_usd = buy_entry.ask.usd?;
        let sell_bid_usd = sell_entry.bid.usd?;

        Some(FixedPoint::premium_bps(buy_ask_usd, sell_bid_usd))
    }

    /// Calculate premium between buy and sell exchanges using USDlike prices.
    /// Uses ask price for buying and bid price for selling.
    pub fn get_premium(&self, buy_exchange: Exchange, sell_exchange: Exchange) -> Option<i32> {
        self.tether_premium(buy_exchange, sell_exchange)
    }

    /// Find the best arbitrage opportunity using USDlike prices.
    /// Returns (buy_exchange, sell_exchange, premium_bps).
    pub fn best_opportunity(&self) -> Option<(Exchange, Exchange, i32)> {
        if self.prices.len() < 2 {
            return None;
        }

        let mut best: Option<(Exchange, Exchange, i32)> = None;

        for (&buy_ex_id, buy_entry) in &self.prices {
            for (&sell_ex_id, sell_entry) in &self.prices {
                if buy_ex_id == sell_ex_id {
                    continue;
                }

                // Use USDlike prices for comparison
                let buy_ask = buy_entry.ask.usdlike.map(|p| p.price);
                let sell_bid = sell_entry.bid.usdlike.map(|p| p.price);
                if let (Some(buy_ask), Some(sell_bid)) = (buy_ask, sell_bid) {
                    let premium = FixedPoint::premium_bps(buy_ask, sell_bid);

                    if best.is_none() || premium > best.as_ref().unwrap().2 {
                        if let (Some(buy_ex), Some(sell_ex)) =
                            (Exchange::from_id(buy_ex_id), Exchange::from_id(sell_ex_id))
                        {
                            best = Some((buy_ex, sell_ex, premium));
                        }
                    }
                }
            }
        }

        best
    }

    /// Get all premium pairs (using USDlike).
    pub fn all_premiums(&self) -> Vec<(Exchange, Exchange, i32)> {
        self.all_premiums_with_quotes()
            .into_iter()
            .map(|(buy_ex, sell_ex, _, _, premium)| (buy_ex, sell_ex, premium))
            .collect()
    }

    /// Get all premium pairs with quote currencies (using USDlike prices).
    pub fn all_premiums_with_quotes(
        &self,
    ) -> Vec<(Exchange, Exchange, QuoteCurrency, QuoteCurrency, i32)> {
        let mut result = Vec::new();

        for (&buy_ex_id, buy_entry) in &self.prices {
            for (&sell_ex_id, sell_entry) in &self.prices {
                if buy_ex_id == sell_ex_id {
                    continue;
                }

                if let (Some(buy_ex), Some(sell_ex)) =
                    (Exchange::from_id(buy_ex_id), Exchange::from_id(sell_ex_id))
                {
                    let buy_ask = buy_entry.ask.usdlike.map(|p| p.price);
                    let sell_bid = sell_entry.bid.usdlike.map(|p| p.price);
                    if let (Some(buy_ask), Some(sell_bid)) = (buy_ask, sell_bid) {
                        let premium = FixedPoint::premium_bps(buy_ask, sell_bid);
                        result.push((
                            buy_ex,
                            sell_ex,
                            buy_entry.mid.original_quote,
                            sell_entry.mid.original_quote,
                            premium,
                        ));
                    }
                }
            }
        }

        result
    }

    /// Get all premium pairs with bid/ask prices (using USDlike).
    pub fn all_premiums_with_bid_ask(
        &self,
    ) -> Vec<(
        Exchange,
        Exchange,
        QuoteCurrency,
        QuoteCurrency,
        FixedPoint,
        FixedPoint,
        i32,
    )> {
        self.all_premiums_with_depth()
            .into_iter()
            .map(
                |(buy_ex, sell_ex, buy_quote, sell_quote, buy_ask, sell_bid, _, _, premium, _, _)| {
                    (
                        buy_ex, sell_ex, buy_quote, sell_quote, buy_ask, sell_bid, premium,
                    )
                },
            )
            .collect()
    }

    /// Get all premium pairs with full depth information.
    /// Now returns USDlike-denominated prices for accurate comparison.
    /// Automatically filters out stale prices.
    /// Returns: (buy_ex, sell_ex, buy_quote, sell_quote, buy_ask, sell_bid, buy_size, sell_size, premium, buy_timestamp_ms, sell_timestamp_ms)
    #[allow(clippy::type_complexity)]
    pub fn all_premiums_with_depth(
        &self,
    ) -> Vec<(
        Exchange,
        Exchange,
        QuoteCurrency,
        QuoteCurrency,
        FixedPoint,
        FixedPoint,
        FixedPoint,
        FixedPoint,
        i32,
        u64, // buy_timestamp_ms
        u64, // sell_timestamp_ms
    )> {
        let mut result = Vec::new();

        for (&buy_ex_id, buy_entry) in &self.prices {
            // Skip stale buy entries
            if buy_entry.is_stale(self.max_staleness_ms) {
                continue;
            }

            for (&sell_ex_id, sell_entry) in &self.prices {
                if buy_ex_id == sell_ex_id {
                    continue;
                }

                // Skip stale sell entries
                if sell_entry.is_stale(self.max_staleness_ms) {
                    continue;
                }

                if let (Some(buy_ex), Some(sell_ex)) =
                    (Exchange::from_id(buy_ex_id), Exchange::from_id(sell_ex_id))
                {
                    let buy_ask = buy_entry.ask.usdlike.map(|p| p.price);
                    let sell_bid = sell_entry.bid.usdlike.map(|p| p.price);
                    if let (Some(buy_ask), Some(sell_bid)) = (buy_ask, sell_bid) {
                        let premium = FixedPoint::premium_bps(buy_ask, sell_bid);
                        result.push((
                            buy_ex,
                            sell_ex,
                            buy_entry.mid.original_quote,
                            sell_entry.mid.original_quote,
                            buy_ask,
                            sell_bid,
                            buy_entry.ask_size,
                            sell_entry.bid_size,
                            premium,
                            buy_entry.timestamp_ms,
                            sell_entry.timestamp_ms,
                        ));
                    }
                }
            }
        }

        result
    }

    /// Get all premium pairs with USDlike and Kimchi premiums.
    /// Returns (buy_ex, sell_ex, quotes, prices, raw_prices, depth, usdlike_premium, usdlike_quote, kimchi_premium, timestamps).
    ///
    /// For KRW ↔ overseas opportunities, KRW prices are converted to the overseas market's
    /// quote currency (USDT or USDC) using `to_usdlike()`.
    #[allow(clippy::type_complexity)]
    pub fn all_premiums_multi_denomination(
        &self,
        rates: &ConversionRates,
    ) -> Vec<(
        Exchange,
        Exchange,
        QuoteCurrency,
        QuoteCurrency,
        FixedPoint,
        FixedPoint, // buy_ask (USDlike), sell_bid (USDlike)
        FixedPoint,
        FixedPoint, // buy_ask_raw, sell_bid_raw (original exchange prices)
        FixedPoint,
        FixedPoint, // bid_size, ask_size
        i32,        // usdlike_premium (same as tether/usdc)
        i32,        // (unused, kept for compatibility)
        i32,        // kimchi_premium
        u64,        // buy_timestamp_ms
        u64,        // sell_timestamp_ms
    )> {
        let mut result = Vec::new();

        for (&buy_ex_id, buy_entry) in &self.prices {
            // Skip stale buy entries
            if buy_entry.is_stale(self.max_staleness_ms) {
                continue;
            }

            for (&sell_ex_id, sell_entry) in &self.prices {
                if buy_ex_id == sell_ex_id {
                    continue;
                }

                // Skip stale sell entries
                if sell_entry.is_stale(self.max_staleness_ms) {
                    continue;
                }

                if let (Some(buy_ex), Some(sell_ex)) =
                    (Exchange::from_id(buy_ex_id), Exchange::from_id(sell_ex_id))
                {
                    // Determine target quote from overseas market (non-KRW side)
                    let target_quote = if buy_entry.ask.original_quote == QuoteCurrency::KRW {
                        // Buy side is KRW, use sell side's quote
                        sell_entry.bid.usdlike.map(|p| p.quote)
                    } else if sell_entry.bid.original_quote == QuoteCurrency::KRW {
                        // Sell side is KRW, use buy side's quote
                        buy_entry.ask.usdlike.map(|p| p.quote)
                    } else {
                        // Neither is KRW, use buy side's quote (both should have usdlike)
                        buy_entry.ask.usdlike.map(|p| p.quote)
                    };

                    // Calculate USDlike premium with proper conversion
                    let (usdlike_premium, buy_ask_usdlike, sell_bid_usdlike) =
                        if let Some(target) = target_quote {
                            // Convert KRW prices to target quote if needed
                            let buy_usdlike = buy_entry.ask.to_usdlike(target, rates, buy_ex);
                            let sell_usdlike = sell_entry.bid.to_usdlike(target, rates, sell_ex);

                            match (buy_usdlike, sell_usdlike) {
                                (Some(buy), Some(sell)) => {
                                    let premium = FixedPoint::premium_bps(buy.price, sell.price);
                                    (premium, buy.price, sell.price)
                                }
                                _ => (0, FixedPoint(0), FixedPoint(0)),
                            }
                        } else {
                            // No valid target quote (shouldn't happen normally)
                            (0, FixedPoint(0), FixedPoint(0))
                        };

                    // Kimchi premium (USD via forex)
                    let kimchi_premium = match (buy_entry.ask.usd, sell_entry.bid.usd) {
                        (Some(buy), Some(sell)) => FixedPoint::premium_bps(buy, sell),
                        _ => 0,
                    };

                    result.push((
                        buy_ex,
                        sell_ex,
                        buy_entry.mid.original_quote,
                        sell_entry.mid.original_quote,
                        buy_ask_usdlike,
                        sell_bid_usdlike,
                        buy_entry.ask.raw,  // Original exchange price for buy (ask)
                        sell_entry.bid.raw, // Original exchange price for sell (bid)
                        buy_entry.ask_size,
                        sell_entry.bid_size,
                        usdlike_premium,
                        usdlike_premium, // Same value for backward compatibility
                        kimchi_premium,
                        buy_entry.timestamp_ms,
                        sell_entry.timestamp_ms,
                    ));
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_premium_matrix_new() {
        let matrix = PremiumMatrix::new(1); // pair_id = 1
        assert_eq!(matrix.pair_id(), 1);
        assert!(matrix.is_empty());
    }

    #[test]
    fn test_premium_matrix_update() {
        let mut matrix = PremiumMatrix::new(1);

        matrix.update_price(Exchange::Binance, FixedPoint::from_f64(50000.0));
        matrix.update_price(Exchange::Coinbase, FixedPoint::from_f64(50500.0));

        assert!(!matrix.is_empty());
        assert_eq!(matrix.exchange_count(), 2);
    }

    #[test]
    fn test_premium_matrix_get_premium() {
        let mut matrix = PremiumMatrix::new(1);

        matrix.update_price(Exchange::Binance, FixedPoint::from_f64(50000.0));
        matrix.update_price(Exchange::Coinbase, FixedPoint::from_f64(50500.0));

        // Buy at Binance ($50,000), sell at Coinbase ($50,500)
        // Premium = (50500 - 50000) / 50000 * 10000 = 100 bps
        let premium = matrix.get_premium(Exchange::Binance, Exchange::Coinbase);
        assert_eq!(premium, Some(100));

        // Reverse: negative premium
        let premium = matrix.get_premium(Exchange::Coinbase, Exchange::Binance);
        assert!(premium.unwrap() < 0);
    }

    #[test]
    fn test_premium_matrix_best_pair() {
        let mut matrix = PremiumMatrix::new(1);

        matrix.update_price(Exchange::Binance, FixedPoint::from_f64(50000.0));
        matrix.update_price(Exchange::Coinbase, FixedPoint::from_f64(50500.0));
        matrix.update_price(Exchange::Kraken, FixedPoint::from_f64(49800.0));

        let (buy, sell, premium) = matrix.best_opportunity().unwrap();

        // Best: Buy at Kraken ($49,800), sell at Coinbase ($50,500)
        assert_eq!(buy, Exchange::Kraken);
        assert_eq!(sell, Exchange::Coinbase);
        assert!(premium > 100); // > 1%
    }

    #[test]
    fn test_premium_matrix_all_premiums() {
        let mut matrix = PremiumMatrix::new(1);

        matrix.update_price(Exchange::Binance, FixedPoint::from_f64(50000.0));
        matrix.update_price(Exchange::Coinbase, FixedPoint::from_f64(50500.0));
        matrix.update_price(Exchange::Kraken, FixedPoint::from_f64(49800.0));

        let premiums = matrix.all_premiums();
        // 3 exchanges = 3 * 2 = 6 pairs (buy/sell combinations)
        assert_eq!(premiums.len(), 6);
    }

    #[test]
    fn test_premium_config() {
        let config = PremiumConfig::default();
        assert!(config.min_premium_bps > 0);
        assert!(config.max_staleness_ms > 0);
    }

    #[test]
    fn test_premium_config_is_profitable() {
        let config = PremiumConfig {
            min_premium_bps: 50,
            ..Default::default()
        };

        assert!(config.is_profitable(100)); // 100 bps > 50 bps
        assert!(!config.is_profitable(30)); // 30 bps < 50 bps
    }
}
