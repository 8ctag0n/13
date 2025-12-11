/// ZyberLink Unified CLI
///
/// Single binary for all ZyberLink operations (FHE and ZK).
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "zyb")]
#[command(about = "ZyberLink unified CLI for FHE and ZK operations")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// FHE operations
    #[command(subcommand)]
    Fhe(FheCommands),

    /// ZK operations
    #[command(subcommand)]
    Zk(ZkCommands),

    /// Interactive wizard for creating jobs
    Wizard,
}

#[derive(Subcommand, Debug)]
enum FheCommands {
    /// Encrypt data for a ZyberLink FHE job
    Encrypt {
        /// Path to save generated files
        #[arg(short, long, default_value = "./fhe-output")]
        path: PathBuf,

        /// Single value to encrypt (0-255).
        #[arg(short, long, conflicts_with = "values")]
        value: Option<u8>,

        /// Comma-separated list of values to encrypt for aggregation jobs (e.g., sum, count_if).
        #[arg(long, value_delimiter = ',', conflicts_with = "value")]
        values: Option<Vec<u8>>,
    },

    /// Decrypt result from a completed FHE job
    Decrypt {
        /// Path to the job folder (containing client_key.bin)
        #[arg(short, long)]
        path: PathBuf,

        /// Encrypted result (base64). If not provided, will prompt interactively.
        #[arg(short, long)]
        result: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ZkCommands {
    /// Create a new ZK proof job
    Create {
        /// Circuit type (10-49)
        #[arg(long)]
        circuit_type: u8,

        /// Path to witness JSON file
        #[arg(long)]
        witness: PathBuf,

        /// Creator public key
        #[arg(long)]
        creator: String,

        /// Server URL
        #[arg(long, default_value = "http://localhost:3000")]
        server: String,

        /// Timeout in seconds
        #[arg(long)]
        timeout: Option<i32>,

        /// Path to Solana keypair for payment (enables direct payment flow)
        #[arg(long)]
        keypair: Option<PathBuf>,

        /// Solana RPC URL
        #[arg(long, default_value = "https://api.devnet.solana.com")]
        rpc_url: String,

        /// Skip payment confirmation prompt
        #[arg(long)]
        skip_confirm: bool,
    },

    /// Check status of a ZK job
    Status {
        /// Job ID
        job_id: i64,

        /// Server URL
        #[arg(long, default_value = "http://localhost:3000")]
        server: String,

        /// Show full details
        #[arg(long, short)]
        verbose: bool,
    },

    /// List available ZK circuits
    Circuits,

    /// Verify a ZK proof
    Verify {
        /// Path to proof file
        proof: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Fhe(fhe_cmd) => match fhe_cmd {
            FheCommands::Encrypt {
                path,
                value,
                values,
            } => zyb_cli::commands::fhe::encrypt_command(path, value, values),
            FheCommands::Decrypt { path, result } => {
                zyb_cli::commands::fhe::decrypt_command(path, result)
            }
        },

        Commands::Zk(zk_cmd) => match zk_cmd {
            ZkCommands::Create {
                circuit_type,
                witness,
                creator,
                server,
                timeout,
                keypair,
                rpc_url,
                skip_confirm,
            } => zyb_cli::commands::zk::create_command(
                circuit_type,
                witness,
                creator,
                server,
                timeout,
                keypair,
                rpc_url,
                skip_confirm,
            ),
            ZkCommands::Status { job_id, server, verbose } => {
                zyb_cli::commands::zk::status_command(job_id, server, verbose)
            }
            ZkCommands::Circuits => zyb_cli::commands::zk::circuits_command(),
            ZkCommands::Verify { proof } => zyb_cli::commands::zk::verify_command(proof),
        },

        Commands::Wizard => zyb_cli::ui::run_wizard(),
    }
}
