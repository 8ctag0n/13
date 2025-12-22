use clap::Parser;

/// ZyberLink Prover Node - Autonomous ZK proof generation daemon
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct ProverArgs {
    #[command(subcommand)]
    pub command: Option<ProverCommand>,

    /// Solana RPC URL
    #[arg(short, long, default_value = "http://localhost:8899", global = true)]
    pub rpc_url: String,

    /// ZyberLink program ID
    #[arg(short, long, global = true)]
    pub program_id: Option<String>,

    /// Path to prover keypair file
    #[arg(short, long, default_value = "~/.config/solana/id.json", global = true)]
    pub keypair: String,

    /// Polling interval in seconds
    #[arg(long, default_value = "5")]
    pub poll_interval: u64,

    /// Minimum job price in lamports to accept (deprecated, use --min-roi instead)
    #[arg(long, default_value = "1000000")]
    pub min_price: u64,

    /// Minimum ROI percentage required to accept a job (0.0 for demo mode)
    #[arg(long, default_value = "0.0")]
    pub min_roi: f64,

    /// Operational cost multiplier for overhead (infrastructure, electricity)
    #[arg(long, default_value = "1.5")]
    pub cost_multiplier: f64,

    /// Mock proving time in seconds (simulates proof generation)
    #[arg(long, default_value = "10")]
    pub mock_proving_time: u64,

    /// Maximum concurrent jobs
    #[arg(long, default_value = "3")]
    pub max_concurrent_jobs: usize,

    /// Gateway URL for witness storage and proof submission
    #[arg(long, default_value = "http://localhost:8080", global = true)]
    pub gateway_url: String,

    /// FHE server key file path (required for FHE jobs)
    #[arg(long, global = true)]
    pub fhe_server_key_path: Option<String>,

    /// Enable TUI (Terminal User Interface) mode
    #[arg(long, global = true)]
    pub tui_mode: bool,

    /// Blink backend URL for ZK job coordination
    #[arg(long, default_value = "http://localhost:3000", global = true)]
    pub blink_backend_url: String,

    /// Path to ZK circuits directory
    #[arg(long, default_value = "./prover-circuits", global = true)]
    pub zk_circuits_path: String,

    /// Enable Futarchy FHE job processing
    #[arg(long, global = true)]
    pub enable_futarchy: bool,

    /// Futarchy blink-server URL (for FHE job polling)
    #[arg(long, default_value = "http://localhost:8090", global = true)]
    pub futarchy_server_url: String,

    /// ZK Generator program ID (for new architecture)
    #[arg(long, env = "ZK_GENERATOR_PROGRAM_ID", global = true)]
    pub zk_generator_program: Option<String>,

    /// FHE Generator program ID (for new architecture)
    #[arg(long, env = "FHE_GENERATOR_PROGRAM_ID", global = true)]
    pub fhe_generator_program: Option<String>,
}

#[derive(Parser, Debug)]
pub enum ProverCommand {
    /// Run the prover daemon (default)
    Run,

    /// Register as a prover on-chain
    Register {
        /// Stake amount in lamports
        #[arg(long, default_value = "10000000000")]
        stake_amount: u64,
    },

    /// Show encryption public key
    ShowPubkey,

    /// Run interactive setup wizard
    Setup {
        /// Stake amount in lamports for registration
        #[arg(long, default_value = "100000000")]
        stake_amount: u64,
    },
}
