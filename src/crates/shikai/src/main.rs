use dotenv::from_filename;
use shikai::Shikai;

#[tokio::main]
async fn main() {
    from_filename(".env.local").ok();

    let block_number = 8457416;
    let slot_number = 7815231;

    let shikai = Shikai::new(
        std::env::var("EXECUTION_RPC_URL").unwrap(),
        std::env::var("BEACON_RPC_URL").unwrap(),
    );

    let execution_header = shikai.fetch_execution_header(block_number).await.unwrap();
    
    let beacon_header = shikai.fetch_beacon_header(slot_number).await.unwrap();
}
