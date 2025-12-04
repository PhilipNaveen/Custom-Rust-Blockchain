use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pallet {
    pub balances: BTreeMap<String, u128>
}

impl Pallet {

    pub fn new() -> Self {
        
        Self {

            balances: BTreeMap::new()
        }
    }

    pub fn set_balance(&mut self, who: &String, amount: u128){

        self.balances.insert(who.clone(), amount);
    }

    pub fn get_balance(&mut self, who: &String) -> u128{
        
        *self.balances.get(who).unwrap_or(&0)
    }

    pub fn transfer(&mut self, sender: String, reciever: String, amount: u128) -> Result<(), &'static str>{
        
        let sender_balance: u128 = self.get_balance(&sender);
        let reciever_balance: u128 = self.get_balance(&reciever);

        let new_sender_balance: u128 = sender_balance.checked_sub(amount).ok_or("Insufficient sender balance")?;
        let new_reciever_balance: u128 = reciever_balance.checked_add(amount).ok_or("Error adding balance")?;

        self.set_balance(&sender, new_sender_balance);
        self.set_balance(&reciever, new_reciever_balance);

        Ok(())
    }
}