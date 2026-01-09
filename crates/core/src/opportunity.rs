//! Arbitrage opportunity detection and route types.

use crate::{Asset, BridgeProtocol, Chain, Exchange, FixedPoint, QuoteCurrency};
use serde::{Deserialize, Serialize};

/// Reason for optimal_size value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimalSizeReason {
    /// Optimal size calculated successfully with profit.
    #[default]
    Ok,
    /// Missing orderbook data for one or both exchanges.
    NoOrderbook,
    /// Orderbook available but trade is not profitable after fees.
    NotProfitable,
    /// Missing KRW conversion rate for cross-currency calculation.
    NoConversionRate,
}

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

/// USD-like premium (USDT vs USDT or USDC vs USDC comparison).
/// This is mutually exclusive - an opportunity uses either USDT, USDC, or BUSD for comparison.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UsdlikePremium {
    /// Premium in basis points
    pub bps: i32,
    /// Which USD-like stablecoin was used for comparison
    pub quote: UsdlikeQuote,
}

/// Trade direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TradeSide {
    Buy = 0,
    Sell = 1,
}

impl TradeSide {
    pub fn opposite(self) -> Self {
        match self {
            TradeSide::Buy => TradeSide::Sell,
            TradeSide::Sell => TradeSide::Buy,
        }
    }
}

/// A single step in an arbitrage route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouteStep {
    Trade {
        exchange: Exchange,
        pair_id: u32,
        side: TradeSide,
        expected_price: u64,
        slippage_bps: u16,
    },
    Bridge {
        protocol: BridgeProtocol,
        source_chain: Chain,
        dest_chain: Chain,
    },
    Withdraw {
        exchange: Exchange,
        chain: Chain,
    },
    Deposit {
        exchange: Exchange,
        chain: Chain,
    },
}

impl RouteStep {
    pub fn trade(
        exchange: Exchange,
        pair_id: u32,
        side: TradeSide,
        expected_price: FixedPoint,
        slippage_bps: u16,
    ) -> Self {
        RouteStep::Trade {
            exchange,
            pair_id,
            side,
            expected_price: expected_price.0,
            slippage_bps,
        }
    }

    pub fn bridge(protocol: BridgeProtocol, source_chain: Chain, dest_chain: Chain) -> Self {
        RouteStep::Bridge {
            protocol,
            source_chain,
            dest_chain,
        }
    }

    pub fn withdraw(exchange: Exchange, chain: Chain) -> Self {
        RouteStep::Withdraw { exchange, chain }
    }

    pub fn deposit(exchange: Exchange, chain: Chain) -> Self {
        RouteStep::Deposit { exchange, chain }
    }
}

/// Premium between two exchanges for a single asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangePairPremium {
    pub buy_exchange: Exchange,
    pub sell_exchange: Exchange,
    pub buy_price: u64,
    pub sell_price: u64,
    /// Premium in basis points: (sell - buy) / buy * 10000
    pub premium_bps: i32,
    /// Net profit after fees (in bps)
    pub net_profit_bps: i32,
}

impl ExchangePairPremium {
    pub fn new(
        buy_exchange: Exchange,
        sell_exchange: Exchange,
        buy_price: FixedPoint,
        sell_price: FixedPoint,
    ) -> Self {
        let premium_bps = FixedPoint::premium_bps(buy_price, sell_price);
        Self {
            buy_exchange,
            sell_exchange,
            buy_price: buy_price.0,
            sell_price: sell_price.0,
            premium_bps,
            net_profit_bps: premium_bps, // TODO: subtract fees
        }
    }
}

/// Detected arbitrage opportunity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub id: u64,
    pub discovered_at_ms: u64,
    pub expires_at_ms: u64,

    // Price info
    pub source_exchange: Exchange,
    pub target_exchange: Exchange,
    /// Quote currency at source exchange (e.g., USDT, USDC, KRW)
    pub source_quote: QuoteCurrency,
    /// Quote currency at target exchange (e.g., USDT, USDC, KRW)
    pub target_quote: QuoteCurrency,
    pub asset: Asset,
    pub source_price: u64,
    pub target_price: u64,
    /// Orderbook depth at source (ask size - how much we can buy)
    pub source_depth: u64,
    /// Orderbook depth at target (bid size - how much we can sell)
    pub target_depth: u64,
    /// Raw premium in basis points (direct price comparison, no currency conversion)
    pub premium_bps: i32,
    /// USD-like premium: same stablecoin comparison (USDT vs USDT or USDC vs USDC)
    /// For KRW markets, converts to overseas market's quote currency.
    /// None if the overseas market doesn't use USDT/USDC/BUSD.
    pub usdlike_premium: Option<UsdlikePremium>,
    /// Kimchi premium: USD price comparison (KRW via forex rate)
    /// Korean price / USD_KRW (하나은행) vs Overseas USD price
    pub kimchi_premium_bps: i32,

    // Execution route
    pub route: Vec<RouteStep>,
    pub total_hops: u8,

    // Cost analysis
    pub estimated_gas_cost: u64,
    pub estimated_bridge_fee: u64,
    pub estimated_trading_fee: u64,
    /// Net profit estimate (can be negative)
    pub net_profit_estimate: i64,

    // Execution conditions
    pub min_amount: u64,
    pub max_amount: u64,
    /// Confidence score 0-100
    pub confidence_score: u8,

    // Optimal execution sizing (from orderbook depth analysis)
    /// Optimal trade size calculated from orderbook depth walking.
    /// Represents maximum profitable amount considering depth and fees.
    pub optimal_size: u64,
    /// Expected profit at optimal_size (in quote currency, FixedPoint scale).
    /// Already accounts for trading fees and withdrawal costs.
    pub optimal_profit: i64,
    /// Reason for optimal_size value (ok, no_orderbook, not_profitable).
    #[serde(default)]
    pub optimal_size_reason: OptimalSizeReason,

    // Price timestamps
    /// Timestamp when source price was recorded (ms since epoch)
    #[serde(default)]
    pub source_price_timestamp_ms: u64,
    /// Timestamp when target price was recorded (ms since epoch)
    #[serde(default)]
    pub target_price_timestamp_ms: u64,
}

impl ArbitrageOpportunity {
    /// Create a new opportunity with default quote currencies (USD).
    pub fn new(
        id: u64,
        source_exchange: Exchange,
        target_exchange: Exchange,
        asset: Asset,
        source_price: FixedPoint,
        target_price: FixedPoint,
    ) -> Self {
        Self::with_quotes(
            id,
            source_exchange,
            target_exchange,
            QuoteCurrency::USD,
            QuoteCurrency::USD,
            asset,
            source_price,
            target_price,
        )
    }

    /// Create a new opportunity with specified quote currencies.
    pub fn with_quotes(
        id: u64,
        source_exchange: Exchange,
        target_exchange: Exchange,
        source_quote: QuoteCurrency,
        target_quote: QuoteCurrency,
        asset: Asset,
        source_price: FixedPoint,
        target_price: FixedPoint,
    ) -> Self {
        Self::with_quotes_and_rates(
            id,
            source_exchange,
            target_exchange,
            source_quote,
            target_quote,
            asset,
            source_price,
            target_price,
            None,
            None,
        )
    }

    /// Create a new opportunity with quote currencies and exchange rates for premium calculation.
    /// - `usd_krw_rate`: USD/KRW exchange rate (e.g., 1450.0 means 1 USD = 1450 KRW)
    /// - `usdt_krw_rate`: USDT/KRW rate from Korean exchange (e.g., 1455.0)
    pub fn with_quotes_and_rates(
        id: u64,
        source_exchange: Exchange,
        target_exchange: Exchange,
        source_quote: QuoteCurrency,
        target_quote: QuoteCurrency,
        asset: Asset,
        source_price: FixedPoint,
        target_price: FixedPoint,
        usd_krw_rate: Option<f64>,
        usdt_krw_rate: Option<f64>,
    ) -> Self {
        Self::with_all_rates(
            id,
            source_exchange,
            target_exchange,
            source_quote,
            target_quote,
            asset,
            source_price,
            target_price,
            usd_krw_rate,
            usdt_krw_rate,
            usdt_krw_rate, // Use USDT rate as fallback for USDC
        )
    }

    /// Create a new opportunity with all exchange rates for multi-denomination premium calculation.
    pub fn with_all_rates(
        id: u64,
        source_exchange: Exchange,
        target_exchange: Exchange,
        source_quote: QuoteCurrency,
        target_quote: QuoteCurrency,
        asset: Asset,
        source_price: FixedPoint,
        target_price: FixedPoint,
        usd_krw_rate: Option<f64>,
        usdt_krw_rate: Option<f64>,
        usdc_krw_rate: Option<f64>,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let premium_bps = FixedPoint::premium_bps(source_price, target_price);

        // Calculate multi-denomination premiums
        let (usdlike_premium, kimchi_premium_bps) = Self::calculate_multi_premiums(
            source_quote,
            target_quote,
            source_price,
            target_price,
            usd_krw_rate,
            usdt_krw_rate,
            usdc_krw_rate,
        );

        Self {
            id,
            discovered_at_ms: now,
            expires_at_ms: now + 30_000, // 30 seconds default
            source_exchange,
            target_exchange,
            source_quote,
            target_quote,
            asset,
            source_price: source_price.0,
            target_price: target_price.0,
            source_depth: 0,
            target_depth: 0,
            premium_bps,
            usdlike_premium,
            kimchi_premium_bps,
            route: Vec::new(),
            total_hops: 0,
            estimated_gas_cost: 0,
            estimated_bridge_fee: 0,
            estimated_trading_fee: 0,
            net_profit_estimate: 0,
            min_amount: 0,
            max_amount: u64::MAX,
            confidence_score: 50,
            optimal_size: 0,
            optimal_profit: 0,
            optimal_size_reason: OptimalSizeReason::default(),
            source_price_timestamp_ms: 0,
            target_price_timestamp_ms: 0,
        }
    }

    /// Set optimal execution size (builder pattern).
    pub fn with_optimal_size(mut self, optimal_size: u64, optimal_profit: i64) -> Self {
        self.optimal_size = optimal_size;
        self.optimal_profit = optimal_profit;
        self
    }

    /// Set orderbook depth (builder pattern).
    pub fn with_depth(mut self, source_depth: FixedPoint, target_depth: FixedPoint) -> Self {
        self.source_depth = source_depth.0;
        self.target_depth = target_depth.0;
        self
    }

    /// Set price timestamps (builder pattern).
    pub fn with_price_timestamps(
        mut self,
        source_timestamp_ms: u64,
        target_timestamp_ms: u64,
    ) -> Self {
        self.source_price_timestamp_ms = source_timestamp_ms;
        self.target_price_timestamp_ms = target_timestamp_ms;
        self
    }

    /// Calculate multi-denomination premiums for opportunities.
    /// Returns (usdlike_premium, kimchi_premium_bps).
    ///
    /// The new system stores prices directly in USDT/USDC denominations,
    /// so premiums are calculated by direct comparison.
    ///
    /// - USDlike Premium: same stablecoin comparison (USDT vs USDT or USDC vs USDC)
    ///   For KRW markets, converts to the overseas market's quote currency.
    /// - Kimchi Premium: USD price comparison (KRW / USD_KRW forex vs overseas USD)
    fn calculate_multi_premiums(
        source_quote: QuoteCurrency,
        target_quote: QuoteCurrency,
        source_price: FixedPoint,
        target_price: FixedPoint,
        usd_krw_rate: Option<f64>,
        usdt_krw_rate: Option<f64>,
        usdc_krw_rate: Option<f64>,
    ) -> (Option<UsdlikePremium>, i32) {
        let source_is_krw = source_quote == QuoteCurrency::KRW;
        let target_is_krw = target_quote == QuoteCurrency::KRW;

        // Raw premium (direct comparison of stored prices)
        let raw_premium = FixedPoint::premium_bps(source_price, target_price);

        // Determine the overseas market's quote (non-KRW side)
        let overseas_quote = if source_is_krw {
            target_quote
        } else if target_is_krw {
            source_quote
        } else {
            // Both are non-KRW: use source quote for USDlike
            source_quote
        };

        // Determine USDlike quote type
        let usdlike_quote = UsdlikeQuote::from_quote_currency(overseas_quote);

        // If neither side is KRW, USDlike premium is raw premium (same currency comparison)
        if !source_is_krw && !target_is_krw {
            let usdlike = usdlike_quote.map(|q| UsdlikePremium {
                bps: raw_premium,
                quote: q,
            });
            return (usdlike, raw_premium);
        }

        // Calculate USDlike premium based on overseas quote
        let usdlike_premium = match usdlike_quote {
            Some(UsdlikeQuote::USDT) => {
                // USDT comparison: use USDT/KRW rate
                usdt_krw_rate.map(|rate| {
                    let (krw_price, overseas_price) = if source_is_krw {
                        (source_price.to_f64(), target_price.to_f64())
                    } else {
                        (target_price.to_f64(), source_price.to_f64())
                    };

                    // Prices should already be in same denomination in new system
                    // This handles legacy case where they might not be
                    let _ = rate; // Rate already applied in price storage
                    let bps = if source_is_krw {
                        ((overseas_price - krw_price) / krw_price * 10000.0) as i32
                    } else {
                        ((krw_price - overseas_price) / overseas_price * 10000.0) as i32
                    };
                    UsdlikePremium {
                        bps,
                        quote: UsdlikeQuote::USDT,
                    }
                })
            }
            Some(UsdlikeQuote::USDC) => {
                // USDC comparison: use USDC/KRW rate
                usdc_krw_rate.map(|rate| {
                    let (krw_price, overseas_price) = if source_is_krw {
                        (source_price.to_f64(), target_price.to_f64())
                    } else {
                        (target_price.to_f64(), source_price.to_f64())
                    };

                    let _ = rate; // Rate already applied in price storage
                    let bps = if source_is_krw {
                        ((overseas_price - krw_price) / krw_price * 10000.0) as i32
                    } else {
                        ((krw_price - overseas_price) / overseas_price * 10000.0) as i32
                    };
                    UsdlikePremium {
                        bps,
                        quote: UsdlikeQuote::USDC,
                    }
                })
            }
            Some(UsdlikeQuote::BUSD) => {
                // BUSD not commonly used, fallback to raw premium
                Some(UsdlikePremium {
                    bps: raw_premium,
                    quote: UsdlikeQuote::BUSD,
                })
            }
            None => None, // Overseas market doesn't use USDlike quote (e.g., USD)
        };

        // Kimchi premium: use USD/KRW forex rate
        let kimchi_premium = match (usd_krw_rate, usdt_krw_rate) {
            (Some(usd_krw), Some(usdt_krw)) if usd_krw > 0.0 && usdt_krw > 0.0 => {
                // Ratio: USDT_KRW / USD_KRW
                let rate_ratio = usdt_krw / usd_krw;

                let (krw_price, overseas_price) = if source_is_krw {
                    (source_price.to_f64(), target_price.to_f64())
                } else {
                    (target_price.to_f64(), source_price.to_f64())
                };

                // Adjust KRW price using forex rate (krw_price is in USDT, convert to USD)
                let krw_price_usd = krw_price * rate_ratio;

                if source_is_krw {
                    ((overseas_price - krw_price_usd) / krw_price_usd * 10000.0) as i32
                } else {
                    ((krw_price_usd - overseas_price) / overseas_price * 10000.0) as i32
                }
            }
            _ => raw_premium,
        };

        (usdlike_premium, kimchi_premium)
    }

    pub fn add_step(&mut self, step: RouteStep) {
        self.route.push(step);
        self.total_hops = self.route.len() as u8;
    }

    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        now > self.expires_at_ms
    }

    pub fn is_profitable(&self) -> bool {
        self.net_profit_estimate > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === TradeSide tests ===

    #[test]
    fn test_trade_side() {
        assert_eq!(TradeSide::Buy.opposite(), TradeSide::Sell);
        assert_eq!(TradeSide::Sell.opposite(), TradeSide::Buy);
    }

    // === RouteStep tests ===

    #[test]
    fn test_route_step_trade() {
        let step = RouteStep::trade(
            Exchange::Binance,
            12345,
            TradeSide::Buy,
            FixedPoint::from_f64(50000.0),
            10, // 0.1% slippage
        );

        assert!(matches!(step, RouteStep::Trade { .. }));
    }

    #[test]
    fn test_route_step_bridge() {
        let step = RouteStep::bridge(BridgeProtocol::Stargate, Chain::Ethereum, Chain::Arbitrum);

        assert!(matches!(step, RouteStep::Bridge { .. }));
    }

    #[test]
    fn test_route_step_withdraw() {
        let step = RouteStep::withdraw(Exchange::Binance, Chain::Ethereum);
        assert!(matches!(step, RouteStep::Withdraw { .. }));
    }

    #[test]
    fn test_route_step_deposit() {
        let step = RouteStep::deposit(Exchange::Coinbase, Chain::Ethereum);
        assert!(matches!(step, RouteStep::Deposit { .. }));
    }

    // === PremiumMatrix tests ===

    #[test]
    fn test_exchange_pair_premium() {
        let premium = ExchangePairPremium::new(
            Exchange::Binance,
            Exchange::Coinbase,
            FixedPoint::from_f64(50000.0), // buy price
            FixedPoint::from_f64(50500.0), // sell price
        );

        // Premium = (50500 - 50000) / 50000 * 10000 = 100 bps (1%)
        assert_eq!(premium.premium_bps, 100);
    }

    #[test]
    fn test_exchange_pair_premium_negative() {
        let premium = ExchangePairPremium::new(
            Exchange::Binance,
            Exchange::Coinbase,
            FixedPoint::from_f64(50500.0), // buy higher
            FixedPoint::from_f64(50000.0), // sell lower
        );

        // Negative premium (loss)
        assert!(premium.premium_bps < 0);
    }

    // === ArbitrageOpportunity tests ===

    #[test]
    fn test_arbitrage_opportunity_new() {
        let asset = Asset::eth();
        let opp = ArbitrageOpportunity::new(
            1,
            Exchange::Binance,
            Exchange::Coinbase,
            asset,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(50500.0),
        );

        assert_eq!(opp.id, 1);
        assert_eq!(opp.source_exchange, Exchange::Binance);
        assert_eq!(opp.target_exchange, Exchange::Coinbase);
        assert_eq!(opp.premium_bps, 100); // 1%
    }

    #[test]
    fn test_arbitrage_opportunity_add_route() {
        let asset = Asset::eth();
        let mut opp = ArbitrageOpportunity::new(
            1,
            Exchange::Binance,
            Exchange::Coinbase,
            asset,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(50500.0),
        );

        opp.add_step(RouteStep::withdraw(Exchange::Binance, Chain::Ethereum));
        opp.add_step(RouteStep::deposit(Exchange::Coinbase, Chain::Ethereum));

        assert_eq!(opp.route.len(), 2);
        assert_eq!(opp.total_hops, 2);
    }

    #[test]
    fn test_arbitrage_opportunity_is_expired() {
        let asset = Asset::eth();
        let mut opp = ArbitrageOpportunity::new(
            1,
            Exchange::Binance,
            Exchange::Coinbase,
            asset,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(50500.0),
        );

        // Set expiry to past
        opp.expires_at_ms = 0;
        assert!(opp.is_expired());

        // Set expiry to future
        opp.expires_at_ms = u64::MAX;
        assert!(!opp.is_expired());
    }

    #[test]
    fn test_arbitrage_opportunity_is_profitable() {
        let asset = Asset::eth();
        let mut opp = ArbitrageOpportunity::new(
            1,
            Exchange::Binance,
            Exchange::Coinbase,
            asset,
            FixedPoint::from_f64(50000.0),
            FixedPoint::from_f64(50500.0),
        );

        opp.net_profit_estimate = 100; // positive
        assert!(opp.is_profitable());

        opp.net_profit_estimate = -100; // negative
        assert!(!opp.is_profitable());
    }
}
