use crate::market::{MarketBar, OrderSide};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub side: OrderSide,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    Buy,
    Sell,
    Hold,
}
pub struct StrategyContext<'a> {
    pub bars: &'a [MarketBar],
    pub current_index: usize,
    pub position: Option<&'a Position>,
}

impl<'a> StrategyContext<'a> {
    pub fn current_bar(&self) -> &MarketBar {
        &self.bars[self.current_index]
    }

    pub fn lookback(&self, periods: usize) -> &[MarketBar] {
        let start = self.current_index.saturating_sub(periods);
        &self.bars[start..=self.current_index]
    }
}
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn generate_signal(&mut self, context: &StrategyContext) -> Signal;
    fn reset(&mut self);
}
#[derive(Debug, Clone)]
struct ExtendedKalmanFilter {

    state: [f64; 3],

    p: [[f64; 3]; 3],

    q: [[f64; 3]; 3],

    r: f64,

    momentum_velocity: [f64; 3],
    momentum_acceleration: [f64; 3],
    nesterov_beta: f64,
    nesterov_gamma: f64,

    dt: f64,
}

impl ExtendedKalmanFilter {
    fn new(initial_price: f64, dt: f64) -> Self {

        let state = [initial_price, 0.0, 0.0];
        

        let mut p = [[0.0; 3]; 3];
        p[0][0] = 100.0;
        p[1][1] = 10.0;
        p[2][2] = 1.0;
        

        let mut q = [[0.0; 3]; 3];
        q[0][0] = 0.01;
        q[1][1] = 0.1;
        q[2][2] = 0.5;
        
        Self {
            state,
            p,
            q,
            r: 0.5,
            momentum_velocity: [0.0; 3],
            momentum_acceleration: [0.0; 3],
            nesterov_beta: 0.9,
            nesterov_gamma: 0.999,
            dt,
        }
    }

    fn predict(&mut self) {
        let lookahead = [
            self.state[0] + self.nesterov_gamma * self.momentum_velocity[0],
            self.state[1] + self.nesterov_gamma * self.momentum_velocity[1],
            self.state[2] + self.nesterov_gamma * self.momentum_velocity[2],
        ];
        


        
        let dt = self.dt;
        let dt2 = dt * dt;
        

        let f = [
            [1.0, dt, 0.5 * dt2],
            [0.0, 1.0, dt],
            [0.0, 0.0, 1.0],
        ];
        

        let mut new_state = [0.0; 3];
        for i in 0..3 {
            for j in 0..3 {
                new_state[i] += f[i][j] * lookahead[j];
            }
        }
        

        for i in 0..3 {
            let velocity_update = new_state[i] - self.state[i];
            self.momentum_acceleration[i] = self.nesterov_beta * self.momentum_acceleration[i] 
                                          + (1.0 - self.nesterov_beta) * velocity_update;
            self.momentum_velocity[i] = self.nesterov_beta * self.momentum_velocity[i] 
                                      + self.momentum_acceleration[i];
        }
        
        self.state = new_state;
        

        let mut fp = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    fp[i][j] += f[i][k] * self.p[k][j];
                }
            }
        }
        
        let mut fpft = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    fpft[i][j] += fp[i][k] * f[j][k];
                }
            }
        }
        

        for i in 0..3 {
            for j in 0..3 {
                self.p[i][j] = fpft[i][j] + self.q[i][j];
            }
        }
    }

    fn update(&mut self, measurements: [f64; 2]) {

        

        let y = [
            measurements[0] - self.state[0],
            measurements[1] - self.state[1],
        ];
        


        let mut s = [[0.0; 2]; 2];
        for i in 0..2 {
            for j in 0..2 {
                s[i][j] = self.p[i][j];
            }
            s[i][i] += self.r;
        }
        

        let det = s[0][0] * s[1][1] - s[0][1] * s[1][0];
        
        let s_inv = if det.abs() > 1e-10 {
            let inv_det = 1.0 / det;
            [
                [inv_det * s[1][1], -inv_det * s[0][1]],
                [-inv_det * s[1][0], inv_det * s[0][0]],
            ]
        } else {

            [[1.0, 0.0], [0.0, 1.0]]
        };
        

        let mut k = [[0.0; 2]; 3];
        for i in 0..3 {
            for j in 0..2 {
                for m in 0..2 {
                    k[i][j] += self.p[i][m] * s_inv[m][j];
                }
            }
        }
        

        for i in 0..3 {
            for j in 0..2 {
                self.state[i] += k[i][j] * y[j];
            }
        }
        


        let mut kh = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..2 {
                kh[i][j] = k[i][j];
            }
        }
        
        let mut i_kh = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                i_kh[i][j] = if i == j { 1.0 } else { 0.0 } - kh[i][j];
            }
        }
        
        let mut new_p = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    new_p[i][j] += i_kh[i][k] * self.p[k][j];
                }
            }
        }
        
        self.p = new_p;
    }

    fn filter(&mut self, measurements: [f64; 2]) -> (f64, f64, f64) {
        self.predict();
        self.update(measurements);
        (self.state[0], self.state[1], self.state[2])
    }

    fn get_state(&self) -> [f64; 3] {
        self.state
    }
    
    fn get_price(&self) -> f64 {
        self.state[0]
    }
    
    fn get_velocity(&self) -> f64 {
        self.state[1]
    }
    
    fn get_acceleration(&self) -> f64 {
        self.state[2]
    }
}
#[derive(Debug, Clone)]
pub struct KalmanStatArb {

    ekf: ExtendedKalmanFilter,

    lookback_period: usize,

    spread_width_bps: f64,

    max_inventory: f64,

    price_history: Vec<f64>,

    inventory: f64,

    target_inventory: f64,

    inventory_skew: f64,

    prev_price: Option<f64>,

    last_price_estimate: f64,

    last_velocity_estimate: f64,
}

impl KalmanStatArb {
    pub fn new(
        lookback_period: usize,
        spread_width_bps: f64,
        max_inventory: f64,
        inventory_skew: f64,
    ) -> Self {

        let dt = 1.0;
        
        Self {
            ekf: ExtendedKalmanFilter::new(50000.0, dt),
            lookback_period,
            spread_width_bps,
            max_inventory,
            price_history: Vec::new(),
            inventory: 0.0,
            target_inventory: 0.0,
            inventory_skew,
            prev_price: None,
            last_price_estimate: 50000.0,
            last_velocity_estimate: 0.0,
        }
    }

    fn calculate_volatility(&self) -> f64 {
        if self.price_history.len() < 2 {
            return 100.0;
        }

        let window = self.lookback_period.min(self.price_history.len());
        let recent_prices: Vec<f64> = self.price_history
            .iter()
            .rev()
            .take(window)
            .copied()
            .collect();
        let mut returns = Vec::new();
        for i in 1..recent_prices.len() {
            let ret = (recent_prices[i-1] - recent_prices[i]) / recent_prices[i];
            returns.push(ret);
        }

        if returns.is_empty() {
            return 100.0;
        }

        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns
            .iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / returns.len() as f64;
        
        variance.sqrt().max(0.0001)
    }

    fn calculate_bid_ask(&self, true_price: f64, velocity: f64, volatility: f64) -> (f64, f64) {

        let half_spread = true_price * volatility * self.spread_width_bps / 10000.0;
        

        let velocity_adjustment = velocity * 0.5;
        

        let inventory_adjustment = self.inventory * self.inventory_skew * volatility * true_price;
        
        let bid = true_price - half_spread + velocity_adjustment - inventory_adjustment;
        let ask = true_price + half_spread + velocity_adjustment - inventory_adjustment;
        
        (bid, ask)
    }
    
    fn should_provide_liquidity(&self) -> bool {

        self.inventory.abs() < self.max_inventory
    }
}

impl Strategy for KalmanStatArb {
    fn name(&self) -> &str {
        "Kalman Filter Stat Arb"
    }

    fn generate_signal(&mut self, context: &StrategyContext) -> Signal {
        let current_bar = context.current_bar();
        let current_price = current_bar.close;
        let price_obs = current_price;
        

        let velocity_obs = if let Some(prev) = self.prev_price {
            current_price - prev
        } else {
            0.0
        };
        


        let measurements = [price_obs, velocity_obs];
        let (price_est, velocity_est, _acceleration_est) = self.ekf.filter(measurements);
        

        self.last_price_estimate = price_est;
        self.last_velocity_estimate = velocity_est;
        

        self.prev_price = Some(current_price);
        

        self.price_history.push(current_price);
        if self.price_history.len() < self.lookback_period {
            return Signal::Hold;
        }
        let volatility = self.calculate_volatility();
        

        
        let price_deviation_bps = ((current_price - price_est) / price_est) * 10000.0;
        

        let trade_threshold = 5.0;
        

        let inventory_adjustment = self.inventory * 10.0;
        

        if !self.should_provide_liquidity() {
            if self.inventory > 0.0 {
                return Signal::Sell;
            } else if self.inventory < 0.0 {
                return Signal::Buy;
            }
        }
        

        if price_deviation_bps < -trade_threshold + inventory_adjustment && self.inventory < self.max_inventory {

            return Signal::Buy;
        }
        
        if price_deviation_bps > trade_threshold + inventory_adjustment && self.inventory > -self.max_inventory {

            return Signal::Sell;
        }
        

        if self.inventory > 0.5 {
            return Signal::Sell;
        } else if self.inventory < -0.5 {
            return Signal::Buy;
        }

        Signal::Hold
    }

    fn reset(&mut self) {
        let dt = 1.0;
        self.ekf = ExtendedKalmanFilter::new(50000.0, dt);
        self.price_history.clear();
        self.inventory = 0.0;
        self.prev_price = None;
        self.last_price_estimate = 50000.0;
        self.last_velocity_estimate = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_market_maker_creation() {
        let strategy = KalmanStatArb::new(30, 50.0, 2.0, 0.05);
        assert_eq!(strategy.lookback_period, 30);
        assert_eq!(strategy.spread_width_bps, 50.0);
    }
}
