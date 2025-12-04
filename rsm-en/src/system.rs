use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pallet {
    block_number: u32,
    nonce: BTreeMap<String, u32>
}

impl Pallet{

    pub fn new() -> Self {

        Self {

            block_number: 0,
            nonce: BTreeMap::new()

        }
    } 

    #[allow(dead_code)]
    pub fn get_block_number(&self) -> u32 {

        self.block_number
    }

    pub fn inc_block_number(&mut self, _who: &String){

        self.block_number = self.block_number.checked_add(1).unwrap();
    }

    pub fn inc_nonce(&mut self, who: &String){

        let nonce: &u32 = self.nonce.get(who).unwrap_or(&0);
        self.nonce.insert(who.clone(), nonce + 1);
    }

    pub fn get_nonce(&self, who: &String) -> u32 {

        *self.nonce.get(who).unwrap_or(&0)
    }

}