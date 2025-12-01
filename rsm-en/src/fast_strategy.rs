/// High-performance trading strategy using compile-time optimization
/// Uses const generics (Rust's template metaprogramming) for zero-cost abstractions

use crate::strategy::{Signal, Strategy, StrategyContext};

/// Fixed-size ring buffer for price history - compile-time sized for cache efficiency
#[derive(Clone)]
struct RingBuffer<const N: usize> {
    data: [f64; N],
    index: usize,
    filled: bool,
}

impl<const N: usize> RingBuffer<N> {
    #[inline(always)]
    const fn new() -> Self {
        Self {
            data: [0.0; N],
            index: 0,
            filled: false,
        }
    }

    #[inline(always)]
    fn push(&mut self, value: f64) {
        self.data[self.index] = value;
        self.index += 1;
        if self.index >= N {
            self.index = 0;
            self.filled = true;
        }
    }

    #[inline(always)]
    fn is_full(&self) -> bool {
        self.filled
    }

    #[inline(always)]
    fn get(&self, lookback: usize) -> Option<f64> {
        if lookback >= N || (!self.filled && lookback >= self.index) {
            return None;
        }
        let idx = if self.index >= lookback + 1 {
            self.index - lookback - 1
        } else {
            N + self.index - lookback - 1
        };
        Some(self.data[idx])
    }

    #[inline(always)]
    fn latest(&self) -> Option<f64> {
        if self.index == 0 && !self.filled {
            None
        } else {
            Some(self.data[(self.index + N - 1) % N])
        }
    }
}

/// Compile-time sized EKF state - fits in CPU registers
#[repr(C, align(32))] // Cache line alignment
#[derive(Clone, Copy)]
struct EKFState {
    price: f64,
    velocity: f64,
    _padding: [f64; 2], // Align to 32 bytes for SIMD
}

/// Ultra-fast EKF with compile-time matrix sizes
#[derive(Clone)]
struct FastEKF<const DT: i32> {
    state: EKFState,
    // Simplified covariance - only diagonal (assumes independence)
    p_diag: [f64; 2], // [price_var, velocity_var]
    q_diag: [f64; 2], // Process noise
    r: f64,           // Measurement noise
}

impl<const DT: i32> FastEKF<DT> {
    #[inline(always)]
    const fn new(initial_price: f64) -> Self {
        Self {
            state: EKFState {
                price: initial_price,
                velocity: 0.0,
                _padding: [0.0; 2],
            },
            p_diag: [100.0, 10.0],
            q_diag: [0.01, 0.1],
            r: 0.5,
        }
    }

    #[inline(always)]
    fn predict(&mut self) {
        let dt = (DT as f64) / 1000.0; // Compile-time constant
        
        // State transition (unrolled at compile time)
        let new_price = self.state.price + self.state.velocity * dt;
        
        // Covariance prediction (diagonal only)
        self.p_diag[0] += self.p_diag[1] * dt * dt + self.q_diag[0];
        self.p_diag[1] += self.q_diag[1];
        
        self.state.price = new_price;
    }

    #[inline(always)]
    fn update(&mut self, price_obs: f64, velocity_obs: f64) {
        // Innovation
        let y_price = price_obs - self.state.price;
        let y_velocity = velocity_obs - self.state.velocity;
        
        // Kalman gains (simplified)
        let s_price = self.p_diag[0] + self.r;
        let s_velocity = self.p_diag[1] + self.r;
        
        let k_price = self.p_diag[0] / s_price;
        let k_velocity = self.p_diag[1] / s_velocity;
        
        // State update
        self.state.price += k_price * y_price;
        self.state.velocity += k_velocity * y_velocity;
        
        // Covariance update
        self.p_diag[0] *= 1.0 - k_price;
        self.p_diag[1] *= 1.0 - k_velocity;
    }

    #[inline(always)]
    fn filter(&mut self, price_obs: f64, velocity_obs: f64) -> (f64, f64) {
        self.predict();
        self.update(price_obs, velocity_obs);
        (self.state.price, self.state.velocity)
    }
}

/// Fast market maker with compile-time optimizations
/// N = lookback period, fixed at compile time for loop unrolling
pub struct FastMarketMaker<const N: usize> {
    ekf: FastEKF<1000>, // DT = 1000 (1.0 time units)
    price_history: RingBuffer<N>,
    max_inventory: f64,
    prev_price: Option<f64>,
}

impl<const N: usize> FastMarketMaker<N> {
    pub const fn new(max_inventory: f64) -> Self {
        Self {
            ekf: FastEKF::new(50000.0),
            price_history: RingBuffer::new(),
            max_inventory,
            prev_price: None,
        }
    }
}

impl<const N: usize> Strategy for FastMarketMaker<N> {
    #[inline(always)]
    fn name(&self) -> &str {
        "Fast Market Maker"
    }

    #[inline] // Force inlining for hot path
    fn generate_signal(&mut self, context: &StrategyContext) -> Signal {
        let current_price = context.current_bar().close;

        // Calculate velocity (fast path)
        let velocity_obs = match self.prev_price {
            Some(prev) => current_price - prev,
            None => {
                self.prev_price = Some(current_price);
                self.price_history.push(current_price);
                return Signal::Hold;
            }
        };

        // EKF update - highly optimized
        let (price_est, _velocity_est) = self.ekf.filter(current_price, velocity_obs);
        
        self.prev_price = Some(current_price);
        self.price_history.push(current_price);

        // Get current inventory from context (backtester manages this)
        let inventory = context.position.map(|p| p.quantity).unwrap_or(0.0);

        // Fast deviation calculation with inventory adjustment
        let price_deviation_bps = ((current_price - price_est) / price_est) * 10000.0;
        let inventory_adj = inventory * 5.0;

        // Branch prediction friendly - most common case first (Hold)
        // Only trade on significant deviations adjusted for inventory risk
        // Threshold: 10 bps base + inventory adjustment
        if price_deviation_bps < -10.0 + inventory_adj && inventory.abs() < self.max_inventory {
            Signal::Buy
        } else if price_deviation_bps > 10.0 + inventory_adj && inventory.abs() < self.max_inventory {
            Signal::Sell
        } else {
            Signal::Hold
        }
    }

    #[inline(always)]
    fn reset(&mut self) {
        self.ekf = FastEKF::new(50000.0);
        self.price_history = RingBuffer::new();
        self.prev_price = None;
    }
}

// Type aliases for common configurations (monomorphization optimization)
pub type FastMM60 = FastMarketMaker<60>;
pub type FastMM100 = FastMarketMaker<100>;
pub type FastMM200 = FastMarketMaker<200>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let mut rb: RingBuffer<4> = RingBuffer::new();
        rb.push(1.0);
        rb.push(2.0);
        rb.push(3.0);
        rb.push(4.0);
        
        assert!(rb.is_full());
        assert_eq!(rb.latest(), Some(4.0));
        assert_eq!(rb.get(0), Some(3.0));
    }

    #[test]
    fn test_fast_ekf() {
        let mut ekf: FastEKF<1000> = FastEKF::new(50000.0);
        let (price, _) = ekf.filter(50100.0, 100.0);
        assert!(price > 49000.0 && price < 51000.0);
    }

    #[test]
    fn test_fast_market_maker() {
        let strategy: FastMM60 = FastMarketMaker::new(5.0);
        assert_eq!(strategy.name(), "Fast Market Maker");
    }
}
