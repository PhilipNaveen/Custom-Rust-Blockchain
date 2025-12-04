use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    #[allow(dead_code)]
    pub fn new(data: [u8; 32]) -> Self {
        Self(data)
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Self(hash)
    }

    pub fn from_string(data: &str) -> Self {
        Self::from_bytes(data.as_bytes())
    }

    pub fn combine(&self, other: &Hash) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.0);
        hasher.update(&other.0);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Hash(hash)
    }

    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastMerkleTree {
    leaves: Vec<Hash>,
    nodes: Vec<Vec<Hash>>,
    root: Option<Hash>,
}

impl FastMerkleTree {
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            nodes: Vec::new(),
            root: None,
        }
    }

    #[allow(dead_code)]
    pub fn from_data(data: Vec<String>) -> Self {
        let mut tree = Self::new();
        for item in data {
            tree.add_leaf(Hash::from_string(&item));
        }
        tree.build();
        tree
    }

    pub fn add_leaf(&mut self, leaf: Hash) {
        self.leaves.push(leaf);
        self.root = None;
    }

    pub fn build(&mut self) {
        if self.leaves.is_empty() {
            self.root = None;
            return;
        }

        self.nodes.clear();
        let mut current_level = self.leaves.clone();
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            

            for chunk in current_level.chunks(2) {
                let combined = if chunk.len() == 2 {
                    chunk[0].combine(&chunk[1])
                } else {

                    chunk[0].combine(&chunk[0])
                };
                next_level.push(combined);
            }
            
            self.nodes.push(current_level);
            current_level = next_level;
        }

        if !current_level.is_empty() {
            self.root = Some(current_level[0].clone());
            self.nodes.push(current_level);
        }
    }

    pub fn get_root(&self) -> Option<&Hash> {
        self.root.as_ref()
    }

    #[allow(dead_code)]
    pub fn get_proof(&self, index: usize) -> Option<Vec<Hash>> {
        if index >= self.leaves.len() || self.nodes.is_empty() {
            return None;
        }

        let mut proof = Vec::new();
        let mut current_index = index;
        for level in &self.nodes {
            if current_index >= level.len() {
                break;
            }

            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            if sibling_index < level.len() {
                proof.push(level[sibling_index].clone());
            } else if current_index < level.len() {

                proof.push(level[current_index].clone());
            }

            current_index /= 2;
        }

        Some(proof)
    }

    #[allow(dead_code)]
    pub fn verify_proof(&self, leaf: &Hash, proof: &[Hash], index: usize) -> bool {
        if let Some(root) = &self.root {
            let calculated_root = self.calculate_root_from_proof(leaf, proof, index);
            calculated_root == *root
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn calculate_root_from_proof(&self, leaf: &Hash, proof: &[Hash], mut index: usize) -> Hash {
        let mut current_hash = leaf.clone();

        for proof_hash in proof {
            current_hash = if index % 2 == 0 {
                current_hash.combine(proof_hash)
            } else {
                proof_hash.combine(&current_hash)
            };
            index /= 2;
        }

        current_hash
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_creation() {
        let hash1 = Hash::from_string("test1");
        let hash2 = Hash::from_string("test2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_merkle_tree_single_leaf() {
        let mut tree = FastMerkleTree::new();
        tree.add_leaf(Hash::from_string("single"));
        tree.build();
        assert!(tree.get_root().is_some());
    }

    #[test]
    fn test_merkle_tree_multiple_leaves() {
        let data = vec!["leaf1".to_string(), "leaf2".to_string(), "leaf3".to_string(), "leaf4".to_string()];
        let tree = FastMerkleTree::from_data(data);
        assert!(tree.get_root().is_some());
        assert_eq!(tree.len(), 4);
    }

    #[test]
    fn test_merkle_proof() {
        let data = vec!["leaf1".to_string(), "leaf2".to_string(), "leaf3".to_string(), "leaf4".to_string()];
        let tree = FastMerkleTree::from_data(data);
        
        let leaf = Hash::from_string("leaf1");
        let proof = tree.get_proof(0).unwrap();
        assert!(tree.verify_proof(&leaf, &proof, 0));
    }
}
