/// Simple HTTP server for live trading dashboard
/// Uses long-polling for real-time updates

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
use tiny_http::{Server, Response, Method, Header};
use serde::{Deserialize, Serialize};

use rsm_en::live_trading::{LiveTradingEngine, StrategyParams, MarketUpdate};
use rsm_en::market::OrderSide;

#[derive(Debug, Deserialize)]
struct StartRequest {
    max_inventory: f64,
    entry_threshold: f64,
    process_noise: f64,
    measurement_noise: f64,
    lookback: usize,
    initial_capital: f64,
}

#[derive(Debug, Serialize)]
struct StartResponse {
    success: bool,
    session_id: Option<String>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct StopRequest {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct PollRequest {
    session_id: String,
}

type UpdateQueue = Arc<Mutex<HashMap<String, Vec<MarketUpdate>>>>;

fn main() {
    println!("PhlopChain Live Trading Server");
    println!("==============================\n");
    
    let server = Server::http("127.0.0.1:8080").unwrap();
    let engine = Arc::new(LiveTradingEngine::new());
    let updates: UpdateQueue = Arc::new(Mutex::new(HashMap::new()));

    println!("🚀 Server running on http://127.0.0.1:8080");
    println!("📊 Trading Dashboard: http://127.0.0.1:8080/trading.html");
    println!("⛏️  Mining Interface: http://127.0.0.1:8080/index.html");
    println!("\nPress Ctrl+C to stop\n");

    for request in server.incoming_requests() {
        let engine = Arc::clone(&engine);
        let updates = Arc::clone(&updates);
        
        thread::spawn(move || {
            handle_request(request, engine, updates);
        });
    }
}

fn handle_request(
    mut request: tiny_http::Request,
    engine: Arc<LiveTradingEngine>,
    updates: UpdateQueue,
) {
    let url = request.url().to_string();
    let method = request.method().clone();

    // CORS headers
    let cors_header = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();
    let content_type = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();

    match (method, url.as_str()) {
        // Serve static files
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_file(request, "static/index.html");
            return;
        }
        (Method::Get, "/trading.html") => {
            serve_file(request, "static/trading.html");
            return;
        }
        
        // API: Start trading
        (Method::Post, "/api/trading/start") => {
            let mut content = String::new();
            request.as_reader().read_to_string(&mut content).unwrap();
            
            if let Ok(req) = serde_json::from_str::<StartRequest>(&content) {
                let params = StrategyParams {
                    max_inventory: req.max_inventory,
                    entry_threshold: req.entry_threshold,
                    process_noise: req.process_noise,
                    measurement_noise: req.measurement_noise,
                    lookback: req.lookback,
                };
                
                let session_id = engine.create_session(req.initial_capital, params);
                engine.start_session(&session_id).unwrap();
                
                // Start update collection thread
                let engine_clone = Arc::clone(&engine);
                let updates_clone = Arc::clone(&updates);
                let session_id_clone = session_id.clone();
                
                thread::spawn(move || {
                    collect_updates(session_id_clone, engine_clone, updates_clone);
                });
                
                let response = StartResponse {
                    success: true,
                    session_id: Some(session_id),
                    message: "Trading session started".to_string(),
                };
                
                let json = serde_json::to_string(&response).unwrap();
                let response = Response::from_string(json)
                    .with_header(cors_header)
                    .with_header(content_type);
                let _ = request.respond(response);
            }
        }
        
        // API: Stop trading
        (Method::Post, "/api/trading/stop") => {
            let mut content = String::new();
            request.as_reader().read_to_string(&mut content).unwrap();
            
            if let Ok(req) = serde_json::from_str::<StopRequest>(&content) {
                let result = engine.stop_session(&req.session_id);
                
                let (success, message) = match result {
                    Ok(_) => (true, "Session stopped".to_string()),
                    Err(e) => (false, e),
                };
                
                let response = StartResponse {
                    success,
                    session_id: None,
                    message,
                };
                
                let json = serde_json::to_string(&response).unwrap();
                let response = Response::from_string(json)
                    .with_header(cors_header)
                    .with_header(content_type);
                let _ = request.respond(response);
            }
        }
        
        // API: Poll for updates (long-polling)
        (Method::Post, "/api/trading/poll") => {
            let mut content = String::new();
            request.as_reader().read_to_string(&mut content).unwrap();
            
            if let Ok(req) = serde_json::from_str::<PollRequest>(&content) {
                // Get available updates
                let mut update_list = updates.lock().unwrap();
                let session_updates = update_list.entry(req.session_id.clone()).or_insert_with(Vec::new);
                
                if let Some(update) = session_updates.pop() {
                    let json = serde_json::to_string(&update).unwrap();
                    let response = Response::from_string(json)
                        .with_header(cors_header)
                        .with_header(content_type);
                    let _ = request.respond(response);
                } else {
                    // No updates available, send empty
                    let response = Response::from_string("{}")
                        .with_header(cors_header)
                        .with_header(content_type);
                    let _ = request.respond(response);
                }
            }
        }
        
        // API: Execute manual trade
        (Method::Post, "/api/trading/trade") => {
            let mut content = String::new();
            request.as_reader().read_to_string(&mut content).unwrap();
            
            #[derive(Deserialize)]
            struct TradeRequest {
                session_id: String,
                side: String,
                price: f64,
            }
            
            if let Ok(req) = serde_json::from_str::<TradeRequest>(&content) {
                let side = if req.side == "sell" {
                    OrderSide::Sell
                } else {
                    OrderSide::Buy
                };
                
                match engine.execute_manual_trade(&req.session_id, side, req.price) {
                    Ok(trade) => {
                        let json = serde_json::to_string(&trade).unwrap();
                        let response = Response::from_string(json)
                            .with_header(cors_header)
                            .with_header(content_type);
                        let _ = request.respond(response);
                    }
                    Err(e) => {
                        let json = format!("{{\"error\": \"{}\"}}", e);
                        let response = Response::from_string(json)
                            .with_header(cors_header)
                            .with_header(content_type);
                        let _ = request.respond(response);
                    }
                }
            }
        }
        
        _ => {
            let response = Response::from_string("Not Found").with_status_code(404);
            let _ = request.respond(response);
        }
    }
}

fn serve_file(request: tiny_http::Request, path: &str) {
    if let Ok(content) = std::fs::read_to_string(path) {
        let content_type = if path.ends_with(".html") {
            "text/html"
        } else if path.ends_with(".js") {
            "application/javascript"
        } else {
            "text/plain"
        };
        
        let header = Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap();
        let response = Response::from_string(content).with_header(header);
        let _ = request.respond(response);
    } else {
        let response = Response::from_string("File not found").with_status_code(404);
        let _ = request.respond(response);
    }
}

fn collect_updates(
    session_id: String,
    engine: Arc<LiveTradingEngine>,
    updates: UpdateQueue,
) {
    engine.run_trading_loop(session_id.clone(), move |update| {
        let mut update_list = updates.lock().unwrap();
        let session_updates = update_list.entry(session_id.clone()).or_insert_with(|| Vec::new());
        session_updates.push(update);
        
        // Keep only last 10 updates
        if session_updates.len() > 10 {
            session_updates.remove(0);
        }
    });
}
