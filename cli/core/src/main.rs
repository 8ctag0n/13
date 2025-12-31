/// ZyberLink Unified CLI
///
/// Single binary for all ZyberLink operations (FHE and ZK).
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use zyb_cli::commands::compliance::ComplianceCommands;
use zyb_cli::commands::demo::DemoCommands;
use zyb_cli::commands::dev_job::DevJobCommands;
use zyb_cli::commands::market::MarketCommands;
use zyb_cli::commands::profile::ProfileCommands;
use zyb_cli::commands::vote::VoteCommands;

#[derive(Parser, Debug)]
#[command(name = "zyb")]
#[command(about = "ZyberLink unified CLI for FHE and ZK operations")]
#[command(version)]
struct Cli {
    /// Path to config file (default: ./zyb.toml or ~/.config/zyb/zyb.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Profile to use (overrides config file default)
    #[arg(long, global = true)]
    profile: Option<String>,

    /// Blockchain to use (solana, starknet)
    #[arg(long, default_value = "solana", env = "ZYB_CHAIN", global = true)]
    chain: String,

    /// Solana RPC URL (use --config and --profile for better config management)
    #[arg(long, env = "SOLANA_RPC_URL", global = true)]
    rpc_url: Option<String>,

    /// Solana keypair path
    #[arg(long, env = "SOLANA_KEYPAIR", global = true)]
    keypair: Option<PathBuf>,

    /// Starknet RPC URL
    #[arg(long, env = "STARKNET_RPC_URL", default_value = "http://localhost:5050", global = true)]
    starknet_rpc_url: String,

    /// Starknet account address
    #[arg(long, env = "STARKNET_ACCOUNT", global = true)]
    starknet_account: Option<String>,

    /// Starknet private key
    #[arg(long, env = "STARKNET_PRIVATE_KEY", global = true)]
    starknet_private_key: Option<String>,

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

    /// Vote operations (private voting)
    #[command(subcommand)]
    Vote(VoteCommands),

    /// Market operations (prediction markets)
    #[command(subcommand)]
    Market(MarketCommands),

    /// Compliance operations (portfolio verification)
    #[command(subcommand)]
    Compliance(ComplianceCommands),

    /// Manage dev-job profiles
    #[command(subcommand)]
    Profile(ProfileCommands),

    /// Developer job runner (testing/demo)
    #[command(subcommand)]
    DevJob(DevJobCommands),

    /// Visual demos for presentations and videos
    #[command(subcommand)]
    Demo(DemoCommands),

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
                skip_confirm,
            } => zyb_cli::commands::zk::create_command(
                circuit_type,
                witness,
                creator,
                server,
                timeout,
                cli.keypair,
                cli.rpc_url.unwrap_or_else(|| "https://api.devnet.solana.com".to_string()),
                cli.chain,
                cli.starknet_rpc_url,
                cli.starknet_account,
                cli.starknet_private_key,
                skip_confirm,
            ),
            ZkCommands::Status { job_id, server, verbose } => {
                zyb_cli::commands::zk::status_command(job_id, server, verbose)
            }
            ZkCommands::Circuits => zyb_cli::commands::zk::circuits_command(),
            ZkCommands::Verify { proof } => zyb_cli::commands::zk::verify_command(proof),
        },

        Commands::Vote(vote_cmd) => zyb_cli::commands::vote::handle_vote_command(vote_cmd),

        Commands::Market(market_cmd) => {
            zyb_cli::commands::market::handle_market_command(market_cmd)
        }

        Commands::Compliance(compliance_cmd) => {
            zyb_cli::commands::compliance::handle_compliance_command(compliance_cmd)
        }

        Commands::Profile(profile_cmd) => {
            zyb_cli::commands::profile::handle_profile_command(profile_cmd)
        }

        Commands::DevJob(dev_job_cmd) => {
            zyb_cli::commands::dev_job::handle_dev_job_command(dev_job_cmd)
        }

        Commands::Demo(demo_cmd) => {
            zyb_cli::commands::demo::handle_demo_command(demo_cmd)
        }

        Commands::Wizard => zyb_cli::ui::run_wizard(),
    }
}
