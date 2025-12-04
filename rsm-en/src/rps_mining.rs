use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Move {
    Rock,
    Paper,
    Scissors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameResult {
    PlayerWin,
    BlockchainWin,
    Tie,
}

impl Move {
    pub fn from_seed(seed: u64) -> Self {
        match seed % 3 {
            0 => Move::Rock,
            1 => Move::Paper,
            _ => Move::Scissors,
        }
    }

    pub fn beats(&self, other: &Move) -> GameResult {
        match (self, other) {
            (Move::Rock, Move::Scissors) => GameResult::PlayerWin,
            (Move::Paper, Move::Rock) => GameResult::PlayerWin,
            (Move::Scissors, Move::Paper) => GameResult::PlayerWin,
            (Move::Scissors, Move::Rock) => GameResult::BlockchainWin,
            (Move::Rock, Move::Paper) => GameResult::BlockchainWin,
            (Move::Paper, Move::Scissors) => GameResult::BlockchainWin,
            _ => GameResult::Tie,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub required_wins: u32,
    pub current_wins: u32,
    pub games_played: u32,
    pub seed: u64,
}

impl Player {
    pub fn new(id: u32, required_wins: u32, base_seed: u64) -> Self {

        let mut hasher = Sha256::new();
        hasher.update(base_seed.to_be_bytes());
        hasher.update(id.to_be_bytes());
        let hash = hasher.finalize();
        let seed = u64::from_be_bytes([
            hash[0], hash[1], hash[2], hash[3],
            hash[4], hash[5], hash[6], hash[7],
        ]);

        Self {
            id,
            required_wins,
            current_wins: 0,
            games_played: 0,
            seed,
        }
    }

    pub fn play_game(&mut self, blockchain_move: Move) -> GameResult {

        let game_seed = self.seed.wrapping_add(self.games_played as u64);
        let player_move = Move::from_seed(game_seed);
        
        self.games_played += 1;
        
        let result = player_move.beats(&blockchain_move);
        if result == GameResult::PlayerWin {
            self.current_wins += 1;
        }
        
        result
    }

    pub fn has_won(&self) -> bool {
        self.current_wins >= self.required_wins
    }

    pub fn reset(&mut self) {
        self.current_wins = 0;
        self.games_played = 0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RPSMiningConfig {
    pub total_players: u32,
    pub blocks_mined: u32,
}

impl RPSMiningConfig {
    pub fn new() -> Self {
        Self {
            total_players: 100,
            blocks_mined: 0,
        }
    }

    pub fn get_win_requirements(&self) -> Vec<u32> {
        let mut requirements = Vec::new();
        let blocks = self.blocks_mined;
        
        if blocks == 0 {

            requirements.resize(100, 1);
        } else {

            let players_with_extra_wins = std::cmp::min(blocks, 100);
            let players_with_one_win = 100 - players_with_extra_wins;
            

            for _ in 0..players_with_one_win {
                requirements.push(1);
            }
            

            for i in 0..players_with_extra_wins {
                requirements.push(2 + (i / 100));
            }
        }
        
        requirements
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RPSMiner {
    pub config: RPSMiningConfig,
    pub players: Vec<Player>,
    pub blockchain_seed: u64,
    pub games_played: u64,
}

impl RPSMiner {
    pub fn new(config: RPSMiningConfig) -> Self {
        let blockchain_seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let win_requirements = config.get_win_requirements();
        let mut players = Vec::new();
        
        for (i, &required_wins) in win_requirements.iter().enumerate() {
            players.push(Player::new(i as u32, required_wins, blockchain_seed));
        }

        Self {
            config,
            players,
            blockchain_seed,
            games_played: 0,
        }
    }

    pub fn mine_block(&mut self, block_data: &str) -> Result<RPSMiningResult, String> {

        let mut hasher = Sha256::new();
        hasher.update(block_data.as_bytes());
        hasher.update(self.blockchain_seed.to_be_bytes());
        hasher.update(self.games_played.to_be_bytes());
        let hash = hasher.finalize();
        let block_seed = u64::from_be_bytes([
            hash[0], hash[1], hash[2], hash[3],
            hash[4], hash[5], hash[6], hash[7],
        ]);

        let mut round = 0;
        let mut total_games = 0;
        let start_time = SystemTime::now();

        loop {
            round += 1;
            let mut all_players_won = true;
            let mut round_games = 0;
            for player in &mut self.players {
                if !player.has_won() {

                    let blockchain_move_seed = block_seed
                        .wrapping_add(round as u64)
                        .wrapping_add(player.id as u64);
                    let blockchain_move = Move::from_seed(blockchain_move_seed);
                    let mut player_won_round = false;
                    while !player_won_round {
                        let result = player.play_game(blockchain_move);
                        round_games += 1;
                        
                        if result == GameResult::PlayerWin {
                            player_won_round = true;
                        }

                    }

                    if !player.has_won() {
                        all_players_won = false;
                    }
                }
            }

            total_games += round_games;
            self.games_played += round_games as u64;

            if all_players_won {
                let mining_time = SystemTime::now()
                    .duration_since(start_time)
                    .unwrap()
                    .as_millis();

                let result = RPSMiningResult {
                    success: true,
                    rounds: round,
                    total_games,
                    mining_time_ms: mining_time,
                    winning_players: self.players.clone(),
                    final_seed: block_seed,
                };
                for player in &mut self.players {
                    player.reset();
                }
                self.config.blocks_mined += 1;
                

                let new_requirements = self.config.get_win_requirements();
                for (i, &required_wins) in new_requirements.iter().enumerate() {
                    if let Some(player) = self.players.get_mut(i) {
                        player.required_wins = required_wins;
                    }
                }

                return Ok(result);
            }
            if round > 1000000 {
                return Err("Mining timeout: too many rounds".to_string());
            }
        }
    }

    pub fn get_difficulty_info(&self) -> DifficultyInfo {
        let requirements = self.config.get_win_requirements();
        let total_required_wins: u32 = requirements.iter().sum();
        
        let mut win_distribution = HashMap::new();
        for &wins in &requirements {
            *win_distribution.entry(wins).or_insert(0) += 1;
        }

        DifficultyInfo {
            block_number: self.config.blocks_mined,
            total_required_wins,
            win_distribution,
            total_players: self.config.total_players,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RPSMiningResult {
    pub success: bool,
    pub rounds: u32,
    pub total_games: u64,
    pub mining_time_ms: u128,
    pub winning_players: Vec<Player>,
    pub final_seed: u64,
}

#[derive(Debug, Clone)]
pub struct DifficultyInfo {
    #[allow(dead_code)]
    pub block_number: u32,
    #[allow(dead_code)]
    pub total_required_wins: u32,
    pub win_distribution: HashMap<u32, u32>,
    pub total_players: u32,
}

impl DifficultyInfo {
    pub fn difficulty_score(&self) -> f64 {

        let mut score = 0.0;
        for (&wins, &count) in &self.win_distribution {
            score += (wins as f64).powi(2) * count as f64;
        }
        score / self.total_players as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_generation() {
        let move1 = Move::from_seed(0);
        let move2 = Move::from_seed(1);
        let move3 = Move::from_seed(2);
        
        assert_eq!(move1, Move::Rock);
        assert_eq!(move2, Move::Paper);
        assert_eq!(move3, Move::Scissors);
    }

    #[test]
    fn test_game_results() {
        assert_eq!(Move::Rock.beats(&Move::Scissors), GameResult::PlayerWin);
        assert_eq!(Move::Paper.beats(&Move::Rock), GameResult::PlayerWin);
        assert_eq!(Move::Scissors.beats(&Move::Paper), GameResult::PlayerWin);
        assert_eq!(Move::Rock.beats(&Move::Rock), GameResult::Tie);
    }

    #[test]
    fn test_player_creation() {
        let player = Player::new(0, 2, 12345);
        assert_eq!(player.id, 0);
        assert_eq!(player.required_wins, 2);
        assert_eq!(player.current_wins, 0);
    }

    #[test]
    fn test_mining_config() {
        let config = RPSMiningConfig::new();
        let requirements = config.get_win_requirements();
        assert_eq!(requirements.len(), 100);
        assert!(requirements.iter().all(|&x| x == 1));
    }

    #[test]
    fn test_difficulty_progression() {
        let mut config = RPSMiningConfig::new();
        

        let req1 = config.get_win_requirements();
        assert!(req1.iter().all(|&x| x == 1));
        

        config.blocks_mined = 1;
        let req2 = config.get_win_requirements();
        assert_eq!(req2.iter().filter(|&&x| x == 1).count(), 99);
        assert_eq!(req2.iter().filter(|&&x| x == 2).count(), 1);
        

        config.blocks_mined = 2;
        let req3 = config.get_win_requirements();
        assert_eq!(req3.iter().filter(|&&x| x == 1).count(), 98);
        assert_eq!(req3.iter().filter(|&&x| x == 2).count(), 2);
    }

    #[test]
    fn test_miner_creation() {
        let config = RPSMiningConfig::new();
        let miner = RPSMiner::new(config);
        assert_eq!(miner.players.len(), 100);
        assert!(miner.players.iter().all(|p| p.required_wins == 1));
    }
}
