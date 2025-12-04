use crate::merkle::Hash;
use crate::rps_mining::{RPSMiningResult};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u128,
    pub nonce: u32,
    pub timestamp: u64,
    pub hash: Hash,
}

impl Transaction {
    pub fn new(from: String, to: String, amount: u128, nonce: u32) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut tx = Self {
            from,
            to,
            amount,
            nonce,
            timestamp,
            hash: Hash::from_string(""),
        };
        

        tx.hash = tx.calculate_hash();
        tx
    }

    pub fn calculate_hash(&self) -> Hash {
        let data = format!(
            "{}{}{}{}{}",
            self.from, self.to, self.amount, self.nonce, self.timestamp
        );
        Hash::from_string(&data)
    }

    pub fn is_valid(&self) -> bool {
        self.hash == self.calculate_hash() && 
        !self.from.is_empty() && 
        !self.to.is_empty() &&
        self.from != self.to
    }

    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u32,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub previous_hash: Hash,
    pub merkle_root: Hash,
    pub hash: Hash,
    pub rps_mining_result: Option<RPSMiningResult>,
}

impl Block {
    pub fn new(index: u32, transactions: Vec<Transaction>, previous_hash: Hash) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let merkle_root = Self::calculate_merkle_root(&transactions);
        
        let mut block = Self {
            index,
            timestamp,
            transactions,
            previous_hash,
            merkle_root,
            hash: Hash::from_string(""),
            rps_mining_result: None,
        };
        
        block.hash = block.calculate_hash();
        block
    }

    pub fn genesis() -> Self {
        let genesis_hash = Hash::from_string("genesis");
        Self::new(0, Vec::new(), genesis_hash)
    }

    pub fn calculate_hash(&self) -> Hash {
        let rps_data = if let Some(ref result) = self.rps_mining_result {
            format!("{}:{}", result.rounds, result.total_games)
        } else {
            "pending".to_string()
        };
        
        let data = format!(
            "{}{}{}{}{}",
            self.index,
            self.timestamp,
            self.previous_hash.to_hex(),
            self.merkle_root.to_hex(),
            rps_data
        );
        Hash::from_string(&data)
    }

    fn calculate_merkle_root(transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash::from_string("empty");
        }

        let mut tree = crate::merkle::FastMerkleTree::new();
        for tx in transactions {
            tree.add_leaf(tx.hash.clone());
        }
        tree.build();
        
        tree.get_root().cloned().unwrap_or_else(|| Hash::from_string("empty"))
    }

    pub fn mine_block_rps(&mut self, rps_miner: &mut crate::rps_mining::RPSMiner) -> Result<(), String> {

        let block_data = format!(
            "{}{}{}{}",
            self.index,
            self.timestamp,
            self.previous_hash.to_hex(),
            self.merkle_root.to_hex()
        );
        

        match rps_miner.mine_block(&block_data) {
            Ok(mining_result) => {
                self.rps_mining_result = Some(mining_result.clone());
                self.hash = self.calculate_hash();
                
                println!("Block mined with RPS: {} (Rounds: {}, Games: {})", 
                         self.hash, mining_result.rounds, mining_result.total_games);
                Ok(())
            }
            Err(e) => Err(e)
        }
    }

    #[allow(dead_code)]
    pub fn mine_block(&mut self, difficulty: usize) {

        let target = "0".repeat(difficulty);
        let mut nonce = 0u64;
        
        loop {
            let data = format!(
                "{}{}{}{}{}",
                self.index,
                self.timestamp,
                self.previous_hash.to_hex(),
                self.merkle_root.to_hex(),
                nonce
            );
            self.hash = Hash::from_string(&data);
            
            if self.hash.to_hex().starts_with(&target) {
                break;
            }
            nonce += 1;
        }
        
        println!("Block mined: {} with nonce: {}", self.hash, nonce);
    }

    #[allow(dead_code)]
    pub fn is_valid(&self, previous_block: Option<&Block>) -> bool {

        if self.hash != self.calculate_hash() {
            return false;
        }
        if self.merkle_root != Self::calculate_merkle_root(&self.transactions) {
            return false;
        }
        if let Some(prev_block) = previous_block {
            if self.previous_hash != prev_block.hash {
                return false;
            }
            if self.index != prev_block.index + 1 {
                return false;
            }
        } else if self.index != 0 {
            return false;
        }
        for tx in &self.transactions {
            if !tx.is_valid() {
                return false;
            }
        }

        true
    }

    #[allow(dead_code)]
    pub fn get_transaction_proof(&self, tx_index: usize) -> Option<Vec<Hash>> {
        if tx_index >= self.transactions.len() {
            return None;
        }

        let mut tree = crate::merkle::FastMerkleTree::new();
        for tx in &self.transactions {
            tree.add_leaf(tx.hash.clone());
        }
        tree.build();
        
        tree.get_proof(tx_index)
    }

    #[allow(dead_code)]
    pub fn verify_transaction_inclusion(&self, tx: &Transaction, proof: &[Hash], tx_index: usize) -> bool {
        let mut tree = crate::merkle::FastMerkleTree::new();
        for transaction in &self.transactions {
            tree.add_leaf(transaction.hash.clone());
        }
        tree.build();
        
        tree.verify_proof(&tx.hash, proof, tx_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            1
        );
        assert!(tx.is_valid());
    }

    #[test]
    fn test_block_creation() {
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            1
        );
        let previous_hash = Hash::from_string("previous");
        let block = Block::new(1, vec![tx], previous_hash);
        assert_eq!(block.index, 1);
        assert_eq!(block.transactions.len(), 1);
    }

    #[test]
    fn test_genesis_block() {
        let genesis = Block::genesis();
        assert_eq!(genesis.index, 0);
        assert!(genesis.transactions.is_empty());
    }

    #[test]
    fn test_block_validation() {
        let genesis = Block::genesis();
        assert!(genesis.is_valid(None));

        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            1
        );
        let block = Block::new(1, vec![tx], genesis.hash.clone());
        assert!(block.is_valid(Some(&genesis)));
    }
}
