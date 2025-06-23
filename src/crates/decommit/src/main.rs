use std::path::PathBuf;
use decommit::{VerifiedHeader, TrustlessHeader};

#[tokio::main]
async fn main() {
    let proof_path = PathBuf::from("proof_bankai.json");
    println!("Reading Recursive Proof...");
    let verified_header = VerifiedHeader::from_proof(proof_path).await.unwrap();
    println!("Verified header for block: {}", verified_header.number());
    println!("Block hash: {:?}", verified_header.parent_hash());

    println!("State Root: {:?}", verified_header.state_root());
    println!("Transactions Root: {:?}", verified_header.transactions_root());
    println!("Receipts Root: {:?}", verified_header.receipts_root());
    println!("Logs Bloom: {:?}", verified_header.logs_bloom());
    println!("Difficulty: {:?}", verified_header.difficulty());
    println!("Number: {:?}", verified_header.number());
    println!("Gas Limit: {:?}", verified_header.gas_limit());
    println!("Gas Used: {:?}", verified_header.gas_used());
}