use serde::{Deserialize, Serialize};
use crate::market::OrderSide;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraderType {
    Retail,
    Institutional,
    HFT,
    MarketMaker,
    Whale,
    Momentum,
    Arbitrageur,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trader {
    pub id: String,
    pub trader_type: TraderType,
    pub capital: f64,
    pub activity_level: f64,
    pub avg_trade_size: f64,
    pub trade_size_variance: f64,
    pub win_rate: f64,               // How often they're "right"
    pub patience: f64,
    pub aggression: f64,
    pub risk_tolerance: f64,
    

    pub trades_today: usize,
    pub pnl: f64,
    pub position: f64,
    pub last_trade_time: u64,
}

impl Trader {
    pub fn new(id: String, trader_type: TraderType, capital: f64) -> Self {
        let (activity_level, avg_trade_size, trade_size_variance, win_rate, patience, aggression, risk_tolerance) = 
            match trader_type {
                TraderType::Retail => {
                    (0.05, capital * 0.02, 0.5, 0.45, 0.3, 0.7, 0.5)
                }
                TraderType::Institutional => {
                    (0.2, capital * 0.1, 0.3, 0.55, 0.8, 0.3, 0.3)
                }
                TraderType::HFT => {
                    (0.95, capital * 0.01, 0.2, 0.52, 0.1, 0.5, 0.2)
                }
                TraderType::MarketMaker => {
                    (0.99, capital * 0.05, 0.3, 0.51, 0.9, 0.1, 0.4)
                }
                TraderType::Whale => {
                    (0.01, capital * 0.3, 0.6, 0.60, 0.9, 0.2, 0.6)
                }
                TraderType::Momentum => {
                    (0.4, capital * 0.05, 0.4, 0.48, 0.5, 0.6, 0.7)
                }
                TraderType::Arbitrageur => {
                    (0.8, capital * 0.03, 0.2, 0.53, 0.2, 0.8, 0.3)
                }
            };
        
        Self {
            id,
            trader_type,
            capital,
            activity_level,
            avg_trade_size,
            trade_size_variance,
            win_rate,
            patience,
            aggression,
            risk_tolerance,
            trades_today: 0,
            pnl: 0.0,
            position: 0.0,
            last_trade_time: 0,
        }
    }
    

    pub fn is_active(&self, random: f64) -> bool {
        random < self.activity_level
    }
    

    pub fn generate_trade_size(&self, random: f64, price: f64) -> f64 {
        let variance_factor = 1.0 + (random - 0.5) * self.trade_size_variance;
        let size_dollars = self.avg_trade_size * variance_factor;
        let size = size_dollars / price;
        

        let position_pct = self.position.abs() / (self.capital / price);
        if position_pct > 0.5 {
            size * 0.5
        } else {
            size
        }
    }
    

    pub fn determine_side(&self, _price: f64, price_change: f64, random: f64) -> OrderSide {
        match self.trader_type {
            TraderType::Retail => {

                if random < 0.5 + price_change * 5.0 {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                }
            }
            TraderType::Institutional => {

                if random < self.win_rate {

                    if price_change > 0.01 {
                        OrderSide::Sell
                    } else if price_change < -0.01 {
                        OrderSide::Buy
                    } else if random < 0.5 {
                        OrderSide::Buy
                    } else {
                        OrderSide::Sell
                    }
                } else {
                    if random < 0.5 {
                        OrderSide::Buy
                    } else {
                        OrderSide::Sell
                    }
                }
            }
            TraderType::HFT | TraderType::Arbitrageur => {

                if self.position > 0.0 {
                    OrderSide::Sell
                } else if self.position < 0.0 {
                    OrderSide::Buy
                } else if random < 0.5 {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                }
            }
            TraderType::MarketMaker => {

                if random < 0.5 {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                }
            }
            TraderType::Whale => {

                if price_change > 0.02 {
                    OrderSide::Sell
                } else if price_change < -0.02 {
                    OrderSide::Buy
                } else if random < 0.5 {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                }
            }
            TraderType::Momentum => {

                if price_change > 0.005 {
                    OrderSide::Buy
                } else if price_change < -0.005 {
                    OrderSide::Sell
                } else if random < 0.5 {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                }
            }
        }
    }
    

    pub fn is_aggressive_order(&self, random: f64) -> bool {
        random < self.aggression
    }
    
    pub fn update_position(&mut self, side: OrderSide, quantity: f64, price: f64) {
        self.trades_today += 1;
        
        match side {
            OrderSide::Buy => {
                self.position += quantity;
                self.pnl -= quantity * price;
            }
            OrderSide::Sell => {
                self.position -= quantity;
                self.pnl += quantity * price;
            }
        }
    }
    
    pub fn reset_daily_stats(&mut self) {
        self.trades_today = 0;
    }
}
#[derive(Debug, Clone)]
pub struct TraderPopulation {
    pub traders: Vec<Trader>,
    pub total_capital: f64,
}

impl TraderPopulation {
    pub fn new() -> Self {
        let mut traders = Vec::new();
        let mut total_capital = 0.0;
        

        

        for i in 0..1000 {
            let capital = 1000.0 + (i as f64 * 5.0);
            traders.push(Trader::new(format!("retail_{}", i), TraderType::Retail, capital));
            total_capital += capital;
        }
        

        for i in 0..100 {
            let capital = 50000.0 + (i as f64 * 1000.0);
            traders.push(Trader::new(format!("inst_{}", i), TraderType::Institutional, capital));
            total_capital += capital;
        }
        

        for i in 0..200 {
            let capital = 10000.0 + (i as f64 * 200.0);
            traders.push(Trader::new(format!("hft_{}", i), TraderType::HFT, capital));
            total_capital += capital;
        }
        

        for i in 0..50 {
            let capital = 30000.0 + (i as f64 * 500.0);
            traders.push(Trader::new(format!("mm_{}", i), TraderType::MarketMaker, capital));
            total_capital += capital;
        }
        

        for i in 0..10 {
            let capital = 150000.0 + (i as f64 * 10000.0);
            traders.push(Trader::new(format!("whale_{}", i), TraderType::Whale, capital));
            total_capital += capital;
        }
        

        for i in 0..50 {
            let capital = 8000.0 + (i as f64 * 200.0);
            traders.push(Trader::new(format!("momentum_{}", i), TraderType::Momentum, capital));
            total_capital += capital;
        }
        

        for i in 0..20 {
            let capital = 15000.0 + (i as f64 * 500.0);
            traders.push(Trader::new(format!("arb_{}", i), TraderType::Arbitrageur, capital));
            total_capital += capital;
        }
        
        Self {
            traders,
            total_capital,
        }
    }
    
    pub fn get_trader_stats(&self) -> TraderStats {
        let mut stats_by_type = std::collections::HashMap::new();
        
        for trader in &self.traders {
            let entry = stats_by_type.entry(trader.trader_type).or_insert(TypeStats {
                count: 0,
                total_capital: 0.0,
                total_trades: 0,
                total_volume: 0.0,
            });
            
            entry.count += 1;
            entry.total_capital += trader.capital;
            entry.total_trades += trader.trades_today;
        }
        
        TraderStats {
            total_traders: self.traders.len(),
            total_capital: self.total_capital,
            stats_by_type,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeStats {
    pub count: usize,
    pub total_capital: f64,
    pub total_trades: usize,
    pub total_volume: f64,
}

#[derive(Debug, Clone)]
pub struct TraderStats {
    pub total_traders: usize,
    pub total_capital: f64,
    pub stats_by_type: std::collections::HashMap<TraderType, TypeStats>,
}

impl TraderStats {
    pub fn print(&self) {
        println!("\n{}", "=".repeat(70));
        println!("TRADER POPULATION STATISTICS");
        println!("{}", "=".repeat(70));
        println!("Total Traders:     {}", self.total_traders);
        println!("Total Capital:     ${:.2}", self.total_capital);
        println!();
        
        let types = vec![
            TraderType::Retail,
            TraderType::Institutional,
            TraderType::HFT,
            TraderType::MarketMaker,
            TraderType::Whale,
            TraderType::Momentum,
            TraderType::Arbitrageur,
        ];
        
        println!("{:<20} {:>10} {:>15} {:>12} {:>12}", 
            "Type", "Count", "Capital ($)", "% Pop", "% Capital");
        println!("{}", "-".repeat(70));
        
        for trader_type in types {
            if let Some(stats) = self.stats_by_type.get(&trader_type) {
                let pop_pct = (stats.count as f64 / self.total_traders as f64) * 100.0;
                let cap_pct = (stats.total_capital / self.total_capital) * 100.0;
                
                println!("{:<20} {:>10} {:>15.2} {:>11.1}% {:>11.1}%",
                    format!("{:?}", trader_type),
                    stats.count,
                    stats.total_capital,
                    pop_pct,
                    cap_pct,
                );
            }
        }
        println!("{}", "=".repeat(70));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trader_creation() {
        let trader = Trader::new("test".to_string(), TraderType::Retail, 10000.0);
        assert_eq!(trader.capital, 10000.0);
        assert_eq!(trader.trader_type, TraderType::Retail);
    }

    #[test]
    fn test_population_creation() {
        let pop = TraderPopulation::new();
        assert!(pop.traders.len() > 0);
        assert!(pop.total_capital > 0.0);
    }

    #[test]
    fn test_trader_stats() {
        let pop = TraderPopulation::new();
        let stats = pop.get_trader_stats();
        assert_eq!(stats.total_traders, pop.traders.len());
    }
}
