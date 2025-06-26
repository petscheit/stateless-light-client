use shikai::Shikai;

#[tokio::main]
async fn main() {
    let block_number = 8457416;
    println!("Verifying header for block: {}", block_number);
    let shikai = Shikai::new("https://sepolia.drpc.org".to_string());

    let header = shikai.fetch_execution_header(block_number).await.unwrap();

    println!("Shikai Header: {:?}", header.0);


}