use crate::market::{MarketBar, OrderSide};
use serde::{Deserialize, Serialize};

/// Position in the market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub side: OrderSide,
}

/// Trading signal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    Buy,
    Sell,
    Hold,
}

/// Strategy context with market data
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

/// Base trait for trading strategies
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn generate_signal(&mut self, context: &StrategyContext) -> Signal;
    fn reset(&mut self);
}

/// Extended Kalman Filter with hierarchical state (position, velocity, acceleration)
#[derive(Debug, Clone)]
struct ExtendedKalmanFilter {
    // State vector: [price, velocity, acceleration]
    state: [f64; 3],
    // State covariance matrix (3x3)
    p: [[f64; 3]; 3],
    // Process noise covariance
    q: [[f64; 3]; 3],
    // Measurement noise
    r: f64,
    // Nesterov momentum parameters
    momentum_velocity: [f64; 3],
    momentum_acceleration: [f64; 3],
    nesterov_beta: f64,
    nesterov_gamma: f64,
    // Time step
    dt: f64,
}

impl ExtendedKalmanFilter {
    fn new(initial_price: f64, dt: f64) -> Self {
        // Initialize state: [price, velocity=0, acceleration=0]
        let state = [initial_price, 0.0, 0.0];
        
        // Initial covariance - higher uncertainty
        let mut p = [[0.0; 3]; 3];
        p[0][0] = 100.0;  // Price uncertainty
        p[1][1] = 10.0;   // Velocity uncertainty
        p[2][2] = 1.0;    // Acceleration uncertainty
        
        // Process noise - models system dynamics uncertainty
        let mut q = [[0.0; 3]; 3];
        q[0][0] = 0.01;   // Price process noise
        q[1][1] = 0.1;    // Velocity process noise
        q[2][2] = 0.5;    // Acceleration process noise
        
        Self {
            state,
            p,
            q,
            r: 0.5, // Measurement noise
            momentum_velocity: [0.0; 3],
            momentum_acceleration: [0.0; 3],
            nesterov_beta: 0.9,      // Momentum coefficient
            nesterov_gamma: 0.999,    // Nesterov lookahead
            dt,
        }
    }

    fn predict(&mut self) {
        // State transition with Nesterov momentum
        // First, compute lookahead position (Nesterov)
        let lookahead = [
            self.state[0] + self.nesterov_gamma * self.momentum_velocity[0],
            self.state[1] + self.nesterov_gamma * self.momentum_velocity[1],
            self.state[2] + self.nesterov_gamma * self.momentum_velocity[2],
        ];
        
        // Kinematic model: x_k+1 = F * x_k
        // price(t+1) = price(t) + velocity(t)*dt + 0.5*acceleration(t)*dt^2
        // velocity(t+1) = velocity(t) + acceleration(t)*dt
        // acceleration(t+1) = acceleration(t) (assume constant acceleration)
        
        let dt = self.dt;
        let dt2 = dt * dt;
        
        // State transition matrix F
        let f = [
            [1.0, dt, 0.5 * dt2],
            [0.0, 1.0, dt],
            [0.0, 0.0, 1.0],
        ];
        
        // Predict new state with Nesterov lookahead
        let mut new_state = [0.0; 3];
        for i in 0..3 {
            for j in 0..3 {
                new_state[i] += f[i][j] * lookahead[j];
            }
        }
        
        // Update momentum (for next iteration)
        for i in 0..3 {
            let velocity_update = new_state[i] - self.state[i];
            self.momentum_acceleration[i] = self.nesterov_beta * self.momentum_acceleration[i] 
                                          + (1.0 - self.nesterov_beta) * velocity_update;
            self.momentum_velocity[i] = self.nesterov_beta * self.momentum_velocity[i] 
                                      + self.momentum_acceleration[i];
        }
        
        self.state = new_state;
        
        // Predict covariance: P_k+1 = F * P_k * F^T + Q
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
                    fpft[i][j] += fp[i][k] * f[j][k]; // F^T is F transposed
                }
            }
        }
        
        // Add process noise
        for i in 0..3 {
            for j in 0..3 {
                self.p[i][j] = fpft[i][j] + self.q[i][j];
            }
        }
    }

    fn update(&mut self, measurements: [f64; 2]) {
        // Measurement model: z = H * x + v
        // Only observe price and velocity (acceleration is too noisy)
        // H = [[1, 0, 0],
        //      [0, 1, 0]]
        // measurements = [price_obs, velocity_obs]
        
        // Innovation (measurement residual): y = z - Hx
        let y = [
            measurements[0] - self.state[0],
            measurements[1] - self.state[1],
        ];
        
        // Innovation covariance: S = H * P * H^T + R
        // For 2x2 submatrix of P (top-left)
        let mut s = [[0.0; 2]; 2];
        for i in 0..2 {
            for j in 0..2 {
                s[i][j] = self.p[i][j];
            }
            s[i][i] += self.r; // Add measurement noise on diagonal
        }
        
        // Compute S^-1 (2x2 matrix inverse)
        let det = s[0][0] * s[1][1] - s[0][1] * s[1][0];
        
        let s_inv = if det.abs() > 1e-10 {
            let inv_det = 1.0 / det;
            [
                [inv_det * s[1][1], -inv_det * s[0][1]],
                [-inv_det * s[1][0], inv_det * s[0][0]],
            ]
        } else {
            // Return identity if determinant is too small
            [[1.0, 0.0], [0.0, 1.0]]
        };
        
        // Kalman gain: K = P * H^T * S^-1
        // K is 3x2 (3 states, 2 measurements)
        // P * H^T gives us first 2 columns of P
        let mut k = [[0.0; 2]; 3];
        for i in 0..3 {
            for j in 0..2 {
                for m in 0..2 {
                    k[i][j] += self.p[i][m] * s_inv[m][j];
                }
            }
        }
        
        // Update state: x = x + K * y
        for i in 0..3 {
            for j in 0..2 {
                self.state[i] += k[i][j] * y[j];
            }
        }
        
        // Update covariance: P = (I - K * H) * P
        // K*H is 3x3 where H = [[1,0,0], [0,1,0]]
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

/// Kalman Filter Market Making Strategy
/// Uses EKF to estimate true price + velocity, then provides liquidity via bid/ask quotes
/// Profits from bid-ask spread while maintaining inventory neutrality
#[derive(Debug, Clone)]
pub struct KalmanStatArb {
    // Extended Kalman filter with 3-level state
    ekf: ExtendedKalmanFilter,
    // Lookback period for volatility estimation
    lookback_period: usize,
    // Bid-ask spread width in basis points of volatility
    spread_width_bps: f64,
    // Maximum inventory position (units)
    max_inventory: f64,
    // Price history for volatility calculation
    price_history: Vec<f64>,
    // Current inventory (positive = long, negative = short)
    inventory: f64,
    // Target inventory (usually 0 for neutrality)
    target_inventory: f64,
    // Inventory skew parameter (how much to adjust quotes based on inventory)
    inventory_skew: f64,
    // Previous price for velocity estimation
    prev_price: Option<f64>,
    // Last true price estimate
    last_price_estimate: f64,
    // Last velocity estimate
    last_velocity_estimate: f64,
}

impl KalmanStatArb {
    pub fn new(
        lookback_period: usize,
        spread_width_bps: f64,
        max_inventory: f64,
        inventory_skew: f64,
    ) -> Self {
        // Time step for kinematic model (assuming 1 unit per bar)
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
            return 100.0; // Default volatility
        }

        let window = self.lookback_period.min(self.price_history.len());
        let recent_prices: Vec<f64> = self.price_history
            .iter()
            .rev()
            .take(window)
            .copied()
            .collect();

        // Calculate returns
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
        
        variance.sqrt().max(0.0001) // Minimum volatility to avoid division by zero
    }

    fn calculate_bid_ask(&self, true_price: f64, velocity: f64, volatility: f64) -> (f64, f64) {
        // Base spread: half-width on each side
        let half_spread = true_price * volatility * self.spread_width_bps / 10000.0;
        
        // Adjust for velocity: if price is rising, widen ask and tighten bid
        let velocity_adjustment = velocity * 0.5;
        
        // Adjust for inventory: if long, lower both quotes to encourage selling
        let inventory_adjustment = self.inventory * self.inventory_skew * volatility * true_price;
        
        let bid = true_price - half_spread + velocity_adjustment - inventory_adjustment;
        let ask = true_price + half_spread + velocity_adjustment - inventory_adjustment;
        
        (bid, ask)
    }
    
    fn should_provide_liquidity(&self) -> bool {
        // Don't provide liquidity if inventory is at max
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

        // Construct 2D measurement vector - only price and velocity (no acceleration noise)
        let price_obs = current_price;
        
        // Estimate velocity from price changes (finite difference: dx/dt)
        let velocity_obs = if let Some(prev) = self.prev_price {
            current_price - prev
        } else {
            0.0
        };
        
        // Update EKF with price and velocity observations only
        // Acceleration is estimated internally by the filter, not observed directly
        let measurements = [price_obs, velocity_obs];
        let (price_est, velocity_est, _acceleration_est) = self.ekf.filter(measurements);
        
        // Store estimates for market making
        self.last_price_estimate = price_est;
        self.last_velocity_estimate = velocity_est;
        
        // Update tracking for next iteration
        self.prev_price = Some(current_price);
        
        // Store history
        self.price_history.push(current_price);

        // Need enough history to calculate volatility
        if self.price_history.len() < self.lookback_period {
            return Signal::Hold;
        }

        // Calculate current volatility
        let volatility = self.calculate_volatility();
        
        // Market making: trade when price deviates from estimate
        // Buy when current price is below estimate (cheap)
        // Sell when current price is above estimate (expensive)
        
        let price_deviation_bps = ((current_price - price_est) / price_est) * 10000.0;
        
        // Very tight threshold - trade frequently to make market
        let trade_threshold = 5.0; // Just 5 bps deviation
        
        // Inventory penalty: adjust behavior based on position
        let inventory_adjustment = self.inventory * 10.0; // Strong inventory aversion
        
        // Check if we need to urgently reduce inventory
        if !self.should_provide_liquidity() {
            if self.inventory > 0.0 {
                return Signal::Sell;
            } else if self.inventory < 0.0 {
                return Signal::Buy;
            }
        }
        
        // Market making signals
        if price_deviation_bps < -trade_threshold + inventory_adjustment && self.inventory < self.max_inventory {
            // Price below estimate - buy
            return Signal::Buy;
        }
        
        if price_deviation_bps > trade_threshold + inventory_adjustment && self.inventory > -self.max_inventory {
            // Price above estimate - sell
            return Signal::Sell;
        }
        
        // Aggressive inventory rebalancing
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
