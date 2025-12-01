# Live Trading Dashboard

The trading server is running at http://127.0.0.1:8080/trading.html

## Features Implemented:
1. Real-time P&L tracking with live updates
2. Adjustable strategy parameters:
   - Max Inventory (1-10 units)
   - Entry Threshold (5-100 bps)
   - EKF Process Noise (0.001-0.1)
   - Measurement Noise (0.1-2.0)
   - Lookback Period (20-200 bars)

3. Live market data display:
   - Current price, EKF estimate, position
   - Best bid/ask, mid price
   - Latency tracking (microseconds)

4. Real-time charts:
   - Equity curve
   - Price & EKF estimate overlay
   - Drawdown tracking

5. Trade log with P&L per trade
6. Performance statistics

## How to Use:
1. Access: http://127.0.0.1:8080/trading.html
2. Adjust parameters using the sliders
3. Click "START TRADING" to begin
4. Watch real-time P&L and charts update
5. Tweak parameters on-the-fly
6. Click "STOP TRADING" to halt
7. Click "RESET STRATEGY" to start fresh

## API Endpoints:
- POST /api/trading/start - Start trading session
- POST /api/trading/stop - Stop trading session  
- POST /api/trading/poll - Poll for market updates (long-polling)

