use crate::merkle::Hash;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: u64,
    pub timestamp: u64,
    pub trader: String,
    pub symbol: String,
    pub side: OrderSide,
    pub price: f64,
    pub quantity: f64,
    pub filled: f64,
    pub hash: Hash,
}

impl Order {
    pub fn new(
        id: u64,
        trader: String,
        symbol: String,
        side: OrderSide,
        price: f64,
        quantity: f64,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let mut order = Self {
            id,
            timestamp,
            trader,
            symbol,
            side,
            price,
            quantity,
            filled: 0.0,
            hash: Hash::from_string(""),
        };

        order.hash = order.calculate_hash();
        order
    }

    pub fn calculate_hash(&self) -> Hash {
        let side_str = match self.side {
            OrderSide::Buy => "buy",
            OrderSide::Sell => "sell",
        };
        let data = format!(
            "{}{}{}{}{}{}{}",
            self.id, self.timestamp, self.trader, self.symbol, side_str, self.price, self.quantity
        );
        Hash::from_string(&data)
    }

    pub fn remaining(&self) -> f64 {
        self.quantity - self.filled
    }

    pub fn is_filled(&self) -> bool {
        self.filled >= self.quantity
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: u64,
    pub timestamp: u64,
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub buyer: String,
    pub seller: String,
    pub buy_order_id: u64,
    pub sell_order_id: u64,
    pub hash: Hash,
}

impl Trade {
    pub fn new(
        id: u64,
        symbol: String,
        price: f64,
        quantity: f64,
        buyer: String,
        seller: String,
        buy_order_id: u64,
        sell_order_id: u64,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let mut trade = Self {
            id,
            timestamp,
            symbol,
            price,
            quantity,
            buyer,
            seller,
            buy_order_id,
            sell_order_id,
            hash: Hash::from_string(""),
        };

        trade.hash = trade.calculate_hash();
        trade
    }

    pub fn calculate_hash(&self) -> Hash {
        let data = format!(
            "{}{}{}{}{}{}{}{}{}",
            self.id,
            self.timestamp,
            self.symbol,
            self.price,
            self.quantity,
            self.buyer,
            self.seller,
            self.buy_order_id,
            self.sell_order_id
        );
        Hash::from_string(&data)
    }
}
#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub orders: VecDeque<Order>,
    pub total_quantity: f64,
}

impl PriceLevel {
    pub fn new(price: f64) -> Self {
        Self {
            price,
            orders: VecDeque::new(),
            total_quantity: 0.0,
        }
    }

    pub fn add_order(&mut self, order: Order) {
        self.total_quantity += order.remaining();
        self.orders.push_back(order);
    }

    pub fn remove_filled_orders(&mut self) {
        self.orders.retain(|o| !o.is_filled());
        self.total_quantity = self.orders.iter().map(|o| o.remaining()).sum();
    }
}
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: BTreeMap<i64, PriceLevel>,
    pub asks: BTreeMap<i64, PriceLevel>,
    pub last_price: Option<f64>,
    pub trades: Vec<Trade>,
}

impl OrderBook {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_price: None,
            trades: Vec::new(),
        }
    }

    fn price_to_key(price: f64) -> i64 {
        (price * 10000.0) as i64
    }

    pub fn add_order(&mut self, mut order: Order) -> Vec<Trade> {
        let mut new_trades = Vec::new();

        match order.side {
            OrderSide::Buy => {

                while order.remaining() > 0.0 {
                    let best_ask = self.asks.iter_mut().next();
                    if let Some((_, level)) = best_ask {
                        if level.price <= order.price && !level.orders.is_empty() {
                            let ask_order = level.orders.front_mut().unwrap();
                            let trade_qty = order.remaining().min(ask_order.remaining());

                            let trade = Trade::new(
                                self.trades.len() as u64,
                                self.symbol.clone(),
                                level.price,
                                trade_qty,
                                order.trader.clone(),
                                ask_order.trader.clone(),
                                order.id,
                                ask_order.id,
                            );

                            order.filled += trade_qty;
                            ask_order.filled += trade_qty;

                            self.last_price = Some(level.price);
                            new_trades.push(trade.clone());
                            self.trades.push(trade);

                            level.remove_filled_orders();

                            if level.orders.is_empty() {
                                let price_key = Self::price_to_key(level.price);
                                self.asks.remove(&price_key);
                                continue;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if order.remaining() > 0.0 {
                    let price_key = -Self::price_to_key(order.price);
                    self.bids
                        .entry(price_key)
                        .or_insert_with(|| PriceLevel::new(order.price))
                        .add_order(order);
                }
            }
            OrderSide::Sell => {

                while order.remaining() > 0.0 {
                    let best_bid = self.bids.iter_mut().next();
                    if let Some((_, level)) = best_bid {
                        if level.price >= order.price && !level.orders.is_empty() {
                            let bid_order = level.orders.front_mut().unwrap();
                            let trade_qty = order.remaining().min(bid_order.remaining());

                            let trade = Trade::new(
                                self.trades.len() as u64,
                                self.symbol.clone(),
                                level.price,
                                trade_qty,
                                bid_order.trader.clone(),
                                order.trader.clone(),
                                bid_order.id,
                                order.id,
                            );

                            order.filled += trade_qty;
                            bid_order.filled += trade_qty;

                            self.last_price = Some(level.price);
                            new_trades.push(trade.clone());
                            self.trades.push(trade);

                            level.remove_filled_orders();

                            if level.orders.is_empty() {
                                let price_key = -Self::price_to_key(level.price);
                                self.bids.remove(&price_key);
                                continue;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if order.remaining() > 0.0 {
                    let price_key = Self::price_to_key(order.price);
                    self.asks
                        .entry(price_key)
                        .or_insert_with(|| PriceLevel::new(order.price))
                        .add_order(order);
                }
            }
        }

        new_trades
    }

    pub fn get_best_bid(&self) -> Option<(f64, f64)> {
        self.bids.values().next().map(|level| (level.price, level.total_quantity))
    }

    pub fn get_best_ask(&self) -> Option<(f64, f64)> {
        self.asks.values().next().map(|level| (level.price, level.total_quantity))
    }

    pub fn get_mid_price(&self) -> Option<f64> {
        match (self.get_best_bid(), self.get_best_ask()) {
            (Some((bid, _)), Some((ask, _))) => Some((bid + ask) / 2.0),
            _ => self.last_price,
        }
    }

    pub fn get_spread(&self) -> Option<f64> {
        match (self.get_best_bid(), self.get_best_ask()) {
            (Some((bid, _)), Some((ask, _))) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn get_bid_depth(&self, levels: usize) -> Vec<(f64, f64)> {
        self.bids
            .values()
            .take(levels)
            .map(|level| (level.price, level.total_quantity))
            .collect()
    }

    pub fn get_ask_depth(&self, levels: usize) -> Vec<(f64, f64)> {
        self.asks
            .values()
            .take(levels)
            .map(|level| (level.price, level.total_quantity))
            .collect()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketBar {
    pub timestamp: u64,
    pub symbol: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub hash: Hash,
}

impl MarketBar {
    pub fn new(
        timestamp: u64,
        symbol: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Self {
        let mut bar = Self {
            timestamp,
            symbol,
            open,
            high,
            low,
            close,
            volume,
            hash: Hash::from_string(""),
        };

        bar.hash = bar.calculate_hash();
        bar
    }

    pub fn calculate_hash(&self) -> Hash {
        let data = format!(
            "{}{}{}{}{}{}{}",
            self.timestamp, self.symbol, self.open, self.high, self.low, self.close, self.volume
        );
        Hash::from_string(&data)
    }

    pub fn from_trades(timestamp: u64, symbol: String, trades: &[Trade]) -> Option<Self> {
        if trades.is_empty() {
            return None;
        }

        let open = trades.first().unwrap().price;
        let close = trades.last().unwrap().price;
        let high = trades.iter().map(|t| t.price).fold(f64::NEG_INFINITY, f64::max);
        let low = trades.iter().map(|t| t.price).fold(f64::INFINITY, f64::min);
        let volume = trades.iter().map(|t| t.quantity).sum();

        Some(Self::new(timestamp, symbol, open, high, low, close, volume))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_creation() {
        let order = Order::new(1, "alice".to_string(), "BTC/USD".to_string(), OrderSide::Buy, 50000.0, 1.0);
        assert_eq!(order.id, 1);
        assert_eq!(order.remaining(), 1.0);
        assert!(!order.is_filled());
    }

    #[test]
    fn test_order_matching() {
        let mut book = OrderBook::new("BTC/USD".to_string());
        
        let buy_order = Order::new(1, "alice".to_string(), "BTC/USD".to_string(), OrderSide::Buy, 50000.0, 1.0);
        book.add_order(buy_order);
        
        let sell_order = Order::new(2, "bob".to_string(), "BTC/USD".to_string(), OrderSide::Sell, 50000.0, 1.0);
        let trades = book.add_order(sell_order);
        
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, 50000.0);
        assert_eq!(trades[0].quantity, 1.0);
    }

    #[test]
    fn test_orderbook_depth() {
        let mut book = OrderBook::new("BTC/USD".to_string());
        
        book.add_order(Order::new(1, "alice".to_string(), "BTC/USD".to_string(), OrderSide::Buy, 49900.0, 1.0));
        book.add_order(Order::new(2, "alice".to_string(), "BTC/USD".to_string(), OrderSide::Buy, 49800.0, 2.0));
        book.add_order(Order::new(3, "bob".to_string(), "BTC/USD".to_string(), OrderSide::Sell, 50100.0, 1.0));
        book.add_order(Order::new(4, "bob".to_string(), "BTC/USD".to_string(), OrderSide::Sell, 50200.0, 2.0));
        
        let bid_depth = book.get_bid_depth(2);
        let ask_depth = book.get_ask_depth(2);
        
        assert_eq!(bid_depth.len(), 2);
        assert_eq!(ask_depth.len(), 2);
        assert_eq!(bid_depth[0].0, 49900.0);
        assert_eq!(ask_depth[0].0, 50100.0);
    }
}
