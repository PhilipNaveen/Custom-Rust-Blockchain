use crate::blockchain::Blockchain;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerSession {
    pub id: String,
    pub name: String,
    pub total_phlopcoin: f64,
    pub blocks_mined: u32,
    pub mining_history: Vec<MiningResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningResult {
    pub block_number: u32,
    pub phlopcoin_earned: f64,
    pub games_played: u64,
    pub rounds: u32,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct StartMiningRequest {
    pub miner_name: String,
}

#[derive(Debug, Deserialize)]
pub struct MineBlockRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct MiningResponse {
    pub success: bool,
    pub message: String,
    pub session: Option<MinerSession>,
    pub mining_result: Option<MiningResult>,
}

#[derive(Debug, Serialize)]
pub struct BlockchainStatus {
    pub total_blocks: usize,
    pub total_games_played: u64,
    pub current_difficulty_score: f64,
    pub active_miners: usize,
}

type SharedBlockchain = Arc<Mutex<Blockchain>>;
type SharedSessions = Arc<Mutex<HashMap<String, MinerSession>>>;

pub struct WebServer {
    blockchain: SharedBlockchain,
    sessions: SharedSessions,
}

impl WebServer {
    pub fn new() -> Self {
        let blockchain = Arc::new(Mutex::new(Blockchain::new()));
        let sessions = Arc::new(Mutex::new(HashMap::new()));
        
        Self {
            blockchain,
            sessions,
        }
    }

    pub async fn start_server(self) {
        let blockchain = self.blockchain.clone();
        let sessions = self.sessions.clone();
        let static_files = warp::path("static")
            .and(warp::fs::dir("static"));
        let index = warp::path::end()
            .map(|| {
                warp::reply::html(include_str!("../static/index.html"))
            });
        let api = warp::path("api");
        let start_mining = api
            .and(warp::path("start"))
            .and(warp::post())
            .and(warp::body::json())
            .and(with_sessions(sessions.clone()))
            .and_then(start_mining_handler);
        let mine_block = api
            .and(warp::path("mine"))
            .and(warp::post())
            .and(warp::body::json())
            .and(with_blockchain(blockchain.clone()))
            .and(with_sessions(sessions.clone()))
            .and_then(mine_block_handler);
        let get_status = api
            .and(warp::path("status"))
            .and(warp::path::param::<String>())
            .and(warp::get())
            .and(with_sessions(sessions.clone()))
            .and_then(get_status_handler);
        let blockchain_status = api
            .and(warp::path("blockchain"))
            .and(warp::get())
            .and(with_blockchain(blockchain.clone()))
            .and(with_sessions(sessions.clone()))
            .and_then(blockchain_status_handler);

        let routes = index
            .or(static_files)
            .or(start_mining)
            .or(mine_block)
            .or(get_status)
            .or(blockchain_status)
            .with(warp::cors().allow_any_origin().allow_headers(vec!["content-type"]).allow_methods(vec!["GET", "POST"]));

        println!("🌐 PhlopChain Web Interface starting on http://localhost:3030");
        warp::serve(routes)
            .run(([127, 0, 0, 1], 3030))
            .await;
    }
}

fn with_blockchain(blockchain: SharedBlockchain) -> impl Filter<Extract = (SharedBlockchain,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || blockchain.clone())
}

fn with_sessions(sessions: SharedSessions) -> impl Filter<Extract = (SharedSessions,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || sessions.clone())
}

async fn start_mining_handler(
    request: StartMiningRequest,
    sessions: SharedSessions,
) -> Result<impl warp::Reply, warp::Rejection> {
    let session_id = Uuid::new_v4().to_string();
    let session = MinerSession {
        id: session_id.clone(),
        name: request.miner_name,
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

    Ok(warp::reply::json(&response))
}

async fn mine_block_handler(
    request: MineBlockRequest,
    blockchain: SharedBlockchain,
    sessions: SharedSessions,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut sessions_guard = sessions.lock().unwrap();
    let session = sessions_guard.get_mut(&request.session_id);

    if session.is_none() {
        let response = MiningResponse {
            success: false,
            message: "Invalid session ID".to_string(),
            session: None,
            mining_result: None,
        };
        return Ok(warp::reply::json(&response));
    }

    let session = session.unwrap();
    

    let tx = Transaction::new(
        "network".to_string(),
        session.name.clone(),
        10,
        session.blocks_mined + 1,
    );

    let mut blockchain_guard = blockchain.lock().unwrap();
    

    if let Err(e) = blockchain_guard.add_transaction(tx) {
        println!("Failed to add transaction: {}", e);

    }
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
                    timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                };

                session.total_phlopcoin += phlopcoin_earned;
                session.blocks_mined += 1;
                session.mining_history.push(mining_result.clone());

                drop(blockchain_guard);

                let response = MiningResponse {
                    success: true,
                    message: format!("Block #{} mined successfully! Earned {:.6} PhlopCoin", block.index, phlopcoin_earned),
                    session: Some(session.clone()),
                    mining_result: Some(mining_result),
                };

                Ok(warp::reply::json(&response))
            } else {
                let response = MiningResponse {
                    success: false,
                    message: "Block mined but no RPS result found".to_string(),
                    session: Some(session.clone()),
                    mining_result: None,
                };
                Ok(warp::reply::json(&response))
            }
        }
        Err(e) => {
            let response = MiningResponse {
                success: false,
                message: format!("Mining failed: {}", e),
                session: Some(session.clone()),
                mining_result: None,
            };
            Ok(warp::reply::json(&response))
        }
    }
}

async fn get_status_handler(
    session_id: String,
    sessions: SharedSessions,
) -> Result<impl warp::Reply, warp::Rejection> {
    let sessions_guard = sessions.lock().unwrap();
    if let Some(session) = sessions_guard.get(&session_id) {
        Ok(warp::reply::json(session))
    } else {
        Ok(warp::reply::with_status(
            "Session not found",
            warp::http::StatusCode::NOT_FOUND,
        ).into_response())
    }
}

async fn blockchain_status_handler(
    blockchain: SharedBlockchain,
    sessions: SharedSessions,
) -> Result<impl warp::Reply, warp::Rejection> {
    let blockchain_guard = blockchain.lock().unwrap();
    let sessions_guard = sessions.lock().unwrap();

    let status = BlockchainStatus {
        total_blocks: blockchain_guard.get_chain_length(),
        total_games_played: blockchain_guard.get_total_rps_games(),
        current_difficulty_score: blockchain_guard.get_rps_difficulty_info().difficulty_score(),
        active_miners: sessions_guard.len(),
    };

    Ok(warp::reply::json(&status))
}

fn calculate_minimum_games_needed(blockchain: &Blockchain) -> f64 {
    let difficulty_info = blockchain.get_rps_difficulty_info();
    let mut min_games = 0.0;
    

    for (&required_wins, &player_count) in &difficulty_info.win_distribution {
        min_games += (required_wins as f64) * (player_count as f64);
    }
    
    min_games.max(1.0)
}
