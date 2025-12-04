use crate::market::{MarketBar, OrderSide};
use crate::strategy::{Position, Signal, Strategy, StrategyContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    pub timestamp: u64,
    pub symbol: String,
    pub side: OrderSide,
    pub price: f64,
    pub quantity: f64,
    pub commission: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub timestamp: u64,
    pub cash: f64,
    pub positions_value: f64,
    pub total_value: f64,
    pub positions: Vec<Position>,
}
pub struct Backtester {
    pub initial_capital: f64,
    pub cash: f64,
    pub positions: HashMap<String, Position>,
    pub trade_history: Vec<TradeExecution>,
    pub portfolio_history: Vec<PortfolioSnapshot>,
    pub commission_rate: f64,
}

impl Backtester {
    pub fn new(initial_capital: f64, commission_rate: f64) -> Self {
        Self {
            initial_capital,
            cash: initial_capital,
            positions: HashMap::new(),
            trade_history: Vec::new(),
            portfolio_history: Vec::new(),
            commission_rate,
        }
    }

    pub fn execute_trade(&mut self, timestamp: u64, symbol: String, side: OrderSide, price: f64, quantity: f64) -> Result<(), String> {
        let trade_value = price * quantity;
        let commission = trade_value * self.commission_rate;

        match side {
            OrderSide::Buy => {
                let total_cost = trade_value + commission;
                if self.cash < total_cost {
                    return Err("Insufficient cash for trade".to_string());
                }

                self.cash -= total_cost;
                self.positions.entry(symbol.clone())
                    .and_modify(|pos| {
                        let total_quantity = pos.quantity + quantity;
                        let total_cost = (pos.entry_price * pos.quantity) + (price * quantity);
                        pos.entry_price = total_cost / total_quantity;
                        pos.quantity = total_quantity;
                    })
                    .or_insert(Position {
                        symbol: symbol.clone(),
                        quantity,
                        entry_price: price,
                        side: OrderSide::Buy,
                    });

                self.trade_history.push(TradeExecution {
                    timestamp,
                    symbol,
                    side: OrderSide::Buy,
                    price,
                    quantity,
                    commission,
                });

                Ok(())
            }
            OrderSide::Sell => {

                if let Some(position) = self.positions.get_mut(&symbol) {
                    if position.quantity < quantity {
                        return Err("Insufficient position for trade".to_string());
                    }

                    let proceeds = trade_value - commission;
                    self.cash += proceeds;

                    position.quantity -= quantity;
                    if position.quantity <= 0.0001 {
                        self.positions.remove(&symbol);
                    }

                    self.trade_history.push(TradeExecution {
                        timestamp,
                        symbol,
                        side: OrderSide::Sell,
                        price,
                        quantity,
                        commission,
                    });

                    Ok(())
                } else {
                    Err("No position to sell".to_string())
                }
            }
        }
    }

    pub fn calculate_portfolio_value(&self, prices: &HashMap<String, f64>) -> f64 {
        let positions_value: f64 = self.positions.iter()
            .map(|(symbol, position)| {
                let price = prices.get(symbol).unwrap_or(&position.entry_price);
                price * position.quantity
            })
            .sum();

        self.cash + positions_value
    }

    pub fn record_snapshot(&mut self, timestamp: u64, prices: &HashMap<String, f64>) {
        let positions_value: f64 = self.positions.iter()
            .map(|(symbol, position)| {
                let price = prices.get(symbol).unwrap_or(&position.entry_price);
                price * position.quantity
            })
            .sum();

        let total_value = self.cash + positions_value;

        self.portfolio_history.push(PortfolioSnapshot {
            timestamp,
            cash: self.cash,
            positions_value,
            total_value,
            positions: self.positions.values().cloned().collect(),
        });
    }

    pub fn run_backtest<S: Strategy>(
        &mut self,
        strategy: &mut S,
        bars: &[MarketBar],
        position_size: f64,
    ) -> BacktestResult {
        strategy.reset();
        
        let symbol = bars[0].symbol.clone();
        
        for (idx, bar) in bars.iter().enumerate() {
            let position = self.positions.get(&symbol);
            
            let context = StrategyContext {
                bars,
                current_index: idx,
                position: position,
            };

            let signal = strategy.generate_signal(&context);

            match signal {
                Signal::Buy => {
                    if position.is_none() {
                        let quantity = (self.cash * position_size) / bar.close;
                        if quantity > 0.0 {
                            let _ = self.execute_trade(
                                bar.timestamp,
                                symbol.clone(),
                                OrderSide::Buy,
                                bar.close,
                                quantity,
                            );
                        }
                    }
                }
                Signal::Sell => {
                    if let Some(pos) = position {
                        let _ = self.execute_trade(
                            bar.timestamp,
                            symbol.clone(),
                            OrderSide::Sell,
                            bar.close,
                            pos.quantity,
                        );
                    }
                }
                Signal::Hold => {}
            }

            let mut prices = HashMap::new();
            prices.insert(symbol.clone(), bar.close);
            self.record_snapshot(bar.timestamp, &prices);
        }
        if let Some(last_bar) = bars.last() {
            let symbol = last_bar.symbol.clone();
            if let Some(position) = self.positions.get(&symbol).cloned() {
                let _ = self.execute_trade(
                    last_bar.timestamp,
                    symbol,
                    OrderSide::Sell,
                    last_bar.close,
                    position.quantity,
                );
            }
        }

        self.calculate_backtest_result(strategy.name())
    }

    fn calculate_backtest_result(&self, strategy_name: &str) -> BacktestResult {
        let final_value = self.portfolio_history.last()
            .map(|s| s.total_value)
            .unwrap_or(self.initial_capital);

        let total_return = (final_value - self.initial_capital) / self.initial_capital;
        

        let returns: Vec<f64> = self.portfolio_history.windows(2)
            .map(|w| {
                if w[0].total_value == 0.0 {
                    0.0
                } else {
                    (w[1].total_value - w[0].total_value) / w[0].total_value
                }
            })
            .collect();
        let sharpe_ratio = calculate_sharpe_ratio(&returns);
        let max_drawdown = calculate_max_drawdown(&self.portfolio_history);
        let win_rate = calculate_win_rate(&self.trade_history);
        let total_trades = self.trade_history.len() / 2;
        
        let total_commission: f64 = self.trade_history.iter().map(|t| t.commission).sum();
        
        BacktestResult {
            strategy_name: strategy_name.to_string(),
            initial_capital: self.initial_capital,
            final_value,
            total_return,
            sharpe_ratio,
            max_drawdown,
            win_rate,
            total_trades,
            total_commission,
            portfolio_history: self.portfolio_history.clone(),
            trade_history: self.trade_history.clone(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub strategy_name: String,
    pub initial_capital: f64,
    pub final_value: f64,
    pub total_return: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub win_rate: f64,
    pub total_trades: usize,
    pub total_commission: f64,
    pub portfolio_history: Vec<PortfolioSnapshot>,
    pub trade_history: Vec<TradeExecution>,
}

impl BacktestResult {
    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(60));
        println!("Backtest Results: {}", self.strategy_name);
        println!("{}", "=".repeat(60));
        println!("Initial Capital:    ${:.2}", self.initial_capital);
        println!("Final Value:        ${:.2}", self.final_value);
        println!("Total Return:       {:.2}%", self.total_return * 100.0);
        println!("Sharpe Ratio:       {:.4}", self.sharpe_ratio);
        println!("Max Drawdown:       {:.2}%", self.max_drawdown * 100.0);
        println!("Win Rate:           {:.2}%", self.win_rate * 100.0);
        println!("Total Trades:       {}", self.total_trades);
        println!("Total Commission:   ${:.2}", self.total_commission);
        println!("{}", "=".repeat(60));
    }

    pub fn get_equity_curve(&self) -> Vec<(u64, f64)> {
        self.portfolio_history.iter()
            .map(|snapshot| (snapshot.timestamp, snapshot.total_value))
            .collect()
    }
}

fn calculate_sharpe_ratio(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }

    let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
    
    let variance: f64 = returns.iter()
        .map(|r| (r - mean_return).powi(2))
        .sum::<f64>() / returns.len() as f64;
    
    let std_dev = variance.sqrt();
    
    if std_dev == 0.0 {
        return 0.0;
    }
    let annualized_return = mean_return * 252.0;
    let annualized_std = std_dev * (252.0_f64).sqrt();
    
    annualized_return / annualized_std
}

fn calculate_max_drawdown(portfolio_history: &[PortfolioSnapshot]) -> f64 {
    if portfolio_history.is_empty() {
        return 0.0;
    }

    let mut max_value = portfolio_history[0].total_value;
    let mut max_drawdown = 0.0;

    for snapshot in portfolio_history {
        if snapshot.total_value > max_value {
            max_value = snapshot.total_value;
        }

        let drawdown = (max_value - snapshot.total_value) / max_value;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    max_drawdown
}

fn calculate_win_rate(trade_history: &[TradeExecution]) -> f64 {
    if trade_history.len() < 2 {
        return 0.0;
    }

    let mut wins = 0;
    let mut total_trades = 0;
    let mut i = 0;

    while i < trade_history.len() - 1 {

        if trade_history[i].side == OrderSide::Buy {

            if let Some(sell_idx) = trade_history[i+1..].iter()
                .position(|t| t.side == OrderSide::Sell && t.symbol == trade_history[i].symbol) {
                
                let buy_trade = &trade_history[i];
                let sell_trade = &trade_history[i + 1 + sell_idx];
                
                let profit = (sell_trade.price - buy_trade.price) * buy_trade.quantity
                    - buy_trade.commission - sell_trade.commission;
                
                if profit > 0.0 {
                    wins += 1;
                }
                total_trades += 1;
                
                i = i + 1 + sell_idx + 1;
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }

    if total_trades == 0 {
        return 0.0;
    }

    wins as f64 / total_trades as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backtester_creation() {
        let backtester = Backtester::new(10000.0, 0.001);
        assert_eq!(backtester.initial_capital, 10000.0);
        assert_eq!(backtester.cash, 10000.0);
    }

    #[test]
    fn test_trade_execution() {
        let mut backtester = Backtester::new(10000.0, 0.001);
        let result = backtester.execute_trade(
            0,
            "BTC/USD".to_string(),
            OrderSide::Buy,
            50000.0,
            0.1,
        );
        assert!(result.is_ok());
        assert!(backtester.cash < 10000.0);
    }

    #[test]
    fn test_sharpe_ratio() {
        let returns = vec![0.01, 0.02, -0.01, 0.015, 0.005];
        let sharpe = calculate_sharpe_ratio(&returns);
        assert!(sharpe > 0.0);
    }
}
