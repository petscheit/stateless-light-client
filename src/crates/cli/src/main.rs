use bankai_core::{db::Status, fetcher::recursive_epoch_input::{RecursiveEpochInputs, RecursiveEpochUpdate}, utils::{constants::{GENESIS_EPOCH, SLOTS_PER_EPOCH}, hashing::get_committee_hash}, BankaiClient};
use cairo_runner::recursive_epoch::RecursiveEpochUpdateCairo;
use bankai_core::fetcher::sync_committee_input::CommitteeUpdateData;
use clap::{Parser, Subcommand};
use dotenv::from_filename;
use tracing::{Level, info, warn, error, debug};
use tracing_subscriber::FmtSubscriber;
use std::time::Instant;

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
        fast_forward: Option<u64>,
        #[arg(long, short)]
        simulate: bool,
        #[arg(long, short)]
        export: Option<String>,
    },
    CommitteeUpdate {
        #[arg(long, short)]
        slot: u64,
    }
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

    info!("🚀 Starting Bankai CLI");
    let start_time = Instant::now();

    let cli = Cli::parse();
    
    info!("🔌 Initializing Bankai client...");
    let bankai = BankaiClient::new(false).await;
    info!("✅ Bankai client initialized successfully");

    match cli.command {
        Commands::Fetch(cmd) => match cmd {
            FetchCommands::Genesis => {
                // info!("📥 Fetching genesis committee information...");
                // let proof: RecursiveEpochUpdate = RecursiveEpochInputs::new(&bankai.client, &bankai.db, None)
                //     .await
                //     .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to generate genesis inputs: {}", e)))?
                //     .into();
                // let committee_hash = get_committee_hash(proof.inputs.epoch_update.aggregate_pub.0);
                // info!("✅ Genesis committee hash: {}", committee_hash);
            }
            FetchCommands::RecursiveEpoch { export } => {
                // info!("📥 Fetching recursive epoch update data...");
                // let proof: RecursiveEpochUpdate = RecursiveEpochInputs::new(&bankai.client, &bankai.db, None)
                //     .await
                //     .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to generate recursive epoch inputs: {}", e)))?
                //     .into();

                // // let cairo_proof: RecursiveEpochUpdateCairo = proof.into();

                // debug!("🧮 Running Cairo program for validation...");
                // let json = serde_json::to_string_pretty(&proof)?;
                // let _pie = cairo_runner::run("cairo/build/recursive_update.json", proof.into())
                //     .map_err(|e| BankaiCliError::ProofGenerationError(format!("Cairo runner failed: {}", e)))?;
                // debug!("✅ Cairo program executed successfully");


                // if let Some(path) = export {
                //     match std::fs::write(path.clone(), json) {
                //         Ok(_) => info!("💾 Proof exported to: {}", path),
                //         Err(e) => return Err(BankaiCliError::IoError(e)),
                //     }
                // } else {
                //     info!("📄 Proof data generated successfully (use --export to save to file)");
                // }
            }
        },
        Commands::Prove(cmd) => match cmd {
            ProveCommands::CommitteeUpdate { slot } => {
                let committee_update_data = CommitteeUpdateData::new(&bankai.client, slot).await.unwrap();
                let cairo_committee_update_data = committee_update_data.into();
                let pie = cairo_runner::run_committee_update("cairo/build/signer_new.json", cairo_committee_update_data).unwrap();
                // println!("PIE: {:?}", pie);
               
            }
            ProveCommands::Genesis => {
                // info!("🔍 Checking for existing genesis proof...");
                // if let Some(_) = bankai.db.get_latest_epoch_update().await
                //     .map_err(|e| BankaiCliError::ProofGenerationError(format!("Database error: {}", e)))? {
                //     return Err(BankaiCliError::ProofGenerationError("Genesis proof already exists".to_string()));
                // }
                
                // info!("🏗️  Generating genesis proof...");
                // let proof: RecursiveEpochUpdate = RecursiveEpochInputs::new(&bankai.client, &bankai.db, None)
                //     .await
                //     .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to generate genesis inputs: {}", e)))?
                //     .into();

                // let epoch = proof.inputs.epoch_update.header.slot / SLOTS_PER_EPOCH;
                // let slot = proof.inputs.epoch_update.header.slot;
                // info!("📊 Genesis proof details - Epoch: {}, Slot: {}", epoch, slot);
                
                // let uuid = bankai.db.create_epoch_update(epoch.clone(), slot, proof.outputs.clone()).await
                //     .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to create epoch update record: {}", e)))?;
                // info!("🆔 Created epoch update record with UUID: {}", uuid);

                // let result = async {
                //     info!("🔄 Updating status to TraceGen...");
                //     bankai.db.update_status(&uuid, Status::TraceGen).await?;
                    
                //     info!("🧮 Running Cairo program to generate PIE...");
                //     let pie = cairo_runner::run("cairo/build/recursive_update.json", proof.into())
                //         .map_err(|e| format!("Cairo runner failed: {}", e))?;
                //     info!("✅ PIE generated successfully");

                //     info!("🚀 Submitting proof to Atlantic...");
                //     let altantic_id = bankai.atlantic_client.submit_stone(pie, format!("epoch_{}", epoch)).await
                //         .map_err(|e| format!("Atlantic submission failed: {}", e))?;
                //     info!("✅ Proof submitted to Atlantic with ID: {}", altantic_id);
                    
                //     bankai.db.add_atlantic_id(&uuid, &altantic_id).await?;
                //     bankai.db.update_status(&uuid, Status::Proving).await?;
                //     info!("🔄 Status updated to Proving");

                //     Ok::<(), Box<dyn std::error::Error>>(())
                // }.await;

                // if let Err(e) = result {
                //     let error_msg = format!("Genesis proof generation failed: {}", e);
                //     error!("❌ {}", error_msg);
                //     if let Err(db_err) = bankai.db.update_error(&uuid, &error_msg).await {
                //         error!("💥 Failed to update error status in database: {}", db_err);
                //     }
                //     return Err(BankaiCliError::ProofGenerationError(error_msg));
                // }
            }
            ProveCommands::RecursiveEpoch { simulate, export, fast_forward } => {
            //     info!("🔍 Looking for previous epoch update...");
            //     let prev_epoch = match bankai.db.get_latest_epoch_update().await
            //         .map_err(|e| BankaiCliError::ProofGenerationError(format!("Database error: {}", e)))? {
            //         Some(epoch_update) => {
            //             info!("✅ Found previous epoch update - Epoch: {}, UUID: {}", epoch_update.epoch_number, epoch_update.uuid);
            //             epoch_update
            //         },
            //         None => return Err(BankaiCliError::ProofGenerationError("No previous epoch update found. Please run genesis first".to_string())),
            //     };

            //     let atlantic_id = prev_epoch.atlantic_id.as_ref()
            //         .ok_or_else(|| BankaiCliError::ProofGenerationError("Previous epoch update has no Atlantic ID".to_string()))?;
                
            //     info!("🔍 Checking Atlantic batch status for ID: {}", atlantic_id);
            //     let status = bankai.atlantic_client.check_batch_status(atlantic_id).await
            //         .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to check Atlantic batch status: {}", e)))?;
                
            //     info!("📊 Atlantic batch status: {}", status);
            //     match status.as_str() {
            //         "FAILED" => {
            //             let error_msg = format!("Proving failed for Atlantic ID: {}", atlantic_id);
            //             error!("❌ {}", error_msg);
            //             bankai.db.update_error(&prev_epoch.uuid, "Proving failed").await
            //                 .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to update error status: {}", e)))?;
            //             return Err(BankaiCliError::ProofGenerationError(error_msg));
            //         }
            //         "DONE" => {
            //             info!("🎉 Proof completed! Fetching from Atlantic...");
            //             let proof = bankai.atlantic_client.fetch_proof(atlantic_id).await
            //                 .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to fetch proof: {}", e)))?;
                        
            //             let proof_id = bankai.db.add_proof(&proof.proof.to_string()).await
            //                 .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to add proof to database: {}", e)))?;
                        
            //             bankai.db.update_proof_id(&prev_epoch.uuid, proof_id).await
            //                 .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to update proof ID: {}", e)))?;
            //             bankai.db.update_status(&prev_epoch.uuid, Status::Done).await
            //                 .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to update status: {}", e)))?;
                        
            //             info!("✅ Proof fetched and stored successfully");
            //         }
            //         _ => {
            //             warn!("⏳ Proof not ready yet (status: {}). Please try again later", status);
            //             return Ok(());
            //         }
            //     }

            //     if simulate {
            //         info!("🧪 Running simulation mode...");
            //         let proof: RecursiveEpochUpdate = RecursiveEpochInputs::new(&bankai.client, &bankai.db, fast_forward)
            //             .await
            //             .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to generate simulation inputs: {}", e)))?
            //             .into();
                    
            //         let sync_committee_info = serde_json::to_string_pretty(&proof.inputs.sync_committee_update)?;
            //         info!("🔍 Sync committee update info:");
            //         println!("{}", sync_committee_info);
            //         return Ok(());
            //     }

            //     if let Some(ff) = fast_forward {
            //         info!("⚡ Fast-forwarding {} epochs", ff);
            //     }

            //     info!("🏗️  Generating recursive epoch proof...");
            //     let proof: RecursiveEpochUpdate = RecursiveEpochInputs::new(&bankai.client, &bankai.db, fast_forward)
            //         .await
            //         .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to generate recursive epoch inputs: {}", e)))?
            //         .into();
                
            //     let epoch = proof.inputs.epoch_update.header.slot / SLOTS_PER_EPOCH;
            //     let slot = proof.inputs.epoch_update.header.slot;
            //     info!("📊 Recursive epoch proof details - Target Epoch: {}, Slot: {}", epoch, slot);
                
            //     let uuid = bankai.db.create_epoch_update(epoch.clone(), slot, proof.outputs.clone()).await
            //         .map_err(|e| BankaiCliError::ProofGenerationError(format!("Failed to create epoch update record: {}", e)))?;
            //     info!("🆔 Created epoch update record with UUID: {}", uuid);

            //     let result = async {
            //         info!("🔄 Updating status to TraceGen...");
            //         // bankai.db.update_status(&uuid, Status::TraceGen).await?;
                    
            //         info!("🧮 Running Cairo program to generate PIE...");
            //         let pie = cairo_runner::run("cairo/build/recursive_update.json", proof.into())
            //             .map_err(|e| format!("Cairo runner failed: {}", e))?;
            //         info!("✅ PIE generated successfully");

            //         info!("🚀 Submitting proof to Atlantic...");
            //         let altantic_id = bankai.atlantic_client.submit_stone(pie, format!("epoch_{}", epoch)).await
            //             .map_err(|e| format!("Atlantic submission failed: {}", e))?;
            //         info!("✅ Proof submitted to Atlantic with ID: {}", altantic_id);
                    
            //         bankai.db.add_atlantic_id(&uuid, &altantic_id).await?;
            //         bankai.db.update_status(&uuid, Status::Proving).await?;
            //         info!("🔄 Status updated to Proving");

            //         Ok::<(), Box<dyn std::error::Error>>(())
            //     }.await;

            //     if let Err(e) = result {
            //         let error_msg = format!("Recursive epoch proof generation failed: {}", e);
            //         error!("❌ {}", error_msg);
            //         if let Err(db_err) = bankai.db.update_error(&uuid, &error_msg).await {
            //             error!("💥 Failed to update error status in database: {}", db_err);
            //         }
            //         return Err(BankaiCliError::ProofGenerationError(error_msg));
            //     }

            //     if let Some(path) = export {
            //         warn!("⚠️  Export functionality not implemented for recursive epoch proving yet");
            //     }
            }
        }
    }

    let duration = start_time.elapsed();
    info!("🏁 Bankai CLI completed successfully in {:.2?}", duration);
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