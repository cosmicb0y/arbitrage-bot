//! Arbitrage opportunity detection and route types.

use crate::{Asset, BridgeProtocol, Chain, Exchange, FixedPoint};
use serde::{Deserialize, Serialize};

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
    pub asset: Asset,
    pub source_price: u64,
    pub target_price: u64,
    /// Premium in basis points
    pub premium_bps: i32,

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
}

impl ArbitrageOpportunity {
    pub fn new(
        id: u64,
        source_exchange: Exchange,
        target_exchange: Exchange,
        asset: Asset,
        source_price: FixedPoint,
        target_price: FixedPoint,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let premium_bps = FixedPoint::premium_bps(source_price, target_price);

        Self {
            id,
            discovered_at_ms: now,
            expires_at_ms: now + 30_000, // 30 seconds default
            source_exchange,
            target_exchange,
            asset,
            source_price: source_price.0,
            target_price: target_price.0,
            premium_bps,
            route: Vec::new(),
            total_hops: 0,
            estimated_gas_cost: 0,
            estimated_bridge_fee: 0,
            estimated_trading_fee: 0,
            net_profit_estimate: 0,
            min_amount: 0,
            max_amount: u64::MAX,
            confidence_score: 50,
        }
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
        let step = RouteStep::bridge(
            BridgeProtocol::Stargate,
            Chain::Ethereum,
            Chain::Arbitrum,
        );

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
