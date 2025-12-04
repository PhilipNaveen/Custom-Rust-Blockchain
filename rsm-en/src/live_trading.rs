
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

use crate::orderbook_market::OrderBookMarket;
use crate::market::OrderSide;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSession {
    pub session_id: String,
    pub initial_capital: f64,
    pub current_capital: f64,
    pub position: f64,
    pub position_entry_price: Option<f64>,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub trade_history: Vec<TradeRecord>,
    pub equity_curve: Vec<f64>,
    pub drawdown_history: Vec<f64>,
    pub is_active: bool,
    pub strategy_params: StrategyParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyParams {
    pub max_inventory: f64,
    pub entry_threshold: f64,
    pub process_noise: f64,
    pub measurement_noise: f64,
    pub lookback: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub bar: usize,
    pub side: OrderSide,
    pub quantity: f64,
    pub price: f64,
    pub pnl: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketUpdate {
    pub bar_number: usize,
    pub current_price: f64,
    pub ekf_estimate: f64,
    pub position: f64,
    pub best_bid: f64,
    pub best_ask: f64,
    pub mid_price: f64,
    pub latency: u128,
    pub stats: TradingStats,
    pub equity_history: Vec<f64>,
    pub price_history: Vec<f64>,
    pub ekf_history: Vec<f64>,
    pub drawdown_history: Vec<f64>,
    pub new_trade: Option<TradeRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingStats {
    pub pnl: f64,
    pub current_value: f64,
    pub return_pct: f64,
    pub total_trades: usize,
    pub win_rate: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
}

impl TradingSession {
    pub fn new(session_id: String, initial_capital: f64, params: StrategyParams) -> Self {
        Self {
            session_id,
            initial_capital,
            current_capital: initial_capital,
            position: 0.0,
            position_entry_price: None,
            total_trades: 0,
            winning_trades: 0,
            trade_history: Vec::new(),
            equity_curve: vec![initial_capital],
            drawdown_history: vec![0.0],
            is_active: false,
            strategy_params: params,
        }
    }

    pub fn calculate_stats(&self) -> TradingStats {
        let pnl = self.current_capital - self.initial_capital;
        let return_pct = (pnl / self.initial_capital) * 100.0;
        
        let win_rate = if self.total_trades > 0 {
            (self.winning_trades as f64 / self.total_trades as f64) * 100.0
        } else {
            0.0
        };
        let returns: Vec<f64> = self.equity_curve
            .windows(2)
            .map(|w| (w[1] - w[0]) / w[0])
            .collect();
        
        let sharpe_ratio = if returns.len() > 1 {
            let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
            let std_dev = (returns.iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>() / returns.len() as f64)
                .sqrt();
            
            if std_dev > 0.0 {
                mean_return / std_dev * (252.0_f64).sqrt()
            } else {
                0.0
            }
        } else {
            0.0
        };
        let max_drawdown = self.drawdown_history.iter()
            .map(|d| d.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        TradingStats {
            pnl,
            current_value: self.current_capital,
            return_pct,
            total_trades: self.total_trades,
            win_rate,
            sharpe_ratio,
            max_drawdown,
        }
    }

    pub fn execute_trade(&mut self, side: OrderSide, quantity: f64, price: f64, bar: usize) -> TradeRecord {
        let commission = 0.001;
        let slippage = 0.0005;
        
        let mut pnl = 0.0;
        
        match side {
            OrderSide::Buy => {
                let cost = quantity * price * (1.0 + commission + slippage);
                self.current_capital -= cost;
                
                if self.position < 0.0 {

                    let entry = self.position_entry_price.unwrap_or(price);
                    pnl = self.position.abs() * (entry - price);
                }
                
                self.position += quantity;
                self.position_entry_price = Some(price);
            }
            OrderSide::Sell => {
                let proceeds = quantity * price * (1.0 - commission - slippage);
                self.current_capital += proceeds;
                
                if self.position > 0.0 {

                    let entry = self.position_entry_price.unwrap_or(price);
                    pnl = self.position.abs() * (price - entry);
                }
                
                self.position -= quantity;
                self.position_entry_price = Some(price);
            }
        }
        let position_value = self.position * price;
        let total_value = self.current_capital + position_value;
        
        self.equity_curve.push(total_value);
        

        let peak = self.equity_curve.iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(&self.initial_capital);
        let drawdown = ((total_value - peak) / peak) * 100.0;
        self.drawdown_history.push(drawdown);
        
        self.total_trades += 1;
        if pnl > 0.0 {
            self.winning_trades += 1;
        }

        let trade = TradeRecord {
            bar,
            side,
            quantity,
            price,
            pnl,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.trade_history.push(trade.clone());
        trade
    }
}

pub struct LiveTradingEngine {
    sessions: Arc<Mutex<HashMap<String, TradingSession>>>,
}

impl LiveTradingEngine {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_session(&self, initial_capital: f64, params: StrategyParams) -> String {
        let session_id = format!("session_{}", uuid::Uuid::new_v4());
        let session = TradingSession::new(session_id.clone(), initial_capital, params);
        
        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id.clone(), session);
        
        session_id
    }

    pub fn start_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.is_active = true;
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    pub fn stop_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.is_active = false;
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    pub fn get_session(&self, session_id: &str) -> Option<TradingSession> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(session_id).cloned()
    }

    pub fn update_parameters(&self, session_id: &str, params: StrategyParams) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.strategy_params = params;
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    pub fn run_trading_loop<F>(&self, session_id: String, mut update_callback: F)
    where
        F: FnMut(MarketUpdate) + Send + 'static,
    {
        let sessions_clone = Arc::clone(&self.sessions);
        
        thread::spawn(move || {

            let symbol = "BTC/USD".to_string();
            let initial_price = 50000.0;
            let mut market = OrderBookMarket::new(symbol, initial_price);
            
            let mut price_history = Vec::new();
            let mut ekf_history = Vec::new();
            let mut bar_count = 0;
            

            market.initialize_depth(10, 10.0, 5.0);
            

            loop {

                let is_active = {
                    let sessions = sessions_clone.lock().unwrap();
                    sessions.get(&session_id)
                        .map(|s| s.is_active)
                        .unwrap_or(false)
                };
                
                if !is_active {
                    break;
                }

                let start_time = Instant::now();
                

                market.step();
                

                let current_price = market.get_mid_price();
                
                let latency = start_time.elapsed().as_micros();
                

                let ekf_estimate = if ekf_history.is_empty() {
                    current_price
                } else {
                    let prev = ekf_history.last().unwrap();
                    prev * 0.95 + current_price * 0.05
                };
                
                price_history.push(current_price);
                ekf_history.push(ekf_estimate);
                

                if price_history.len() > 200 {
                    price_history.remove(0);
                    ekf_history.remove(0);
                }
                

                let (session, new_trade) = {
                    let sessions = sessions_clone.lock().unwrap();
                    let session = sessions.get(&session_id).cloned();
                    (session, None)
                };
                
                if let Some(session) = session {
                    let update = MarketUpdate {
                        bar_number: bar_count as usize,
                        current_price,
                        ekf_estimate,
                        position: session.position,
                        best_bid: current_price - 5.0,
                        best_ask: current_price + 5.0,
                        mid_price: current_price,
                        latency,
                        stats: session.calculate_stats(),
                        equity_history: session.equity_curve.clone(),
                        price_history: price_history.clone(),
                        ekf_history: ekf_history.clone(),
                        drawdown_history: session.drawdown_history.clone(),
                        new_trade,
                    };
                    
                    update_callback(update);
                }
                
                bar_count += 1;
                

                thread::sleep(Duration::from_millis(200));
            }
        });
    }
    
    pub fn execute_manual_trade(&self, session_id: &str, side: OrderSide, price: f64) -> Result<TradeRecord, String> {
        let mut sessions = self.sessions.lock().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            let quantity = 1.0;
            let trade = session.execute_trade(side, quantity, price, 0);
            Ok(trade)
        } else {
            Err("Session not found".to_string())
        }
    }
}
mod uuid {
    use std::time::SystemTime;
    
    pub struct Uuid;
    
    impl Uuid {
        pub fn new_v4() -> String {
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_micros();
            format!("{:x}", timestamp)
        }
    }
}
