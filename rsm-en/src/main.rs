use crate::blockchain::Blockchain;
use crate::transaction::Transaction;

mod balances;
mod system;
mod merkle;
mod transaction;
mod blockchain;

fn main() {
    println!("🚀 PhlopChain - Fast Merkle Tree Blockchain Implementation");
    println!("{}", "=".repeat(60));

    // Initialize blockchain
    let mut blockchain = Blockchain::new();
    println!("✅ Blockchain initialized with genesis block");
    println!("Genesis block hash: {}", blockchain.get_latest_block().hash);

    // Display initial balances
    println!("\n💰 Initial Account Balances:");
    println!("Alice: {} tokens", blockchain.get_balance(&"alice".to_string()));
    println!("Bob: {} tokens", blockchain.get_balance(&"bob".to_string()));
    println!("Charlie: {} tokens", blockchain.get_balance(&"charlie".to_string()));

    // Create and add transactions
    println!("\n📝 Creating transactions...");
    
    let tx1 = Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        200,
        1
    );
    
    let tx2 = Transaction::new(
        "alice".to_string(),
        "charlie".to_string(),
        150,
        2
    );

    let tx3 = Transaction::new(
        "bob".to_string(),
        "charlie".to_string(),
        100,
        1
    );

    // Add transactions to the blockchain
    match blockchain.add_transaction(tx1.clone()) {
        Ok(_) => println!("✅ Transaction 1 added: Alice -> Bob (200 tokens)"),
        Err(e) => println!("❌ Transaction 1 failed: {}", e),
    }

    match blockchain.add_transaction(tx2.clone()) {
        Ok(_) => println!("✅ Transaction 2 added: Alice -> Charlie (150 tokens)"),
        Err(e) => println!("❌ Transaction 2 failed: {}", e),
    }

    match blockchain.add_transaction(tx3.clone()) {
        Ok(_) => println!("✅ Transaction 3 added: Bob -> Charlie (100 tokens)"),
        Err(e) => println!("❌ Transaction 3 failed: {}", e),
    }

    println!("\n⛏️  Mining pending transactions...");
    println!("Pending transactions: {}", blockchain.get_pending_transaction_count());

    // Mine a new block
    match blockchain.mine_pending_transactions("miner".to_string()) {
        Ok(block) => {
            println!("✅ Block mined successfully!");
            println!("Block index: {}", block.index);
            println!("Block hash: {}", block.hash);
            println!("Merkle root: {}", block.merkle_root);
            println!("Transactions in block: {}", block.transactions.len());
            println!("Mining nonce: {}", block.nonce);
        }
        Err(e) => println!("❌ Mining failed: {}", e),
    }

    // Display updated balances
    println!("\n💰 Updated Account Balances:");
    println!("Alice: {} tokens", blockchain.get_balance(&"alice".to_string()));
    println!("Bob: {} tokens", blockchain.get_balance(&"bob".to_string()));
    println!("Charlie: {} tokens", blockchain.get_balance(&"charlie".to_string()));
    println!("Miner: {} tokens", blockchain.get_balance(&"miner".to_string()));

    // Validate the blockchain
    println!("\n🔍 Blockchain Validation:");
    if blockchain.is_chain_valid() {
        println!("✅ Blockchain is valid!");
    } else {
        println!("❌ Blockchain validation failed!");
    }

    // Display blockchain statistics
    println!("\n📊 Blockchain Statistics:");
    println!("Chain length: {} blocks", blockchain.get_chain_length());
    println!("Current difficulty: {}", blockchain.difficulty);
    println!("Mining reward: {} tokens", blockchain.mining_reward);
    println!("Network hash rate: {:.2} H/s", blockchain.get_network_hash_rate());

    // Demonstrate Merkle proof functionality
    println!("\n🌳 Fast Merkle Tree Proof Demonstration:");
    if let Some((proof, tx_index, block_index)) = blockchain.get_transaction_proof(&tx1.hash) {
        println!("✅ Generated Merkle proof for transaction 1");
        println!("Transaction index in block: {}", tx_index);
        println!("Block index: {}", block_index);
        println!("Proof length: {} hashes", proof.len());

        // Verify the proof
        let is_valid = blockchain.verify_transaction_proof(&tx1, &proof, tx_index, block_index);
        if is_valid {
            println!("✅ Merkle proof verification successful!");
        } else {
            println!("❌ Merkle proof verification failed!");
        }
    }

    // Display transaction history
    println!("\n📋 Transaction History for Alice:");
    let alice_history = blockchain.get_transaction_history(&"alice".to_string());
    for (i, tx) in alice_history.iter().enumerate() {
        println!("{}. {} -> {} ({} tokens) [{}]", 
                 i + 1, tx.from, tx.to, tx.amount, tx.hash.to_hex()[..8].to_string());
    }

    // Display state root
    if let Some(state_root) = blockchain.get_state_root() {
        println!("\n🗂️  Current State Root: {}", state_root);
    }

    // Test invalid transaction
    println!("\n🚫 Testing invalid transaction (insufficient funds):");
    let invalid_tx = Transaction::new(
        "charlie".to_string(),
        "alice".to_string(),
        10000, // More than Charlie has
        1
    );

    match blockchain.add_transaction(invalid_tx) {
        Ok(_) => println!("❌ Invalid transaction was accepted (this shouldn't happen)"),
        Err(e) => println!("✅ Invalid transaction rejected: {}", e),
    }

    // Add more transactions and mine another block
    println!("\n⛏️  Mining another block...");
    let tx4 = Transaction::new(
        "bob".to_string(),
        "alice".to_string(),
        50,
        2
    );

    if blockchain.add_transaction(tx4).is_ok() {
        match blockchain.mine_pending_transactions("miner2".to_string()) {
            Ok(block) => {
                println!("✅ Second block mined!");
                println!("Block hash: {}", block.hash);
            }
            Err(e) => println!("❌ Second block mining failed: {}", e),
        }
    }

    // Final blockchain state
    println!("\n🏁 Final Blockchain State:");
    println!("Total blocks: {}", blockchain.get_chain_length());
    println!("Blockchain valid: {}", blockchain.is_chain_valid());
    
    // Display all blocks
    for (i, block) in blockchain.chain.iter().enumerate() {
        println!("\nBlock {}: {}", i, block.hash.to_hex()[..16].to_string());
        println!("  Transactions: {}", block.transactions.len());
        println!("  Timestamp: {}", block.timestamp);
        if i > 0 {
            println!("  Previous: {}", block.previous_hash.to_hex()[..16].to_string());
        }
    }

    println!("\n🎉 PhlopChain demonstration completed successfully!");
}
