use crate::market::{Order, OrderBook, OrderSide, Trade, MarketBar};
use crate::rps_mining::Move;
use crate::traders::{TraderPopulation, Trader, TraderType};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarketRegime {
    Trending,
    MeanReverting,
    HighVolatility,
    LowVolatility,
    Crisis,
}
#[derive(Debug, Clone)]
pub struct MarketMicrostructure {
    pub base_spread_bps: f64,
    pub depth_at_touch: f64,
    pub depth_falloff: f64,
    pub tick_size: f64,
    pub lot_size: f64,
}

impl Default for MarketMicrostructure {
    fn default() -> Self {
        Self {
            base_spread_bps: 5.0,
            depth_at_touch: 10.0,
            depth_falloff: 0.8,
            tick_size: 0.01,
            lot_size: 0.01,
        }
    }
}
pub struct MarketSimulator {
    pub orderbooks: HashMap<String, OrderBook>,
    pub order_id_counter: u64,
    pub current_time: u64,
    pub symbols: Vec<String>,
    pub seed: u64,
    

    pub regime: MarketRegime,
    pub trend: f64,
    pub volatility: f64,
    pub volume_profile: Vec<f64>,
    pub microstructure: MarketMicrostructure,
    

    pub last_prices: HashMap<String, f64>,
    pub price_history: HashMap<String, Vec<f64>>,
    pub volume_history: HashMap<String, Vec<f64>>,
    pub volatility_cluster: f64,
    pub momentum: f64,
    

    pub traders: TraderPopulation,
}

impl MarketSimulator {
    pub fn new(symbols: Vec<String>) -> Self {
        let mut orderbooks = HashMap::new();
        let mut last_prices = HashMap::new();
        let mut price_history = HashMap::new();
        let mut volume_history = HashMap::new();
        
        for symbol in &symbols {
            orderbooks.insert(symbol.clone(), OrderBook::new(symbol.clone()));
            last_prices.insert(symbol.clone(), 0.0);
            price_history.insert(symbol.clone(), Vec::new());
            volume_history.insert(symbol.clone(), Vec::new());
        }

        let seed = 42;
        

        let mut volume_profile = Vec::new();
        for hour in 0..24 {
            let normalized = hour as f64 / 24.0;

            let volume = 1.0 + 0.5 * (normalized * std::f64::consts::PI * 2.0).cos();
            volume_profile.push(volume);
        }

        let traders = TraderPopulation::new();

        Self {
            orderbooks,
            order_id_counter: 0,
            current_time: 0,
            symbols,
            seed,
            regime: MarketRegime::Trending,
            trend: 0.0,
            volatility: 0.02,
            volume_profile,
            microstructure: MarketMicrostructure::default(),
            last_prices,
            price_history,
            volume_history,
            volatility_cluster: 1.0,
            momentum: 0.0,
            traders,
        }
    }
    
    pub fn set_initial_price(&mut self, symbol: &str, price: f64) {
        self.last_prices.insert(symbol.to_string(), price);
    }
    
    pub fn print_trader_stats(&self) {
        self.traders.get_trader_stats().print();
    }

    pub fn next_order_id(&mut self) -> u64 {
        self.order_id_counter += 1;
        self.order_id_counter
    }
    fn generate_random(&mut self, range: u64) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(self.seed.to_be_bytes());
        hasher.update(self.current_time.to_be_bytes());
        hasher.update(self.order_id_counter.to_be_bytes());
        let hash = hasher.finalize();
        
        let value = u64::from_be_bytes([
            hash[0], hash[1], hash[2], hash[3],
            hash[4], hash[5], hash[6], hash[7],
        ]);
        
        self.seed = value;
        value % range
    }

    fn generate_random_move(&mut self) -> Move {
        Move::from_seed(self.generate_random(3))
    }
    fn update_regime(&mut self, symbol: &str) {
        if let Some(history) = self.price_history.get(symbol) {
            if history.len() < 20 {
                return;
            }
            
            let recent = &history[history.len().saturating_sub(20)..];
            

            let returns: Vec<f64> = recent.windows(2)
                .map(|w| ((w[1] / w[0]) - 1.0).abs())
                .collect();
            let avg_vol = returns.iter().sum::<f64>() / returns.len() as f64;
            

            let trend_strength = (recent.last().unwrap() - recent.first().unwrap()) / recent.first().unwrap();
            

            self.regime = if avg_vol > 0.03 {
                MarketRegime::HighVolatility
            } else if avg_vol > 0.02 {
                if trend_strength.abs() > 0.02 {
                    MarketRegime::Trending
                } else {
                    MarketRegime::MeanReverting
                }
            } else {
                MarketRegime::LowVolatility
            };
            

            self.volatility_cluster = 0.9 * self.volatility_cluster + 0.1 * avg_vol * 50.0;
        }
    }
    pub fn generate_price_movement(&mut self, symbol: &str, base_price: f64) -> f64 {
        let current_price = self.last_prices.get(symbol).copied().unwrap_or(base_price);
        

        let player_move1 = self.generate_random_move();
        let blockchain_move1 = self.generate_random_move();
        let player_move2 = self.generate_random_move();
        let blockchain_move2 = self.generate_random_move();

        use crate::rps_mining::GameResult;
        let result1 = player_move1.beats(&blockchain_move1);
        let result2 = player_move2.beats(&blockchain_move2);

        let mut net_score = 0;
        if result1 == GameResult::PlayerWin { net_score += 1; }
        if result1 == GameResult::BlockchainWin { net_score -= 1; }
        if result2 == GameResult::PlayerWin { net_score += 1; }
        if result2 == GameResult::BlockchainWin { net_score -= 1; }
        let regime_multiplier = match self.regime {
            MarketRegime::Trending => {
                self.trend = 0.95 * self.trend + 0.05 * net_score as f64;
                1.0 + self.trend * 0.5
            }
            MarketRegime::MeanReverting => {

                if let Some(history) = self.price_history.get(symbol) {
                    if history.len() > 20 {
                        let mean = history.iter().rev().take(20).sum::<f64>() / 20.0;
                        let deviation = (current_price - mean) / mean;
                        net_score = (net_score as f64 - deviation * 10.0) as i32;
                    }
                }
                0.7
            }
            MarketRegime::HighVolatility => 2.5,
            MarketRegime::LowVolatility => 0.3,
            MarketRegime::Crisis => 5.0,
        };
        let vol_multiplier = self.volatility_cluster;
        self.momentum = 0.9 * self.momentum + 0.1 * net_score as f64;
        let momentum_drift = self.momentum * 0.0001;
        let base_vol = 0.0003;
        let price_change = (net_score as f64 * base_vol * current_price * regime_multiplier * vol_multiplier)
            + (momentum_drift * current_price);
        let noise = ((self.generate_random(100) as f64 - 50.0) / 500.0) * current_price * 0.0001;

        let new_price = current_price + price_change + noise;
        

        if let Some(history) = self.price_history.get_mut(symbol) {
            history.push(new_price);
            if history.len() > 200 {
                history.remove(0);
            }
        }
        
        self.last_prices.insert(symbol.to_string(), new_price);
        self.update_regime(symbol);
        
        new_price.max(current_price * 0.9).min(current_price * 1.1)
    }
    pub fn add_market_maker_orders(&mut self, symbol: &str, center_price: f64) {
        let spread_bps = self.microstructure.base_spread_bps * 
            match self.regime {
                MarketRegime::HighVolatility | MarketRegime::Crisis => 2.0,
                MarketRegime::LowVolatility => 0.5,
                _ => 1.0,
            };
        
        let half_spread = center_price * (spread_bps / 10000.0) / 2.0;
        let tick = self.microstructure.tick_size;
        

        let num_levels = 10;
        let mut depth = self.microstructure.depth_at_touch;
        
        for i in 0..num_levels {

            let ticks_away = (i + 1) as f64;
            let buy_price = center_price - half_spread - ticks_away * tick * 5.0;
            let sell_price = center_price + half_spread + ticks_away * tick * 5.0;
            

            let buy_price = (buy_price / tick).floor() * tick;
            let sell_price = (sell_price / tick).ceil() * tick;
            

            let depth_noise = 1.0 + ((self.generate_random(40) as f64 - 20.0) / 100.0);
            let quantity = depth * depth_noise;

            let buy_order = Order::new(
                self.next_order_id(),
                format!("mm_{}", self.generate_random(100)),
                symbol.to_string(),
                OrderSide::Buy,
                buy_price,
                quantity,
            );

            let sell_order = Order::new(
                self.next_order_id(),
                format!("mm_{}", self.generate_random(100)),
                symbol.to_string(),
                OrderSide::Sell,
                sell_price,
                quantity,
            );

            if let Some(book) = self.orderbooks.get_mut(symbol) {
                book.add_order(buy_order);
                book.add_order(sell_order);
            }
            

            depth *= self.microstructure.depth_falloff;
        }
    }
    pub fn generate_trader_orders(&mut self, symbol: &str, base_price: f64) -> Vec<Trade> {
        let mut all_trades = Vec::new();
        
        let current_price = self.last_prices.get(symbol).copied().unwrap_or(base_price);
        

        let price_change = if let Some(history) = self.price_history.get(symbol) {
            if history.len() >= 2 {
                (history[history.len()-1] - history[history.len()-2]) / history[history.len()-2]
            } else {
                0.0
            }
        } else {
            0.0
        };
        

        for trader_idx in 0..self.traders.traders.len() {
            let random1 = self.generate_random(10000) as f64 / 10000.0;
            let random2 = self.generate_random(10000) as f64 / 10000.0;
            let random3 = self.generate_random(10000) as f64 / 10000.0;
            

            let trader = self.traders.traders[trader_idx].clone();
            
            if !trader.is_active(random1) {
                continue;
            }
            
            let side = trader.determine_side(current_price, price_change, random2);
            let quantity = trader.generate_trade_size(random3, current_price);
            
            if quantity < 0.0001 {
                continue;
            }
            

            let is_aggressive = trader.is_aggressive_order(random1);
            
            let order_price = if is_aggressive {

                if side == OrderSide::Buy {
                    current_price * 1.001
                } else {
                    current_price * 0.999
                }
            } else {

                let spread_bps = self.microstructure.base_spread_bps;
                let half_spread = current_price * (spread_bps / 20000.0);
                
                if side == OrderSide::Buy {
                    current_price - half_spread - (random2 * half_spread * 0.5)
                } else {
                    current_price + half_spread + (random2 * half_spread * 0.5)
                }
            };
            
            let order = Order::new(
                self.next_order_id(),
                trader.id.clone(),
                symbol.to_string(),
                side,
                order_price,
                quantity,
            );
            
            if let Some(book) = self.orderbooks.get_mut(symbol) {
                let trades = book.add_order(order);
                

                for trade in &trades {
                    if trade.buyer == trader.id {
                        self.traders.traders[trader_idx].update_position(OrderSide::Buy, trade.quantity, trade.price);
                    } else if trade.seller == trader.id {
                        self.traders.traders[trader_idx].update_position(OrderSide::Sell, trade.quantity, trade.price);
                    }
                }
                
                all_trades.extend(trades);
            }
        }
        
        all_trades
    }
    pub fn step(&mut self, symbol: &str, base_price: f64) -> Vec<Trade> {
        self.current_time += 1;
        let target_price = self.generate_price_movement(symbol, base_price);
        self.add_market_maker_orders(symbol, target_price);
        let all_trades = self.generate_trader_orders(symbol, target_price);

        all_trades
    }
    pub fn simulate_session(&mut self, symbol: &str, initial_price: f64, num_bars: usize) -> Vec<MarketBar> {
        let mut bars = Vec::new();
        let mut current_base_price = initial_price;

        println!("\nSimulating {} bars with {} traders...", num_bars, self.traders.traders.len());

        for bar_idx in 0..num_bars {
            let bar_start_time = self.current_time;
            let trades = self.step(symbol, current_base_price);

            if let Some(bar) = MarketBar::from_trades(bar_start_time, symbol.to_string(), &trades) {
                current_base_price = bar.close;
                bars.push(bar);
            } else {

                let bar = MarketBar::new(
                    bar_start_time,
                    symbol.to_string(),
                    current_base_price,
                    current_base_price,
                    current_base_price,
                    current_base_price,
                    0.0,
                );
                bars.push(bar);
            }
            if bar_idx % 10 == 0 {

                let trend_direction = if self.generate_random(2) == 0 { 1.0 } else { -1.0 };
                current_base_price *= 1.0 + (trend_direction * 0.005);
            }
            

            if (bar_idx + 1) % 100 == 0 {
                println!("   Progress: {}/{} bars ({:.1}%)", bar_idx + 1, num_bars, ((bar_idx + 1) as f64 / num_bars as f64) * 100.0);
            }
        }

        bars
    }

    pub fn get_orderbook(&self, symbol: &str) -> Option<&OrderBook> {
        self.orderbooks.get(symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_simulator_creation() {
        let symbols = vec!["BTC/USD".to_string()];
        let simulator = MarketSimulator::new(symbols);
        assert_eq!(simulator.symbols.len(), 1);
        assert!(simulator.orderbooks.contains_key("BTC/USD"));
    }

    #[test]
    fn test_price_generation() {
        let symbols = vec!["BTC/USD".to_string()];
        let mut simulator = MarketSimulator::new(symbols);
        let price = simulator.generate_price_movement("BTC/USD", 50000.0);
        assert!(price > 49000.0 && price < 51000.0);
    }

    #[test]
    fn test_market_simulation() {
        let symbols = vec!["BTC/USD".to_string()];
        let mut simulator = MarketSimulator::new(symbols);
        let bars = simulator.simulate_session("BTC/USD", 50000.0, 100);
        assert_eq!(bars.len(), 100);
    }
}
