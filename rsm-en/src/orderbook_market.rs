use crate::market::{MarketBar, Order, OrderBook, OrderSide, Trade};
use crate::traders::{TraderPopulation, TraderType};
use sha2::{Digest, Sha256};

/// Order book based market simulation with realistic depth
pub struct OrderBookMarket {
    pub orderbook: OrderBook,
    pub order_id_counter: u64,
    pub current_time: u64,
    pub seed: u64,
    pub traders: TraderPopulation,
    pub last_trade_price: f64,
    pub fair_value: f64,
}

impl OrderBookMarket {
    pub fn new(symbol: String, initial_price: f64) -> Self {
        Self {
            orderbook: OrderBook::new(symbol),
            order_id_counter: 0,
            current_time: 0,
            seed: 42,
            traders: TraderPopulation::new(),
            last_trade_price: initial_price,
            fair_value: initial_price,
        }
    }

    fn next_order_id(&mut self) -> u64 {
        self.order_id_counter += 1;
        self.order_id_counter
    }

    fn generate_random(&mut self) -> f64 {
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
        (value % 1000000) as f64 / 1000000.0
    }

    /// Build initial order book depth with market makers
    pub fn initialize_depth(&mut self, levels: usize, base_spread_bps: f64, quantity_per_level: f64) {
        let spread = self.fair_value * base_spread_bps / 10000.0;
        let mid_price = self.fair_value;

        // Add multiple levels on each side
        for i in 0..levels {
            let offset = spread * (1.0 + i as f64 * 0.5);
            let quantity = quantity_per_level * (0.8f64).powi(i as i32);

            // Bid side
            let bid_price = mid_price - offset;
            let bid_order = Order::new(
                self.next_order_id(),
                "MarketMaker".to_string(),
                self.orderbook.symbol.clone(),
                OrderSide::Buy,
                bid_price,
                quantity,
            );
            self.orderbook.add_order(bid_order);

            // Ask side
            let ask_price = mid_price + offset;
            let ask_order = Order::new(
                self.next_order_id(),
                "MarketMaker".to_string(),
                self.orderbook.symbol.clone(),
                OrderSide::Sell,
                ask_price,
                quantity,
            );
            self.orderbook.add_order(ask_order);
        }
    }

    /// Replenish order book depth (market makers adding liquidity)
    fn replenish_depth(&mut self, base_spread_bps: f64) {
        let mid_price = self.get_mid_price();
        let spread = mid_price * base_spread_bps / 10000.0;

        // Check if we need more depth at best levels
        let best_bid = self.orderbook.get_best_bid();
        let best_ask = self.orderbook.get_best_ask();

        // Add liquidity if spread is too wide or depth is low
        if let (Some((bid_price, bid_qty)), Some((ask_price, ask_qty))) = (best_bid, best_ask) {
            let current_spread = (ask_price - bid_price) / mid_price * 10000.0;

            // If spread is wider than 2x base, add tighter quotes
            if current_spread > base_spread_bps * 2.0 || bid_qty < 0.5 || ask_qty < 0.5 {
                let new_bid_price = mid_price - spread;
                let new_ask_price = mid_price + spread;

                let bid_order = Order::new(
                    self.next_order_id(),
                    "MarketMaker".to_string(),
                    self.orderbook.symbol.clone(),
                    OrderSide::Buy,
                    new_bid_price,
                    1.0 + self.generate_random() * 2.0,
                );
                self.orderbook.add_order(bid_order);

                let ask_order = Order::new(
                    self.next_order_id(),
                    "MarketMaker".to_string(),
                    self.orderbook.symbol.clone(),
                    OrderSide::Sell,
                    new_ask_price,
                    1.0 + self.generate_random() * 2.0,
                );
                self.orderbook.add_order(ask_order);
            }
        } else {
            // No depth - rebuild
            self.initialize_depth(5, base_spread_bps, 2.0);
        }
    }

    /// Update fair value with random walk
    fn update_fair_value(&mut self) {
        let volatility = 0.001; // 10 bps per step for more movement
        let drift = (self.generate_random() - 0.5) * volatility;
        self.fair_value *= 1.0 + drift;
    }

    /// Generate realistic trader activity
    fn generate_trader_orders(&mut self) -> Vec<Trade> {
        let mut all_trades = Vec::new();
        
        // Each trader has a chance to trade
        let num_active = (self.traders.traders.len() as f64 * 0.02) as usize; // 2% active per step
        let num_traders = self.traders.traders.len();
        
        for _ in 0..num_active.max(1) {
            let trader_idx = (self.generate_random() * num_traders as f64) as usize;
            let trader_type = self.traders.traders.get(trader_idx).map(|t| t.trader_type);
            
            if let Some(trader_type) = trader_type {
                let order = self.generate_order_for_trader_type(trader_type);
                if let Some(order) = order {
                    let trades = self.orderbook.add_order(order);
                    if let Some(last_trade) = trades.last() {
                        self.last_trade_price = last_trade.price;
                    }
                    all_trades.extend(trades);
                }
            }
        }

        all_trades
    }

    fn generate_order_for_trader_type(&mut self, trader_type: TraderType) -> Option<Order> {
        let mid_price = self.get_mid_price();
        
        // Different trader types have different behaviors
        let (side, price, quantity) = match trader_type {
            TraderType::Retail => {
                // Retail: small orders, wider spreads, market orders
                let side = if self.generate_random() > 0.5 { OrderSide::Buy } else { OrderSide::Sell };
                let price_offset = (self.generate_random() - 0.5) * 0.01; // 1% range
                let price = mid_price * (1.0 + price_offset);
                let quantity = 0.1 + self.generate_random() * 0.3;
                (side, price, quantity)
            },
            TraderType::Institutional => {
                // Institutional: large orders, passive, limit orders near mid
                let side = if self.fair_value > mid_price { OrderSide::Buy } else { OrderSide::Sell };
                let price_offset = (self.generate_random() - 0.5) * 0.002; // 0.2% range
                let price = mid_price * (1.0 + price_offset);
                let quantity = 2.0 + self.generate_random() * 5.0;
                (side, price, quantity)
            },
            TraderType::HFT => {
                // HFT: very tight spreads, small size, high frequency
                let side = if self.generate_random() > 0.5 { OrderSide::Buy } else { OrderSide::Sell };
                let price_offset = (self.generate_random() - 0.5) * 0.0005; // 0.05% range
                let price = mid_price * (1.0 + price_offset);
                let quantity = 0.05 + self.generate_random() * 0.1;
                (side, price, quantity)
            },
            TraderType::MarketMaker => {
                // Already handled in replenish_depth
                return None;
            },
            TraderType::Whale => {
                // Whale: very large orders, can move market
                let side = if self.fair_value > mid_price { OrderSide::Buy } else { OrderSide::Sell };
                let price_offset = (self.generate_random() - 0.5) * 0.005; // 0.5% range
                let price = mid_price * (1.0 + price_offset);
                let quantity = 10.0 + self.generate_random() * 20.0;
                (side, price, quantity)
            },
            TraderType::Momentum => {
                // Momentum: trade with recent direction
                let recent_trend = self.last_trade_price - self.fair_value;
                let side = if recent_trend > 0.0 { OrderSide::Buy } else { OrderSide::Sell };
                let price = if side == OrderSide::Buy { 
                    mid_price * 1.003 // Aggressive buy
                } else {
                    mid_price * 0.997 // Aggressive sell
                };
                let quantity = 0.5 + self.generate_random() * 1.5;
                (side, price, quantity)
            },
            TraderType::Arbitrageur => {
                // Arbitrageur: trade when price deviates from fair value
                let deviation = (mid_price - self.fair_value) / self.fair_value;
                if deviation.abs() < 0.001 {
                    return None; // No arb opportunity
                }
                let side = if deviation > 0.0 { OrderSide::Sell } else { OrderSide::Buy };
                let price = if side == OrderSide::Buy {
                    self.fair_value * 1.0005 // Willing to pay slight premium
                } else {
                    self.fair_value * 0.9995 // Willing to sell slight discount
                };
                let quantity = 1.0 + self.generate_random() * 3.0;
                (side, price, quantity)
            },
        };

        Some(Order::new(
            self.next_order_id(),
            format!("{:?}", trader_type),
            self.orderbook.symbol.clone(),
            side,
            price,
            quantity,
        ))
    }

    pub fn get_mid_price(&self) -> f64 {
        if let (Some((bid, _)), Some((ask, _))) = (self.orderbook.get_best_bid(), self.orderbook.get_best_ask()) {
            (bid + ask) / 2.0
        } else {
            self.last_trade_price
        }
    }

    pub fn get_spread_bps(&self) -> f64 {
        if let (Some((bid, _)), Some((ask, _))) = (self.orderbook.get_best_bid(), self.orderbook.get_best_ask()) {
            ((ask - bid) / self.get_mid_price()) * 10000.0
        } else {
            0.0
        }
    }

    /// Simulate one time step with order book dynamics
    pub fn step(&mut self) -> Vec<Trade> {
        self.current_time += 1;

        // Update fair value (random walk)
        self.update_fair_value();

        // Replenish market maker liquidity
        self.replenish_depth(10.0); // 10 bps base spread

        // Generate trader activity
        let trades = self.generate_trader_orders();

        trades
    }

    /// Simulate a full session and generate market bars
    pub fn simulate_session(&mut self, num_bars: usize) -> Vec<MarketBar> {
        let mut bars = Vec::new();

        println!("\nSimulating order book market with {} traders...", self.traders.traders.len());
        println!("Initial fair value: ${:.2}", self.fair_value);

        // Initialize order book
        self.initialize_depth(10, 10.0, 3.0);

        for bar_idx in 0..num_bars {
            let bar_start_time = self.current_time;
            let mut bar_trades = Vec::new();

            // Multiple steps per bar (reduced to 3 for speed)
            for _ in 0..3 {
                let trades = self.step();
                bar_trades.extend(trades);
            }

            if let Some(bar) = MarketBar::from_trades(bar_start_time, self.orderbook.symbol.clone(), &bar_trades) {
                bars.push(bar);
            } else {
                // No trades - use mid price
                let mid = self.get_mid_price();
                bars.push(MarketBar::new(
                    bar_start_time,
                    self.orderbook.symbol.clone(),
                    mid, mid, mid, mid, 0.0,
                ));
            }

            if (bar_idx + 1) % 1000 == 0 {
                let spread = self.get_spread_bps();
                println!("   Progress: {}/{} bars ({:.1}%) | Price: ${:.2} | Spread: {:.1} bps",
                    bar_idx + 1, num_bars, ((bar_idx + 1) as f64 / num_bars as f64) * 100.0,
                    self.get_mid_price(), spread);
            }
        }

        bars
    }

    pub fn print_trader_stats(&self) {
        self.traders.get_trader_stats().print();
    }
}
