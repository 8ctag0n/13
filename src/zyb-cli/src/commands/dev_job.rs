use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use blake2::{Blake2s256, Digest};
use clap::{Args, Subcommand, ValueEnum};
use rand::seq::SliceRandom;
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use zyberlink_fhe::{generate_keys, prelude::*, ClientKey, FheUint8};
use zyberlink_sdk::MarketplaceSDK;
use zyberlink_types::fhe::{FheConsensusConfig, FheOperation, FhePredicate, HistogramBin};

#[derive(Subcommand, Debug)]
pub enum DevJobCommands {
    /// Create dev jobs (continuous or one-cycle runs)
    Run(DevJobRunArgs),
    /// Verify jobs end-to-end (submit -> wait -> decrypt)
    Verify(DevJobVerifyArgs),
    /// Plan jobs without submitting (dry run)
    Plan(DevJobPlanArgs),
    /// Simulate webapp flow (validate-and-build)
    WebappFlow(DevJobWebappArgs),
    /// Simulate webapp Proof of Innocence flow
    WebappFlowPoi(DevJobCommonArgs),
}

#[derive(Args, Debug, Clone)]
pub struct DevJobCommonArgs {
    /// Config file path (default: dev-job.toml in current directory)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Profile name from config (overrides common.profile)
    #[arg(long)]
    pub profile: Option<String>,

    /// Program ID (overrides PROGRAM_ID env var)
    #[arg(long)]
    pub program_id: Option<String>,

    /// Solana RPC URL (overrides SOLANA_RPC_URL env var)
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// Backend URL (overrides BACKEND_URL env var)
    #[arg(long)]
    pub backend_url: Option<String>,

    /// Path to Solana keypair JSON (overrides USER_KEYPAIR env var)
    #[arg(long)]
    pub keypair: Option<PathBuf>,

    /// Disable auto-airdrop on low balance
    #[arg(long)]
    pub no_airdrop: bool,

    /// Emit JSON events in addition to human logs
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct DevJobRunArgs {
    #[command(flatten)]
    pub common: DevJobCommonArgs,

    /// Comma-separated list of job types to run (overrides --cases)
    #[arg(long, value_delimiter = ',')]
    pub types: Option<Vec<DevJobType>>,

    /// Comma-separated list of cases to run (poi, futarchy, analytics, mix)
    #[arg(long, value_delimiter = ',')]
    pub cases: Option<Vec<DevJobCase>>,

    /// Seconds between jobs
    #[arg(long)]
    pub interval_secs: Option<u64>,

    /// Run a single cycle and exit
    #[arg(long)]
    pub once: bool,

    /// Shuffle job order each cycle
    #[arg(long)]
    pub shuffle: bool,
}

#[derive(Args, Debug)]
pub struct DevJobVerifyArgs {
    #[command(flatten)]
    pub common: DevJobCommonArgs,

    /// Comma-separated list of job types to verify
    #[arg(long, value_delimiter = ',')]
    pub types: Option<Vec<DevJobType>>,

    /// Verify all supported types (excluding histogram)
    #[arg(long)]
    pub all: bool,
}

#[derive(Args, Debug)]
pub struct DevJobPlanArgs {
    #[command(flatten)]
    pub common: DevJobCommonArgs,

    /// Comma-separated list of job types to plan (overrides --cases)
    #[arg(long, value_delimiter = ',')]
    pub types: Option<Vec<DevJobType>>,

    /// Comma-separated list of cases to plan (poi, futarchy, analytics, mix)
    #[arg(long, value_delimiter = ',')]
    pub cases: Option<Vec<DevJobCase>>,
}

#[derive(Args, Debug)]
pub struct DevJobWebappArgs {
    #[command(flatten)]
    pub common: DevJobCommonArgs,

    /// Wait for completion and verify result
    #[arg(long)]
    pub verify: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DevJobType {
    Add,
    Multiply,
    Sum,
    Threshold,
    Range,
    Average,
    #[value(name = "count-if")]
    CountIf,
    Histogram,
}

#[derive(Clone, Copy, Debug, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DevJobCase {
    Poi,
    Futarchy,
    Analytics,
    Mix,
}

#[derive(Clone, Debug)]
struct JobSpec {
    name: &'static str,
    operation: FheOperation,
    values: Vec<u8>,
    expected: Option<u8>,
}

impl JobSpec {
    fn new(
        name: &'static str,
        operation: FheOperation,
        values: Vec<u8>,
        expected: Option<u8>,
    ) -> Self {
        Self {
            name,
            operation,
            values,
            expected,
        }
    }
}

#[derive(Default)]
struct RunStats {
    total: u64,
    submitted: u64,
    failed: u64,
    total_price: u64,
}

struct JobRunOutcome {
    total_price: u64,
}

struct VerifyOutcome {
    status: &'static str,
}

impl VerifyOutcome {
    fn passed() -> Self {
        Self { status: "passed" }
    }

    fn failed() -> Self {
        Self { status: "failed" }
    }
}

#[derive(Debug)]
struct DevJobConfig {
    rpc_url: String,
    program_id: solana_sdk::pubkey::Pubkey,
    backend_url: String,
    keypair_path: PathBuf,
    no_airdrop: bool,
    json: bool,
}

struct DevJobContext {
    sdk: MarketplaceSDK,
    rpc_client: RpcClient,
    http_client: reqwest::Client,
    user_keypair: Keypair,
    backend_url: String,
    json: bool,
}

#[derive(Default, Deserialize)]
struct DevJobFileConfig {
    common: Option<DevJobCommonFileConfig>,
    profiles: Option<HashMap<String, DevJobProfileConfig>>,
    run: Option<DevJobRunFileConfig>,
    verify: Option<DevJobVerifyFileConfig>,
    webapp: Option<DevJobWebappFileConfig>,
}

#[derive(Default, Deserialize)]
struct DevJobCommonFileConfig {
    profile: Option<String>,
    program_id: Option<String>,
    rpc_url: Option<String>,
    backend_url: Option<String>,
    keypair: Option<PathBuf>,
    no_airdrop: Option<bool>,
    json: Option<bool>,
}

#[derive(Clone, Default, Deserialize)]
struct DevJobProfileConfig {
    program_id: Option<String>,
    rpc_url: Option<String>,
    backend_url: Option<String>,
    keypair: Option<PathBuf>,
    no_airdrop: Option<bool>,
    json: Option<bool>,
}

#[derive(Default, Deserialize)]
struct DevJobRunFileConfig {
    types: Option<Vec<DevJobType>>,
    cases: Option<Vec<DevJobCase>>,
    interval_secs: Option<u64>,
    once: Option<bool>,
    shuffle: Option<bool>,
}

#[derive(Default, Deserialize)]
struct DevJobVerifyFileConfig {
    types: Option<Vec<DevJobType>>,
    all: Option<bool>,
}

#[derive(Default, Deserialize)]
struct DevJobWebappFileConfig {
    verify: Option<bool>,
}

pub fn handle_dev_job_command(command: DevJobCommands) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("Failed to start tokio runtime")?;
    rt.block_on(handle_dev_job_command_async(command))
}

async fn handle_dev_job_command_async(command: DevJobCommands) -> Result<()> {
    let _ = env_logger::try_init();

    match command {
        DevJobCommands::Run(args) => {
            let (args, file_config) = apply_run_config(args)?;
            let context = build_context(&args.common, file_config.as_ref()).await?;
            run_dev_job_sequence(context, args, file_config.as_ref()).await
        }
        DevJobCommands::Verify(args) => {
            let (args, file_config) = apply_verify_config(args)?;
            let context = build_context(&args.common, file_config.as_ref()).await?;
            run_verify_sequence(context, args, file_config.as_ref()).await
        }
        DevJobCommands::Plan(args) => {
            let (args, file_config) = apply_plan_config(args)?;
            let context = build_context(&args.common, file_config.as_ref()).await?;
            run_plan_sequence(context, args, file_config.as_ref()).await
        }
        DevJobCommands::WebappFlow(args) => {
            let (args, file_config) = apply_webapp_config(args)?;
            let context = build_context(&args.common, file_config.as_ref()).await?;
            run_webapp_flow(
                &context.rpc_client,
                &context.http_client,
                &context.user_keypair,
                &context.backend_url,
                args.verify,
                context.json,
            )
            .await
        }
        DevJobCommands::WebappFlowPoi(args) => {
            let file_config = load_file_config(&args.config)?;
            let context = build_context(&args, file_config.as_ref()).await?;
            run_webapp_flow_poi(
                &context.rpc_client,
                &context.http_client,
                &context.user_keypair,
                &context.backend_url,
                context.json,
            )
            .await
        }
    }
}

async fn build_context(
    args: &DevJobCommonArgs,
    file_config: Option<&DevJobFileConfig>,
) -> Result<DevJobContext> {
    let config = load_config(args, file_config)?;

    let user_keypair = if config.keypair_path.exists() {
        log::info!("Loading keypair from {}", config.keypair_path.display());
        read_keypair_file(&config.keypair_path)?
    } else {
        log::info!("Creating new keypair at {}", config.keypair_path.display());
        let keypair = Keypair::new();
        write_keypair_file(&keypair, &config.keypair_path)?;
        keypair
    };

    log::info!("Dev Job starting...");
    log::info!("RPC URL: {}", config.rpc_url);
    log::info!("Program ID: {}", config.program_id);
    log::info!("Backend URL: {}", config.backend_url);
    log::info!("User: {}", user_keypair.pubkey());

    let http_client = reqwest::Client::new();
    let rpc_client = RpcClient::new_with_commitment(config.rpc_url, CommitmentConfig::confirmed());

    if !config.no_airdrop {
        ensure_balance(&rpc_client, &user_keypair).await?;
    }

    let sdk = MarketplaceSDK::new(config.program_id);

    Ok(DevJobContext {
        sdk,
        rpc_client,
        http_client,
        user_keypair,
        backend_url: config.backend_url,
        json: config.json,
    })
}

fn load_config(
    args: &DevJobCommonArgs,
    file_config: Option<&DevJobFileConfig>,
) -> Result<DevJobConfig> {
    let common = file_config.and_then(|cfg| cfg.common.as_ref());
    let profile_name = args
        .profile
        .clone()
        .or_else(|| common.and_then(|cfg| cfg.profile.clone()));

    let profile = match (profile_name, file_config.and_then(|cfg| cfg.profiles.as_ref())) {
        (Some(name), Some(profiles)) => Some(
            profiles
                .get(&name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Profile not found in config: {}", name))?,
        ),
        (Some(name), None) => {
            return Err(anyhow::anyhow!("Profile not found in config: {}", name))
        }
        (None, _) => None,
    };

    let rpc_url = args
        .rpc_url
        .clone()
        .or_else(|| profile.as_ref().and_then(|cfg| cfg.rpc_url.clone()))
        .or_else(|| common.and_then(|cfg| cfg.rpc_url.clone()))
        .or_else(|| std::env::var("SOLANA_RPC_URL").ok())
        .unwrap_or_else(|| "http://localhost:8899".to_string());

    let program_id_str = args
        .program_id
        .clone()
        .or_else(|| profile.as_ref().and_then(|cfg| cfg.program_id.clone()))
        .or_else(|| common.and_then(|cfg| cfg.program_id.clone()))
        .or_else(|| std::env::var("PROGRAM_ID").ok())
        .context("PROGRAM_ID env var required (or use --program-id)")?;

    let program_id: solana_sdk::pubkey::Pubkey = program_id_str
        .parse()
        .context("Invalid PROGRAM_ID")?;

    let backend_url = args
        .backend_url
        .clone()
        .or_else(|| profile.as_ref().and_then(|cfg| cfg.backend_url.clone()))
        .or_else(|| common.and_then(|cfg| cfg.backend_url.clone()))
        .or_else(|| std::env::var("BACKEND_URL").ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string());

    let keypair_path = args
        .keypair
        .clone()
        .or_else(|| profile.as_ref().and_then(|cfg| cfg.keypair.clone()))
        .or_else(|| common.and_then(|cfg| cfg.keypair.clone()))
        .or_else(|| std::env::var("USER_KEYPAIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/tmp/job-creator-keypair.json"));

    let no_airdrop = args.no_airdrop
        || profile.as_ref().and_then(|cfg| cfg.no_airdrop).unwrap_or(false)
        || common.and_then(|cfg| cfg.no_airdrop).unwrap_or(false);
    let json = args.json
        || profile.as_ref().and_then(|cfg| cfg.json).unwrap_or(false)
        || common.and_then(|cfg| cfg.json).unwrap_or(false);

    Ok(DevJobConfig {
        rpc_url,
        program_id,
        backend_url,
        keypair_path,
        no_airdrop,
        json,
    })
}

async fn run_dev_job_sequence(
    context: DevJobContext,
    args: DevJobRunArgs,
    file_config: Option<&DevJobFileConfig>,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  Dev Job - Run Mode");
    log::info!("===========================================");
    log::info!("");

    let run_config = file_config.and_then(|cfg| cfg.run.as_ref());
    let types = args.types.as_deref().or(run_config.and_then(|cfg| cfg.types.as_deref()));
    let cases = args.cases.as_deref().or(run_config.and_then(|cfg| cfg.cases.as_deref()));
    let interval_secs = args
        .interval_secs
        .or(run_config.and_then(|cfg| cfg.interval_secs))
        .unwrap_or(10);
    let once = args.once || run_config.and_then(|cfg| cfg.once).unwrap_or(false);
    let shuffle = args.shuffle || run_config.and_then(|cfg| cfg.shuffle).unwrap_or(false);

    let base_specs = build_specs_for_run(types, cases)?;
    if base_specs.is_empty() {
        anyhow::bail!("No job specs resolved from --types or --cases");
    }

    let mut cycle = 0u64;
    let interval = Duration::from_secs(interval_secs);
    let mut rng = rand::thread_rng();

    loop {
        cycle += 1;
        log::info!("--- Cycle {} ---", cycle);

        let mut cycle_specs = base_specs.clone();
        if shuffle {
            cycle_specs.shuffle(&mut rng);
        }

        let mut stats = RunStats::default();
        for spec in cycle_specs {
            stats.total += 1;
            match create_and_submit_job(&context, &spec).await {
                Ok(outcome) => {
                    stats.submitted += 1;
                    stats.total_price += outcome.total_price;
                }
                Err(e) => {
                    stats.failed += 1;
                    log::error!("Failed to create job {}: {}", spec.name, e);
                    emit_json(
                        context.json,
                        serde_json::json!({
                            "event": "job_failed",
                            "job": spec.name,
                            "error": e.to_string(),
                        }),
                    );
                }
            }

            if interval.as_secs() > 0 {
                tokio::time::sleep(interval).await;
            }
        }

        log::info!(
            "Cycle {} summary: submitted {}, failed {}, total_price {} lamports",
            cycle,
            stats.submitted,
            stats.failed,
            stats.total_price
        );
        emit_json(
            context.json,
            serde_json::json!({
                "event": "run_summary",
                "cycle": cycle,
                "submitted": stats.submitted,
                "failed": stats.failed,
                "total_price_lamports": stats.total_price,
            }),
        );

        if once {
            break;
        }
    }

    Ok(())
}

async fn run_verify_sequence(
    context: DevJobContext,
    args: DevJobVerifyArgs,
    file_config: Option<&DevJobFileConfig>,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  Dev Job - Verify Mode");
    log::info!("===========================================");
    log::info!("");

    let verify_config = file_config.and_then(|cfg| cfg.verify.as_ref());
    let all = args.all || verify_config.and_then(|cfg| cfg.all).unwrap_or(false);
    let types = if all {
        vec![
            DevJobType::Add,
            DevJobType::Multiply,
            DevJobType::Sum,
            DevJobType::Threshold,
            DevJobType::Range,
            DevJobType::Average,
            DevJobType::CountIf,
        ]
    } else if let Some(types) = args
        .types
        .clone()
        .or_else(|| verify_config.and_then(|cfg| cfg.types.clone()))
    {
        types
    } else {
        anyhow::bail!("Use --types or --all to select what to verify");
    };

    let mut passed = 0u64;
    let mut failed = 0u64;
    for job_type in types {
        let spec = spec_from_type(job_type)?;
        let expected = spec.expected.context("Selected type does not support verify")?;

        let outcome = run_verified_job(
            &context.sdk,
            &context.rpc_client,
            &context.http_client,
            &context.user_keypair,
            &context.backend_url,
            spec.operation.clone(),
            &spec.values,
            expected,
            spec.name,
            context.json,
        )
        .await?;

        if outcome.status == "passed" {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    log::info!("Verify summary: {} passed, {} failed", passed, failed);
    emit_json(
        context.json,
        serde_json::json!({
            "event": "verify_summary",
            "passed": passed,
            "failed": failed,
        }),
    );

    Ok(())
}

fn apply_run_config(args: DevJobRunArgs) -> Result<(DevJobRunArgs, Option<DevJobFileConfig>)> {
    let file_config = load_file_config(&args.common.config)?;
    Ok((args, file_config))
}

fn apply_verify_config(
    args: DevJobVerifyArgs,
) -> Result<(DevJobVerifyArgs, Option<DevJobFileConfig>)> {
    let file_config = load_file_config(&args.common.config)?;
    Ok((args, file_config))
}

fn apply_webapp_config(
    args: DevJobWebappArgs,
) -> Result<(DevJobWebappArgs, Option<DevJobFileConfig>)> {
    let file_config = load_file_config(&args.common.config)?;
    let mut args = args;
    if !args.verify {
        if let Some(verify) = file_config
            .as_ref()
            .and_then(|cfg| cfg.webapp.as_ref())
            .and_then(|cfg| cfg.verify)
        {
            args.verify = verify;
        }
    }
    Ok((args, file_config))
}

fn apply_plan_config(args: DevJobPlanArgs) -> Result<(DevJobPlanArgs, Option<DevJobFileConfig>)> {
    let file_config = load_file_config(&args.common.config)?;
    Ok((args, file_config))
}

fn load_file_config(path: &Option<PathBuf>) -> Result<Option<DevJobFileConfig>> {
    let config_path = path.clone().unwrap_or_else(|| PathBuf::from("dev-job.toml"));
    if !config_path.exists() {
        if path.is_some() {
            anyhow::bail!("Config file not found: {}", config_path.display());
        }
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file {}", config_path.display()))?;
    let config: DevJobFileConfig =
        toml::from_str(&contents).context("Failed to parse dev-job.toml")?;
    Ok(Some(config))
}

async fn run_plan_sequence(
    context: DevJobContext,
    args: DevJobPlanArgs,
    file_config: Option<&DevJobFileConfig>,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  Dev Job - Plan Mode");
    log::info!("===========================================");
    log::info!("");

    let run_config = file_config.and_then(|cfg| cfg.run.as_ref());
    let types = args.types.as_deref().or(run_config.and_then(|cfg| cfg.types.as_deref()));
    let cases = args.cases.as_deref().or(run_config.and_then(|cfg| cfg.cases.as_deref()));

    let specs = build_specs_for_run(types, cases)?;
    if specs.is_empty() {
        anyhow::bail!("No job specs resolved from --types or --cases");
    }

    let required_provers = 3u8;
    let mut total_price = 0u64;

    for spec in &specs {
        let cost_config = spec.operation.get_cost_config();
        let price = cost_config.min_payment_lamports * 2 * (required_provers as u64);
        total_price += price;
        log::info!(
            "- {} | op={} | values={} | expected={} | price={} | timeout={}s",
            spec.name,
            spec.operation.name(),
            spec.values.len(),
            spec.expected
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string()),
            price,
            cost_config.timeout_seconds
        );
        emit_json(
            context.json,
            serde_json::json!({
                "event": "plan_job",
                "job": spec.name,
                "operation": spec.operation.name(),
                "values": spec.values.len(),
                "expected": spec.expected,
                "price_lamports": price,
                "timeout_seconds": cost_config.timeout_seconds,
            }),
        );
    }

    log::info!("Plan summary: {} jobs, total_price {} lamports", specs.len(), total_price);
    emit_json(
        context.json,
        serde_json::json!({
            "event": "plan_summary",
            "jobs": specs.len(),
            "total_price_lamports": total_price,
        }),
    );

    Ok(())
}

fn build_specs_for_run(
    types: Option<&[DevJobType]>,
    cases: Option<&[DevJobCase]>,
) -> Result<Vec<JobSpec>> {
    if let Some(types) = types {
        let mut specs = Vec::with_capacity(types.len());
        for job_type in types {
            specs.push(spec_from_type(*job_type)?);
        }
        return Ok(specs);
    }

    let cases = cases.map(|cases| cases.to_vec()).unwrap_or_else(|| vec![DevJobCase::Mix]);
    let mut specs = Vec::new();
    for case in cases {
        specs.extend(specs_from_case(case));
    }
    Ok(specs)
}

fn spec_from_type(job_type: DevJobType) -> Result<JobSpec> {
    Ok(match job_type {
        DevJobType::Add => JobSpec::new(
            "Add (50 + 10)",
            FheOperation::Add(10),
            vec![50],
            Some(60),
        ),
        DevJobType::Multiply => JobSpec::new(
            "Multiply (5 * 3)",
            FheOperation::Multiply(3),
            vec![5],
            Some(15),
        ),
        DevJobType::Sum => JobSpec::new(
            "Sum [10,20,30]",
            FheOperation::Sum { expected_count: 3 },
            vec![10, 20, 30],
            Some(60),
        ),
        DevJobType::Threshold => JobSpec::new(
            "Threshold 75 >= 50",
            FheOperation::Threshold {
                threshold: 50,
                greater_or_equal: true,
            },
            vec![75],
            Some(1),
        ),
        DevJobType::Range => JobSpec::new(
            "Range 50 in [0,100]",
            FheOperation::RangeCheck { min: 0, max: 100 },
            vec![50],
            Some(1),
        ),
        DevJobType::Average => JobSpec::new(
            "Average [10,20,30,40,50]",
            FheOperation::Average { expected_count: 5 },
            vec![10, 20, 30, 40, 50],
            Some(30),
        ),
        DevJobType::CountIf => JobSpec::new(
            "CountIf >= 18 (PoI)",
            FheOperation::CountIf {
                predicate: FhePredicate::GreaterThan(17),
                expected_count: 4,
            },
            vec![15, 20, 25, 17],
            Some(2),
        ),
        DevJobType::Histogram => JobSpec::new(
            "Histogram [0-9,10-19,20-29,30-39]",
            FheOperation::Histogram {
                bins: default_histogram_bins(),
            },
            vec![5, 12, 19, 27, 33],
            None,
        ),
    })
}

fn specs_from_case(case: DevJobCase) -> Vec<JobSpec> {
    match case {
        DevJobCase::Poi => vec![JobSpec::new(
            "CountIf >= 18 (PoI)",
            FheOperation::CountIf {
                predicate: FhePredicate::GreaterThan(17),
                expected_count: 4,
            },
            vec![15, 20, 25, 17],
            Some(2),
        )],
        DevJobCase::Futarchy => vec![
            JobSpec::new(
                "Sum [10,20,30,40,50]",
                FheOperation::Sum { expected_count: 5 },
                vec![10, 20, 30, 40, 50],
                Some(150),
            ),
            JobSpec::new(
                "Average [10,20,30,40,50]",
                FheOperation::Average { expected_count: 5 },
                vec![10, 20, 30, 40, 50],
                Some(30),
            ),
            JobSpec::new(
                "Threshold 75 >= 50",
                FheOperation::Threshold {
                    threshold: 50,
                    greater_or_equal: true,
                },
                vec![75],
                Some(1),
            ),
        ],
        DevJobCase::Analytics => vec![
            JobSpec::new(
                "Sum [10,20,30,40,50]",
                FheOperation::Sum { expected_count: 5 },
                vec![10, 20, 30, 40, 50],
                Some(150),
            ),
            JobSpec::new(
                "Average [10,20,30,40,50]",
                FheOperation::Average { expected_count: 5 },
                vec![10, 20, 30, 40, 50],
                Some(30),
            ),
            JobSpec::new(
                "Histogram [0-9,10-19,20-29,30-39]",
                FheOperation::Histogram {
                    bins: default_histogram_bins(),
                },
                vec![5, 12, 19, 27, 33],
                None,
            ),
        ],
        DevJobCase::Mix => vec![
            JobSpec::new(
                "CountIf >= 18 (PoI)",
                FheOperation::CountIf {
                    predicate: FhePredicate::GreaterThan(17),
                    expected_count: 4,
                },
                vec![15, 20, 25, 17],
                Some(2),
            ),
            JobSpec::new(
                "Threshold 75 >= 50",
                FheOperation::Threshold {
                    threshold: 50,
                    greater_or_equal: true,
                },
                vec![75],
                Some(1),
            ),
            JobSpec::new(
                "Range 50 in [0,100]",
                FheOperation::RangeCheck { min: 0, max: 100 },
                vec![50],
                Some(1),
            ),
            JobSpec::new(
                "Sum [10,20,30,40,50]",
                FheOperation::Sum { expected_count: 5 },
                vec![10, 20, 30, 40, 50],
                Some(150),
            ),
            JobSpec::new(
                "Average [10,20,30,40,50]",
                FheOperation::Average { expected_count: 5 },
                vec![10, 20, 30, 40, 50],
                Some(30),
            ),
            JobSpec::new(
                "Histogram [0-9,10-19,20-29,30-39]",
                FheOperation::Histogram {
                    bins: default_histogram_bins(),
                },
                vec![5, 12, 19, 27, 33],
                None,
            ),
        ],
    }
}

fn default_histogram_bins() -> Vec<HistogramBin> {
    vec![
        HistogramBin::new(0, 9, "0-9"),
        HistogramBin::new(10, 19, "10-19"),
        HistogramBin::new(20, 29, "20-29"),
        HistogramBin::new(30, 39, "30-39"),
    ]
}

async fn create_and_submit_job(context: &DevJobContext, spec: &JobSpec) -> Result<JobRunOutcome> {
    log::info!("Creating job: {}", spec.name);

    let (encrypted_data, server_key, _client_key) =
        create_fhe_data_with_values(&spec.values, &spec.operation)?;
    log::info!(
        "FHE data created ({} bytes encrypted, {} bytes server key)",
        encrypted_data.len(),
        server_key.len()
    );

    let cost_config = spec.operation.get_cost_config();
    let required_provers = 3u8;
    let consensus_threshold = 2u8;

    let mut encrypted_input = Vec::new();
    encrypted_input.extend_from_slice(&(encrypted_data.len() as u32).to_le_bytes());
    encrypted_input.extend_from_slice(&encrypted_data);
    encrypted_input.extend_from_slice(&server_key);

    let commitment = upload_witness(&context.http_client, &context.backend_url, &encrypted_input).await?;
    log::info!("Witness uploaded, commitment: {}", commitment);

    let fhe_config = FheConsensusConfig {
        required_provers,
        consensus_threshold,
        submission_timeout_secs: cost_config.timeout_seconds,
        operation: spec.operation.clone(),
    };

    let total_price = cost_config.min_payment_lamports * 2 * (required_provers as u64);
    let job_id = context.sdk.get_next_job_id(&context.rpc_client)?;

    let create_job_ix = context.sdk.create_fhe_job(
        context.user_keypair.pubkey(),
        job_id,
        &encrypted_input,
        fhe_config,
        total_price,
        cost_config.timeout_seconds,
    )?;

    let recent_blockhash = context.rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&context.user_keypair.pubkey()));
    tx.sign(&[&context.user_keypair], recent_blockhash);

    let signature = context.rpc_client.send_and_confirm_transaction(&tx)?;
    log::info!("Job {} created, signature: {}", job_id, signature);
    emit_json(
        context.json,
        serde_json::json!({
            "event": "job_submitted",
            "job": spec.name,
            "job_id": job_id,
            "signature": signature.to_string(),
            "operation": spec.operation.name(),
            "price_lamports": total_price,
        }),
    );
    log::info!("");

    Ok(JobRunOutcome { total_price })
}

#[derive(Debug, Deserialize)]
struct WitnessUploadResponse {
    commitment: String,
}

#[derive(Debug, Deserialize)]
struct JobStatusResponse {
    status: String,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FheResultResponse {
    encrypted_result: String,
}

#[derive(Debug, Deserialize)]
struct ServerKeyUploadResponse {
    server_key_hash: String,
}

#[derive(Debug, Deserialize)]
struct ValidateAndBuildResponse {
    job_id: u64,
    transaction: String,
}

async fn run_verified_job(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
    operation: FheOperation,
    test_values: &[u8],
    expected_result: u8,
    test_name: &str,
    emit_json_events: bool,
) -> Result<VerifyOutcome> {
    log::info!("[1/6] Generating FHE keys and encrypting data...");
    let (encrypted_data, server_key, client_key) =
        create_fhe_data_with_values(test_values, &operation)?;
    log::info!(
        "  Encrypted {} values ({} bytes data, {} bytes server key)",
        test_values.len(),
        encrypted_data.len(),
        server_key.len()
    );

    log::info!("[2/6] Uploading witness to backend...");
    let mut encrypted_input = Vec::new();
    encrypted_input.extend_from_slice(&(encrypted_data.len() as u32).to_le_bytes());
    encrypted_input.extend_from_slice(&encrypted_data);
    encrypted_input.extend_from_slice(&server_key);

    let commitment = upload_witness(http_client, backend_url, &encrypted_input).await?;
    log::info!("  Commitment: {}", commitment);

    log::info!("[3/6] Creating job on-chain...");
    let cost_config = operation.get_cost_config();
    let required_provers = 3u8;
    let consensus_threshold = 2u8;

    let fhe_config = FheConsensusConfig {
        required_provers,
        consensus_threshold,
        submission_timeout_secs: cost_config.timeout_seconds,
        operation: operation.clone(),
    };

    let total_price = cost_config.min_payment_lamports * 2 * (required_provers as u64);
    let job_id = sdk.get_next_job_id(rpc_client)?;

    let create_job_ix = sdk.create_fhe_job(
        user_keypair.pubkey(),
        job_id,
        &encrypted_input,
        fhe_config,
        total_price,
        cost_config.timeout_seconds,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&user_keypair.pubkey()));
    tx.sign(&[user_keypair], recent_blockhash);

    let signature = rpc_client.send_and_confirm_transaction(&tx)?;
    log::info!("  Job {} created, signature: {}", job_id, signature);
    emit_json(
        emit_json_events,
        serde_json::json!({
            "event": "job_submitted",
            "job": test_name,
            "job_id": job_id,
            "signature": signature.to_string(),
            "operation": operation.name(),
        }),
    );

    log::info!(
        "[4/6] Waiting for job completion (timeout: {}s)...",
        cost_config.timeout_seconds
    );
    let final_status =
        poll_job_status(http_client, backend_url, job_id, cost_config.timeout_seconds as u64)
            .await?;

    if final_status != "completed" {
        log::error!("  Job failed with status: {}", final_status);
        log::error!("");
        log::error!("===========================================");
        log::error!("  {} Test: FAILED (job status: {})", test_name, final_status);
        log::error!("===========================================");
        return Ok(VerifyOutcome::failed());
    }
    log::info!("  Job completed!");
    emit_json(
        emit_json_events,
        serde_json::json!({
            "event": "job_completed",
            "job": test_name,
            "job_id": job_id,
            "status": final_status,
        }),
    );

    log::info!("[5/6] Fetching encrypted result...");
    let encrypted_result = fetch_result(http_client, backend_url, job_id).await?;
    log::info!("  Got encrypted result ({} bytes)", encrypted_result.len());

    log::info!("[6/6] Decrypting and verifying result...");
    let decrypted = decrypt_result(&encrypted_result, &client_key, &operation)?;
    log::info!("  Decrypted result: {}", decrypted);
    log::info!("  Expected result:  {}", expected_result);

    log::info!("");
    if decrypted == expected_result {
        log::info!("===========================================");
        log::info!("  {} Test: PASSED", test_name);
        log::info!("===========================================");
        emit_json(
            emit_json_events,
            serde_json::json!({
                "event": "job_verified",
                "job": test_name,
                "job_id": job_id,
                "status": "passed",
                "result": decrypted,
                "expected": expected_result,
            }),
        );
        return Ok(VerifyOutcome::passed());
    } else {
        log::error!("===========================================");
        log::error!("  {} Test: FAILED", test_name);
        log::error!("  Expected: {}, Got: {}", expected_result, decrypted);
        log::error!("===========================================");
        emit_json(
            emit_json_events,
            serde_json::json!({
                "event": "job_verified",
                "job": test_name,
                "job_id": job_id,
                "status": "failed",
                "result": decrypted,
                "expected": expected_result,
            }),
        );
        return Ok(VerifyOutcome::failed());
    }
}

fn create_fhe_data_with_values(
    values: &[u8],
    operation: &FheOperation,
) -> Result<(Vec<u8>, Vec<u8>, ClientKey)> {
    let (client_key, server_key) = generate_keys()?;

    let encrypted_bytes = match operation {
        FheOperation::Add(_)
        | FheOperation::Multiply(_)
        | FheOperation::Threshold { .. }
        | FheOperation::RangeCheck { .. } => {
            if values.len() != 1 {
                anyhow::bail!(
                    "Single-value operations require exactly 1 input value, got {}",
                    values.len()
                );
            }
            let encrypted = FheUint8::encrypt(values[0], &client_key);
            bincode::serialize(&encrypted)?
        }
        FheOperation::Sum { .. }
        | FheOperation::Average { .. }
        | FheOperation::CountIf { .. }
        | FheOperation::Histogram { .. } => {
            let mut encrypted_values: Vec<Vec<u8>> = Vec::with_capacity(values.len());
            for &value in values {
                let encrypted = FheUint8::encrypt(value, &client_key);
                let enc_bytes = bincode::serialize(&encrypted)?;
                encrypted_values.push(enc_bytes);
            }
            bincode::serialize(&encrypted_values)?
        }
        FheOperation::FutarchyPoolUpdate { .. } => {
            anyhow::bail!("FutarchyPoolUpdate is not supported by dev-job yet");
        }
    };

    let server_key_bytes = bincode::serialize(&server_key)?;

    Ok((encrypted_bytes, server_key_bytes, client_key))
}

fn decrypt_result(
    encrypted_result: &[u8],
    client_key: &ClientKey,
    operation: &FheOperation,
) -> Result<u8> {
    match operation {
        FheOperation::Average { .. } => {
            let (sum_bytes, count): (Vec<u8>, u16) = bincode::deserialize(encrypted_result)?;
            let sum_encrypted: FheUint8 = bincode::deserialize(&sum_bytes)?;
            let sum: u8 = sum_encrypted.decrypt(client_key);
            let average = sum / (count as u8);
            Ok(average)
        }
        _ => {
            let encrypted: FheUint8 = bincode::deserialize(encrypted_result)?;
            let decrypted: u8 = encrypted.decrypt(client_key);
            Ok(decrypted)
        }
    }
}

async fn upload_witness(
    http_client: &reqwest::Client,
    backend_url: &str,
    data: &[u8],
) -> Result<String> {
    let mut hasher = Blake2s256::new();
    hasher.update(data);
    let local_commitment = hex::encode(hasher.finalize());

    let url = format!("{}/witness", backend_url);
    let response = http_client
        .post(&url)
        .body(data.to_vec())
        .header("Content-Type", "application/octet-stream")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Backend returned error: {}", response.status());
    }

    let resp: WitnessUploadResponse = response.json().await?;

    if resp.commitment != local_commitment {
        anyhow::bail!(
            "Commitment mismatch! Local: {}, Backend: {}",
            local_commitment,
            resp.commitment
        );
    }

    Ok(resp.commitment)
}

async fn poll_job_status(
    http_client: &reqwest::Client,
    backend_url: &str,
    job_id: u64,
    timeout_secs: u64,
) -> Result<String> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs + 60);
    let poll_interval = Duration::from_secs(10);

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Job polling timeout after {}s", timeout_secs);
        }

        let url = format!("{}/api/jobs/{}", backend_url, job_id);
        let response = http_client.get(&url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let status: JobStatusResponse = resp.json().await?;
                log::info!("  Status: {} (elapsed: {:?})", status.status, start.elapsed());

                match status.status.as_str() {
                    "completed" => return Ok("completed".to_string()),
                    "failed" => {
                        return Ok(format!("failed: {}", status.error.unwrap_or_default()))
                    }
                    "expired" => return Ok("expired".to_string()),
                    _ => {}
                }
            }
            Ok(resp) => {
                log::warn!("  Status check returned: {}", resp.status());
            }
            Err(e) => {
                log::warn!("  Status check failed: {}", e);
            }
        }

        tokio::time::sleep(poll_interval).await;
    }
}

async fn fetch_result(
    http_client: &reqwest::Client,
    backend_url: &str,
    job_id: u64,
) -> Result<Vec<u8>> {
    let url = format!("{}/api/jobs/{}/result", backend_url, job_id);
    let response = http_client.get(&url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch result: {}", response.status());
    }

    let result: FheResultResponse = response.json().await?;
    let bytes = BASE64.decode(&result.encrypted_result)?;
    Ok(bytes)
}

async fn ensure_balance(rpc_client: &RpcClient, keypair: &Keypair) -> Result<()> {
    let balance = rpc_client.get_balance(&keypair.pubkey())?;
    log::info!("User balance: {} SOL", balance as f64 / 1_000_000_000.0);

    if balance < 20_000_000 {
        log::warn!("Low balance! Requesting airdrop...");
        let signature = rpc_client.request_airdrop(&keypair.pubkey(), 1_000_000_000)?;
        log::info!("Airdrop signature: {}", signature);
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Ok(())
}

fn read_keypair_file(path: &PathBuf) -> Result<Keypair> {
    let contents = std::fs::read_to_string(path)?;
    let bytes: Vec<u8> = serde_json::from_str(&contents)?;
    Keypair::try_from(bytes.as_slice()).context("Invalid keypair bytes")
}

fn write_keypair_file(keypair: &Keypair, path: &PathBuf) -> Result<()> {
    let bytes = keypair.to_bytes();
    let json = serde_json::to_string(&bytes.to_vec())?;
    std::fs::write(path, json)?;
    Ok(())
}

async fn run_webapp_flow(
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
    verify_result: bool,
    emit_json_events: bool,
) -> Result<()> {
    let total_steps = if verify_result { 8 } else { 6 };

    log::info!("");
    log::info!("===========================================");
    log::info!(
        "  Webapp Flow Simulation{}",
        if verify_result { " + Verification" } else { "" }
    );
    log::info!("  Testing: server_key pre-upload + validate-and-build");
    if verify_result {
        log::info!("  + Wait for completion + Decrypt & verify result");
    }
    log::info!("===========================================");
    log::info!("");

    log::info!("[1/{}] Generating FHE keys and encrypting data...", total_steps);
    let test_values: Vec<u8> = vec![10, 20, 30];
    let expected_result: u8 = test_values.iter().map(|&x| x as u16).sum::<u16>() as u8;
    let operation = FheOperation::Sum { expected_count: 3 };

    let (encrypted_data, server_key, client_key) =
        create_fhe_data_with_values(&test_values, &operation)?;
    log::info!(
        "  Encrypted {} values: {} bytes encrypted_data, {} bytes server_key ({:.1} MB)",
        test_values.len(),
        encrypted_data.len(),
        server_key.len(),
        server_key.len() as f64 / 1024.0 / 1024.0
    );

    log::info!(
        "[2/{}] Uploading server_key to /api/server-key/upload...",
        total_steps
    );
    let upload_url = format!("{}/api/server-key/upload", backend_url);

    let upload_start = std::time::Instant::now();
    let upload_response = http_client
        .post(&upload_url)
        .header("Content-Type", "application/octet-stream")
        .body(server_key.clone())
        .timeout(Duration::from_secs(600))
        .send()
        .await
        .context("Failed to upload server key")?;

    if !upload_response.status().is_success() {
        let error_text = upload_response.text().await.unwrap_or_default();
        anyhow::bail!("Server key upload failed: {}", error_text);
    }

    let upload_result: ServerKeyUploadResponse = upload_response.json().await?;
    let upload_elapsed = upload_start.elapsed();
    log::info!(
        "  Uploaded in {:.1}s, hash: {}",
        upload_elapsed.as_secs_f64(),
        upload_result.server_key_hash
    );

    log::info!("[3/{}] Creating signature for job creation...", total_steps);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let nonce = format!("{:x}", rand::random::<u64>());
    let job_id_placeholder = timestamp * 1000 + rand::random::<u64>() % 1000;
    let message = format!("create_job:{}:{}:{}", job_id_placeholder, timestamp, nonce);

    let signature = user_keypair.sign_message(message.as_bytes());
    let signature_base58 = bs58::encode(signature.as_ref()).into_string();
    log::info!("  Message: {}", message);
    log::info!("  Signature: {}...", &signature_base58[..20]);

    log::info!("[4/{}] Calling /api/jobs/validate-and-build...", total_steps);
    let encrypted_data_base64 = BASE64.encode(&encrypted_data);

    let validate_url = format!("{}/api/jobs/validate-and-build", backend_url);
    let validate_body = serde_json::json!({
        "creator_pubkey": user_keypair.pubkey().to_string(),
        "encrypted_data": encrypted_data_base64,
        "server_key_hash": upload_result.server_key_hash,
        "message": message,
        "signature": signature_base58,
        "nonce": nonce,
        "operation": "sum",
        "operation_value": 0,
        "expected_count": test_values.len(),
        "price_lamports": 15000000,
        "required_provers": 3,
        "consensus_threshold": 2,
        "payment_method": "SOL"
    });

    let validate_response = http_client
        .post(&validate_url)
        .header("Content-Type", "application/json")
        .json(&validate_body)
        .send()
        .await
        .context("Failed to call validate-and-build")?;

    if !validate_response.status().is_success() {
        let error_text = validate_response.text().await.unwrap_or_default();
        anyhow::bail!("validate-and-build failed: {}", error_text);
    }

    let validate_result: ValidateAndBuildResponse = validate_response.json().await?;
    log::info!("  Job ID: {}", validate_result.job_id);
    log::info!(
        "  Transaction received ({} bytes base64)",
        validate_result.transaction.len()
    );

    log::info!("[5/{}] Signing and submitting transaction...", total_steps);
    let tx_bytes = BASE64.decode(&validate_result.transaction)?;
    let mut tx: Transaction = bincode::deserialize(&tx_bytes)?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    tx.sign(&[user_keypair], recent_blockhash);

    let signature = rpc_client
        .send_and_confirm_transaction(&tx)
        .context("Failed to submit transaction")?;
    log::info!("  Transaction confirmed: {}", signature);

    log::info!("[6/{}] Confirming job with backend...", total_steps);
    let confirm_url = format!("{}/api/jobs/{}/confirm", backend_url, validate_result.job_id);
    let confirm_response = http_client
        .post(&confirm_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "signature": signature.to_string() }))
        .send()
        .await?;

    if !confirm_response.status().is_success() {
        log::warn!("  Confirm returned non-success (may be ok if chain sync handles it)");
    } else {
        log::info!("  Job confirmed with backend");
    }

    if !verify_result {
        log::info!("");
        log::info!("===========================================");
        log::info!("  SUCCESS! Webapp flow works correctly");
        log::info!("  Job ID: {}", validate_result.job_id);
        log::info!("===========================================");
        log::info!("");
        emit_json(
            emit_json_events,
            serde_json::json!({
                "event": "webapp_flow_complete",
                "job_id": validate_result.job_id,
                "status": "submitted",
            }),
        );
        return Ok(());
    }

    log::info!("[7/{}] Waiting for job completion...", total_steps);
    log::info!("  Test values: {:?} -> expected sum: {}", test_values, expected_result);

    let poll_interval = Duration::from_secs(5);
    let max_wait = Duration::from_secs(180);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > max_wait {
            anyhow::bail!("Timeout waiting for job completion after {:?}", max_wait);
        }

        let result_url = format!("{}/api/jobs/{}/result", backend_url, validate_result.job_id);
        if let Ok(resp) = http_client.get(&result_url).send().await {
            if resp.status().is_success() {
                log::info!(
                    "  Job completed! (result available, elapsed: {:?})",
                    start.elapsed()
                );
                break;
            }
        }

        log::info!("  Waiting for result... (elapsed: {:?})", start.elapsed());
        tokio::time::sleep(poll_interval).await;
    }

    log::info!("[8/{}] Fetching and verifying result...", total_steps);

    let encrypted_result = fetch_result(http_client, backend_url, validate_result.job_id).await?;
    log::info!("  Got encrypted result ({} bytes)", encrypted_result.len());

    let decrypted = decrypt_result(&encrypted_result, &client_key, &operation)?;
    log::info!("  Decrypted result: {}", decrypted);
    log::info!("  Expected result:  {}", expected_result);

    log::info!("");
    if decrypted == expected_result {
        log::info!("===========================================");
        log::info!("  SUCCESS! Full E2E test PASSED");
        log::info!("  Job ID: {}", validate_result.job_id);
        log::info!("  Input: {:?}", test_values);
        log::info!("  Operation: Sum");
        log::info!("  Result: {} (correct!)", decrypted);
        log::info!("===========================================");
        emit_json(
            emit_json_events,
            serde_json::json!({
                "event": "webapp_flow_verified",
                "job_id": validate_result.job_id,
                "status": "passed",
                "result": decrypted,
                "expected": expected_result,
            }),
        );
    } else {
        log::error!("===========================================");
        log::error!("  FAILED! Result mismatch");
        log::error!("  Expected: {}, Got: {}", expected_result, decrypted);
        log::error!("===========================================");
        emit_json(
            emit_json_events,
            serde_json::json!({
                "event": "webapp_flow_verified",
                "job_id": validate_result.job_id,
                "status": "failed",
                "result": decrypted,
                "expected": expected_result,
            }),
        );
        anyhow::bail!("Result verification failed");
    }
    log::info!("");

    Ok(())
}

async fn run_webapp_flow_poi(
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
    emit_json_events: bool,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  Webapp Proof of Innocence Flow");
    log::info!("  Testing: count_if with Equals predicate");
    log::info!("===========================================");
    log::info!("");

    let user_history: Vec<u8> = vec![10, 20, 30, 40, 50];
    let sanctioned_index: u8 = 66;
    let expected_result: u8 = 0;

    log::info!("  User history: {:?}", user_history);
    log::info!("  Sanctioned index to check: {}", sanctioned_index);
    log::info!(
        "  Expected result: {} (0 = innocent, >0 = guilty)",
        expected_result
    );
    log::info!("");

    log::info!("[1/8] Generating FHE keys and encrypting user history...");
    let operation = FheOperation::CountIf {
        predicate: FhePredicate::Equals(sanctioned_index),
        expected_count: user_history.len() as u16,
    };

    let (encrypted_data, server_key, client_key) =
        create_fhe_data_with_values(&user_history, &operation)?;
    log::info!(
        "  Encrypted {} history entries: {} bytes data, {:.1} MB server_key",
        user_history.len(),
        encrypted_data.len(),
        server_key.len() as f64 / 1024.0 / 1024.0
    );

    log::info!("[2/8] Uploading server_key to /api/server-key/upload...");
    let upload_url = format!("{}/api/server-key/upload", backend_url);
    let upload_start = std::time::Instant::now();

    let upload_response = http_client
        .post(&upload_url)
        .header("Content-Type", "application/octet-stream")
        .body(server_key.clone())
        .timeout(Duration::from_secs(600))
        .send()
        .await
        .context("Failed to upload server key")?;

    if !upload_response.status().is_success() {
        let error_text = upload_response.text().await.unwrap_or_default();
        anyhow::bail!("Server key upload failed: {}", error_text);
    }

    let upload_result: ServerKeyUploadResponse = upload_response.json().await?;
    log::info!(
        "  Uploaded in {:.1}s, hash: {}",
        upload_start.elapsed().as_secs_f64(),
        upload_result.server_key_hash
    );

    log::info!("[3/8] Creating signature for job creation...");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let nonce = format!("{:x}", rand::random::<u64>());
    let job_id_placeholder = timestamp * 1000 + rand::random::<u64>() % 1000;
    let message = format!("create_job:{}:{}:{}", job_id_placeholder, timestamp, nonce);

    let signature = user_keypair.sign_message(message.as_bytes());
    let signature_base58 = bs58::encode(signature.as_ref()).into_string();
    log::info!("  Message: {}", message);

    log::info!("[4/8] Calling /api/jobs/validate-and-build with count_if...");
    let encrypted_data_base64 = BASE64.encode(&encrypted_data);
    let validate_url = format!("{}/api/jobs/validate-and-build", backend_url);

    let validate_body = serde_json::json!({
        "creator_pubkey": user_keypair.pubkey().to_string(),
        "encrypted_data": encrypted_data_base64,
        "server_key_hash": upload_result.server_key_hash,
        "message": message,
        "signature": signature_base58,
        "nonce": nonce,
        "operation": "count_if",
        "operation_value": sanctioned_index,
        "expected_count": user_history.len(),
        "price_lamports": 50000000,
        "required_provers": 3,
        "consensus_threshold": 2,
        "payment_method": "SOL",
        "predicate": {
            "Equals": sanctioned_index
        }
    });

    log::info!("  Request body (predicate): {:?}", validate_body["predicate"]);

    let validate_response = http_client
        .post(&validate_url)
        .header("Content-Type", "application/json")
        .json(&validate_body)
        .send()
        .await
        .context("Failed to call validate-and-build")?;

    if !validate_response.status().is_success() {
        let status = validate_response.status();
        let error_text = validate_response.text().await.unwrap_or_default();
        anyhow::bail!("validate-and-build failed ({}): {}", status, error_text);
    }

    let validate_result: ValidateAndBuildResponse = validate_response.json().await?;
    log::info!("  Job ID: {}", validate_result.job_id);

    log::info!("[5/8] Signing and submitting transaction...");
    let tx_bytes = BASE64.decode(&validate_result.transaction)?;
    let mut tx: Transaction = bincode::deserialize(&tx_bytes)?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    tx.sign(&[user_keypair], recent_blockhash);

    let tx_signature = rpc_client
        .send_and_confirm_transaction(&tx)
        .context("Failed to submit transaction")?;
    log::info!("  Transaction confirmed: {}", tx_signature);

    log::info!("[6/8] Confirming job with backend...");
    let confirm_url = format!("{}/api/jobs/{}/confirm", backend_url, validate_result.job_id);
    let _ = http_client
        .post(&confirm_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "signature": tx_signature.to_string() }))
        .send()
        .await;
    log::info!("  Job confirmed");

    log::info!("[7/8] Waiting for job completion...");
    let poll_interval = Duration::from_secs(10);
    let max_wait = Duration::from_secs(400);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > max_wait {
            anyhow::bail!("Timeout waiting for job completion");
        }

        let result_url = format!("{}/api/jobs/{}/result", backend_url, validate_result.job_id);
        if let Ok(resp) = http_client.get(&result_url).send().await {
            if resp.status().is_success() {
                log::info!("  Job completed! (elapsed: {:?})", start.elapsed());
                break;
            }
        }

        log::info!("  Waiting... (elapsed: {:?})", start.elapsed());
        tokio::time::sleep(poll_interval).await;
    }

    log::info!("[8/8] Decrypting and verifying result...");
    let encrypted_result = fetch_result(http_client, backend_url, validate_result.job_id).await?;
    log::info!("  Got encrypted result ({} bytes)", encrypted_result.len());

    let decrypted = decrypt_result(&encrypted_result, &client_key, &operation)?;
    log::info!("  Decrypted result: {}", decrypted);
    log::info!("  Expected result:  {}", expected_result);

    log::info!("");
    if decrypted == expected_result {
        log::info!("===========================================");
        log::info!("  PROOF OF INNOCENCE: VERIFIED");
        log::info!("  User history: {:?}", user_history);
        log::info!("  Sanctioned index checked: {}", sanctioned_index);
        log::info!("  Count of matches: {} (0 = INNOCENT)", decrypted);
        log::info!("===========================================");
        emit_json(
            emit_json_events,
            serde_json::json!({
                "event": "webapp_flow_poi_verified",
                "job_id": validate_result.job_id,
                "status": "passed",
                "result": decrypted,
                "expected": expected_result,
            }),
        );
    } else {
        log::error!("===========================================");
        log::error!("  RESULT MISMATCH");
        log::error!("  Expected: {}, Got: {}", expected_result, decrypted);
        log::error!("===========================================");
        emit_json(
            emit_json_events,
            serde_json::json!({
                "event": "webapp_flow_poi_verified",
                "job_id": validate_result.job_id,
                "status": "failed",
                "result": decrypted,
                "expected": expected_result,
            }),
        );
        anyhow::bail!("Result verification failed");
    }
    log::info!("");

    Ok(())
}

fn emit_json(enabled: bool, value: serde_json::Value) {
    if enabled {
        println!("{}", value);
    }
}
