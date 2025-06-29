use alloy_primitives::{hex::FromHex, FixedBytes};
use dotenv::from_filename;
use shikai::Shikai;

#[tokio::main]
async fn main() {
    from_filename(".env.local").ok();

    // let block_number = 8457416;

    // let address = Address::from_hex("0xE919522e686D4e998e0434488273C7FA2ce153D8").unwrap();
    // let slot_number = 7815231;

    let shikai = Shikai::new(
        std::env::var("EXECUTION_RPC_URL").unwrap(),
        std::env::var("BEACON_RPC_URL").unwrap(),
    );

    let tx_hash = FixedBytes::<32>::from_hex(
        "0x47cea127fc2dcf17430191190d7edfb4ce971d82e8bef7a8ec866b66512e53c5",
    )
    .unwrap();

    let tx = shikai.execution().tx(tx_hash).await.unwrap();
    println!("Verified Transaction: {:?}", tx.0);

    // let account = shikai
    //     .execution()
    //     .account(address, block_number)
    //     .await
    //     .unwrap();
    // println!("Verified Account: {:?}", account);

    // let execution_header = shikai.execution().header(block_number).await.unwrap();
    // println!("Verified Execution Header: {:?}", execution_header.0);

    // let beacon_header = shikai.beacon().header(slot_number).await.unwrap();
    // println!("Verified Beacon Header: {:?}", beacon_header.0);
}
