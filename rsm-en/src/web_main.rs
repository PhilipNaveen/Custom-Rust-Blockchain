use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
mod balances;
mod system;
mod merkle;
mod transaction;
mod blockchain;
mod rps_mining;

use blockchain::Blockchain;
use transaction::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MinerSession {
    id: String,
    name: String,
    total_phlopcoin: f64,
    blocks_mined: u32,
    mining_history: Vec<MiningResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MiningResult {
    block_number: u32,
    phlopcoin_earned: f64,
    games_played: u64,
    rounds: u32,
    timestamp: String,
    block_hash: String,
}

#[derive(Debug, Deserialize)]
struct StartMiningRequest {
    miner_name: String,
}

#[derive(Debug, Deserialize)]
struct MineBlockRequest {
    session_id: String,
}

#[derive(Debug, Serialize)]
struct MiningResponse {
    success: bool,
    message: String,
    session: Option<MinerSession>,
    mining_result: Option<MiningResult>,
}

#[derive(Debug, Serialize)]
struct BlockchainStatus {
    total_blocks: usize,
    total_games_played: u64,
    current_difficulty_score: f64,
    active_miners: usize,
}

type SharedBlockchain = Arc<Mutex<Blockchain>>;
type SharedSessions = Arc<Mutex<HashMap<String, MinerSession>>>;

fn main() {
    println!("🌐 PhlopChain Web Interface starting on http://localhost:3030");
    println!("📖 Visit http://localhost:3030 in your browser to start mining!");
    
    let blockchain = Arc::new(Mutex::new(Blockchain::new()));
    let sessions: SharedSessions = Arc::new(Mutex::new(HashMap::new()));

    let listener = TcpListener::bind("0.0.0.0:3030").unwrap();
    println!("PhlopChain web server running on http://0.0.0.0:3030");
    
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let blockchain_clone = Arc::clone(&blockchain);
        let sessions_clone = Arc::clone(&sessions);
        
        thread::spawn(move || {
            handle_connection(stream, blockchain_clone, sessions_clone);
        });
    }
}

fn handle_connection(mut stream: TcpStream, blockchain: SharedBlockchain, sessions: SharedSessions) {
    let mut buffer = [0; 4096];
    let bytes_read = stream.read(&mut buffer).unwrap_or(0);
    
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request_line = request.lines().next().unwrap_or("");
    
    println!("Received request: {}", request_line);
    
    let (status_line, contents) = if request_line.starts_with("GET / ") {
        println!("📄 Serving index page...");
        ("HTTP/1.1 200 OK".to_string(), get_index_html())
    } else if request_line.starts_with("OPTIONS") {

        ("HTTP/1.1 200 OK".to_string(), String::new())
    } else if request_line.starts_with("POST /api/start") {
        handle_start_mining(&request, sessions)
    } else if request_line.starts_with("POST /api/mine") {
        handle_mine_block(&request, blockchain, sessions)
    } else if request_line.starts_with("GET /api/blockchain") {
        handle_blockchain_status(blockchain, sessions)
    } else if request_line.starts_with("GET /api/history") {
        handle_mining_history(blockchain, sessions)
    } else if request_line.starts_with("GET /api/status/") {
        let session_id = extract_session_id(request_line);
        handle_get_status(&session_id, sessions)
    } else {
        ("HTTP/1.1 404 NOT FOUND".to_string(), "404 Not Found".to_string())
    };
    
    let response = format!(
        "{}\r\nContent-Type: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n\r\n{}",
        status_line,
        if contents.starts_with("{") || contents.starts_with("[") { "application/json" } else { "text/html" },
        contents
    );
    
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn extract_body(request: &str) -> String {
    if let Some(body_start) = request.find("\r\n\r\n") {
        let body = &request[body_start + 4..];

        body.trim_matches('\0').trim().to_string()
    } else {
        String::new()
    }
}

fn extract_session_id(request_line: &str) -> String {
    if let Some(path) = request_line.split_whitespace().nth(1) {
        if let Some(id) = path.strip_prefix("/api/status/") {
            return id.to_string();
        }
    }
    String::new()
}

fn handle_start_mining(request: &str, sessions: SharedSessions) -> (String, String) {
    let body = extract_body(request);
    println!("Received start mining request body: '{}'", body);
    
    if let Ok(req) = serde_json::from_str::<StartMiningRequest>(&body) {
        let session_id = generate_uuid();
        let session = MinerSession {
            id: session_id.clone(),
            name: req.miner_name,
            total_phlopcoin: 0.0,
            blocks_mined: 0,
            mining_history: Vec::new(),
        };
        
        let mut sessions_guard = sessions.lock().unwrap();
        sessions_guard.insert(session_id, session.clone());
        
        let response = MiningResponse {
            success: true,
            message: "Mining session started successfully!".to_string(),
            session: Some(session),
            mining_result: None,
        };
        
        ("HTTP/1.1 200 OK".to_string(), serde_json::to_string(&response).unwrap())
    } else {
        ("HTTP/1.1 400 BAD REQUEST".to_string(), "Invalid request".to_string())
    }
}

fn handle_mine_block(request: &str, blockchain: SharedBlockchain, sessions: SharedSessions) -> (String, String) {
    let body = extract_body(request);
    println!("Received mine block request body: '{}'", body);
    
    if let Ok(req) = serde_json::from_str::<MineBlockRequest>(&body) {
        let mut sessions_guard = sessions.lock().unwrap();
        if let Some(session) = sessions_guard.get_mut(&req.session_id) {

            let tx1 = Transaction::new(
                "alice".to_string(),
                session.name.clone(),
                5,
                1,
            );
            let tx2 = Transaction::new(
                session.name.clone(),
                "bob".to_string(),
                3,
                session.blocks_mined + 1,
            );
            
            let mut blockchain_guard = blockchain.lock().unwrap();
            

            let _ = blockchain_guard.add_transaction(tx1);
            let _ = blockchain_guard.add_transaction(tx2);
            
            match blockchain_guard.mine_pending_transactions(session.name.clone()) {
                Ok(block) => {
                    if let Some(ref rps_result) = block.rps_mining_result {
                        let min_games_needed = calculate_minimum_games_needed(&blockchain_guard);
                        let actual_games = rps_result.total_games as f64;
                        let phlopcoin_earned = min_games_needed / (actual_games * actual_games);
                        
                        let mining_result = MiningResult {
                            block_number: block.index,
                            phlopcoin_earned,
                            games_played: rps_result.total_games,
                            rounds: rps_result.rounds,
                            timestamp: format_timestamp(std::time::SystemTime::now()),
                            block_hash: format!("{}", block.hash),
                        };
                        
                        session.total_phlopcoin += phlopcoin_earned;
                        session.blocks_mined += 1;
                        session.mining_history.push(mining_result.clone());
                        
                        let response = MiningResponse {
                            success: true,
                            message: format!("Block #{} mined successfully! Earned {:.6} PhlopCoin", block.index, phlopcoin_earned),
                            session: Some(session.clone()),
                            mining_result: Some(mining_result),
                        };
                        
                        ("HTTP/1.1 200 OK".to_string(), serde_json::to_string(&response).unwrap())
                    } else {
                        let response = MiningResponse {
                            success: false,
                            message: "Mining failed - no RPS result".to_string(),
                            session: Some(session.clone()),
                            mining_result: None,
                        };
                        ("HTTP/1.1 500 INTERNAL SERVER ERROR".to_string(), serde_json::to_string(&response).unwrap())
                    }
                }
                Err(e) => {
                    let response = MiningResponse {
                        success: false,
                        message: format!("Mining failed: {}", e),
                        session: Some(session.clone()),
                        mining_result: None,
                    };
                    ("HTTP/1.1 500 INTERNAL SERVER ERROR".to_string(), serde_json::to_string(&response).unwrap())
                }
            }
        } else {
            ("HTTP/1.1 404 NOT FOUND".to_string(), "Session not found".to_string())
        }
    } else {
        ("HTTP/1.1 400 BAD REQUEST".to_string(), "Invalid request".to_string())
    }
}

fn handle_blockchain_status(blockchain: SharedBlockchain, sessions: SharedSessions) -> (String, String) {
    let blockchain_guard = blockchain.lock().unwrap();
    let sessions_guard = sessions.lock().unwrap();
    
    let status = BlockchainStatus {
        total_blocks: blockchain_guard.get_chain_length(),
        total_games_played: blockchain_guard.get_total_rps_games(),
        current_difficulty_score: blockchain_guard.get_rps_difficulty_info().difficulty_score(),
        active_miners: sessions_guard.len(),
    };
    
    ("HTTP/1.1 200 OK".to_string(), serde_json::to_string(&status).unwrap())
}

fn handle_mining_history(_blockchain: SharedBlockchain, sessions: SharedSessions) -> (String, String) {
    let sessions_guard = sessions.lock().unwrap();
    

    let mut all_mining_history: Vec<MiningResult> = Vec::new();
    
    for session in sessions_guard.values() {
        all_mining_history.extend(session.mining_history.clone());
    }
    

    all_mining_history.sort_by(|a, b| b.block_number.cmp(&a.block_number));
    

    let recent_history: Vec<MiningResult> = all_mining_history.into_iter().take(20).collect();
    
    ("HTTP/1.1 200 OK".to_string(), serde_json::to_string(&recent_history).unwrap())
}

fn handle_get_status(session_id: &str, sessions: SharedSessions) -> (String, String) {
    let sessions_guard = sessions.lock().unwrap();
    if let Some(session) = sessions_guard.get(session_id) {
        ("HTTP/1.1 200 OK".to_string(), serde_json::to_string(session).unwrap())
    } else {
        ("HTTP/1.1 404 NOT FOUND".to_string(), "Session not found".to_string())
    }
}

fn calculate_minimum_games_needed(blockchain: &Blockchain) -> f64 {
    let difficulty_info = blockchain.get_rps_difficulty_info();
    let mut min_games = 0.0;
    
    for (&required_wins, &player_count) in &difficulty_info.win_distribution {
        min_games += (required_wins as f64) * (player_count as f64);
    }
    
    min_games.max(1.0)
}

fn generate_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("miner_{}", timestamp)
}

fn format_timestamp(time: std::time::SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    let duration = time.duration_since(UNIX_EPOCH).unwrap();
    let secs = duration.as_secs();
    

    format!("{} seconds since epoch", secs)
}

fn get_index_html() -> String {
    println!("🔍 Loading index.html...");
    let html_content = include_str!("../static/index.html");
    println!("✅ Successfully loaded HTML file ({} bytes)", html_content.len());
    html_content.to_string()
}
