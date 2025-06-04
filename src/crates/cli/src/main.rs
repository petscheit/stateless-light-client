use bankai_core::{db::Status, fetcher::recursive_epoch_input::{RecursiveEpochInputs, RecursiveEpochUpdate}, utils::{constants::{GENESIS_EPOCH, SLOTS_PER_EPOCH}, hashing::get_committee_hash}, BankaiClient};
use clap::{Parser, Subcommand};
use dotenv::from_filename;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Subcommand)]
enum Commands {
    /// Generate and manage proofs for the light client state
    #[command(subcommand)]
    Prove(ProveCommands),

    /// Fetch proof data from the network
    #[command(subcommand)]
    Fetch(FetchCommands),
}

#[derive(Subcommand)]
enum FetchCommands {
    Genesis,
    /// Fetch a sync committee update proof for a given slot
    RecursiveEpoch {
        /// Export output to a JSON file
        #[arg(long, short)]
        export: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProveCommands {
    Genesis,
    RecursiveEpoch {
        #[arg(long, short)]
        export: Option<String>,
    },
}


#[derive(Parser)]
#[command(
    author,
    version,
    about = "Bankai CLI - Recursive Epoch Update for Ethereum",
    long_about = "A command-line interface for managing the Bankai Recursive Epoch Update for Starknet. \
                  This tool helps generate, verify, and manage proofs for recursive epoch updates."
)]
struct Cli {
    /// Optional RPC URL (defaults to RPC_URL_BEACON environment variable)
    #[arg(long, short)]
    rpc_url: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<(), BankaiCliError> {
    // Load .env.sepolia file
    from_filename(".env.local").ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cli = Cli::parse();
    let bankai = BankaiClient::new(false).await;

    match cli.command {
        Commands::Fetch(cmd) => match cmd {
            FetchCommands::Genesis => {
                let proof: RecursiveEpochUpdate = RecursiveEpochInputs::new(&bankai.client, &bankai.db)
                    .await
                    .unwrap()
                    .into();
                let committee_hash = get_committee_hash(proof.inputs.epoch_update.aggregate_pub.0);
                println!("Genesis committee hash: {}", committee_hash);
            }
            FetchCommands::RecursiveEpoch { export } => {
                let proof: RecursiveEpochUpdate = RecursiveEpochInputs::new(&bankai.client, &bankai.db)
                    .await
                    .unwrap()
                    .into();

                let json = serde_json::to_string_pretty(&proof)?;
                let _pie = cairo_runner::run("cairo/build/recursive_update.json", proof.into()).unwrap();

                if let Some(path) = export {
                    match std::fs::write(path.clone(), json) {
                        Ok(_) => println!("Proof exported to {}", path),
                        Err(e) => return Err(BankaiCliError::IoError(e)),
                    }
                } else {
                    // println!("{}", json);
                }
            }
        },
        Commands::Prove(cmd) => match cmd {
            ProveCommands::Genesis => {
                if let Some(_) = bankai.db.get_latest_epoch_update().await.unwrap() {
                    panic!("Genesis proof already exists");
                }
                let proof: RecursiveEpochUpdate = RecursiveEpochInputs::new(&bankai.client, &bankai.db)
                    .await
                    .unwrap()
                    .into();

                let epoch = proof.inputs.epoch_update.header.slot / SLOTS_PER_EPOCH;
                let slot = proof.inputs.epoch_update.header.slot;
                let uuid = bankai.db.create_epoch_update(epoch.clone(), slot, proof.outputs.clone()).await.unwrap();

                // Wrap the proof generation and submission in error handling
                let result = async {
                    bankai.db.update_status(&uuid, Status::TraceGen).await?;
                    let pie = cairo_runner::run("cairo/build/recursive_update.json", proof.into())
                        .map_err(|e| format!("Cairo runner failed: {}", e))?;

                    let altantic_id = bankai.atlantic_client.submit_stone(pie, format!("epoch_{}", epoch)).await
                        .map_err(|e| format!("Atlantic submission failed: {}", e))?;
                    bankai.db.add_atlantic_id(&uuid, &altantic_id).await?;
                    bankai.db.update_status(&uuid, Status::Proving).await?;

                    println!("Proof submitted to Atlantic: {}", altantic_id);
                    Ok::<(), Box<dyn std::error::Error>>(())
                }.await;

                if let Err(e) = result {
                    let error_msg = format!("Genesis proof generation failed: {}", e);
                    if let Err(db_err) = bankai.db.update_error(&uuid, &error_msg).await {
                        eprintln!("Failed to update error status in database: {}", db_err);
                    }
                    return Err(BankaiCliError::ProofGenerationError(error_msg));
                }
            }
            ProveCommands::RecursiveEpoch { export } => {
                let prev_epoch = match bankai.db.get_latest_epoch_update().await.unwrap() {
                    Some(epoch_update) => epoch_update,
                    None => panic!("No previous epoch update found. Pls run genesis first"),
                };

                let atlantic_id = prev_epoch.atlantic_id.as_ref().unwrap();
                let status = bankai.atlantic_client.check_batch_status(atlantic_id).await.unwrap();
                match status.as_str() {
                    "FAILED" => {
                        bankai.db.update_error(&prev_epoch.uuid, "Proving failed").await.unwrap();
                        panic!("Proving failed. Pls try again: {}", atlantic_id);
                    }
                    "DONE" => {
                        let proof = bankai.atlantic_client.fetch_proof(atlantic_id).await.unwrap();
                        let proof_id = bankai.db.add_proof(&proof.proof.to_string()).await.unwrap();
                        bankai.db.update_proof_id(&prev_epoch.uuid, proof_id).await.unwrap();
                        bankai.db.update_status(&prev_epoch.uuid, Status::Done).await.unwrap();
                        println!("Proof fetched from Atlantic: {}", atlantic_id);
                    }
                    _ => {
                        panic!("Proof not done yet. Pls try again soon: {}", atlantic_id);
                    }
                }

                let proof: RecursiveEpochUpdate = RecursiveEpochInputs::new(&bankai.client, &bankai.db)
                    .await
                    .unwrap()
                    .into();
                let epoch = proof.inputs.epoch_update.header.slot / SLOTS_PER_EPOCH;
                println!("Epoch: {}", epoch);
                let slot = proof.inputs.epoch_update.header.slot;
                println!("Slot: {}", slot);
                let uuid = bankai.db.create_epoch_update(epoch.clone(), slot, proof.outputs.clone()).await.unwrap();

                // Wrap the proof generation and submission in error handling
                let result = async {
                    bankai.db.update_status(&uuid, Status::TraceGen).await?;
                    let pie = cairo_runner::run("cairo/build/recursive_update.json", proof.into())
                        .map_err(|e| format!("Cairo runner failed: {}", e))?;

                    let altantic_id = bankai.atlantic_client.submit_stone(pie, format!("epoch_{}", epoch)).await
                        .map_err(|e| format!("Atlantic submission failed: {}", e))?;
                    bankai.db.add_atlantic_id(&uuid, &altantic_id).await?;
                    bankai.db.update_status(&uuid, Status::Proving).await?;

                    println!("Proof submitted to Atlantic: {}", altantic_id);
                    Ok::<(), Box<dyn std::error::Error>>(())
                }.await;

                if let Err(e) = result {
                    let error_msg = format!("Recursive epoch proof generation failed: {}", e);
                    if let Err(db_err) = bankai.db.update_error(&uuid, &error_msg).await {
                        eprintln!("Failed to update error status in database: {}", db_err);
                    }
                    return Err(BankaiCliError::ProofGenerationError(error_msg));
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum BankaiCliError {
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Proof generation error: {0}")]
    ProofGenerationError(String),
}