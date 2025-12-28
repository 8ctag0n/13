//! pBTCFi Callback Executor
//!
//! Executes pending Starknet callbacks via starkli
//! Reads from pbtcfi_callbacks table and invokes contract functions

use anyhow::Result;
use sqlx::postgres::PgPool;
use std::env;
use std::process::Stdio;
use tokio::process::Command;

const MAX_RETRIES: i32 = 3;

#[derive(Debug, sqlx::FromRow)]
struct PendingCallback {
    id: i32,
    loan_id: String,
    callback_type: String,
    target_contract: String,
    function_name: String,
    call_args: serde_json::Value,
    retry_count: i32,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let dry_run = args.contains(&"--dry-run".to_string());
    let status_only = args.contains(&"--status".to_string());
    let limit: i64 = args
        .iter()
        .position(|x| x == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://zyberlink:dev_password@localhost:5432/zyberlink".into());

    let pool = PgPool::connect(&database_url).await?;

    println!("\n{}", "=".repeat(60));
    println!("pBTCFi Callback Executor");
    println!("{}", "=".repeat(60));
    println!("RPC: {}", env::var("STARKNET_RPC_URL").unwrap_or_else(|_| "http://localhost:5050".into()));
    println!("Dry Run: {}", dry_run);
    println!("{}\n", "=".repeat(60));

    if status_only {
        show_status(&pool).await?;
        return Ok(());
    }

    process_callbacks(&pool, limit, dry_run).await
}

async fn show_status(pool: &PgPool) -> Result<()> {
    let stats: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*) FROM pbtcfi_callbacks GROUP BY status"
    )
    .fetch_all(pool)
    .await?;

    println!("Callback Status:");
    println!("{}", "-".repeat(30));
    for (status, count) in &stats {
        println!("  {}: {}", status, count);
    }

    let recent: Vec<(i32, String, String, String, Option<String>)> = sqlx::query_as(
        r#"
        SELECT id, loan_id, callback_type, status, error_message
        FROM pbtcfi_callbacks
        ORDER BY created_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(pool)
    .await?;

    if !recent.is_empty() {
        println!("\nRecent Callbacks (last 10):");
        println!("{}", "-".repeat(80));
        for (id, loan_id, cb_type, status, error) in recent {
            let icon = match status.as_str() {
                "pending" => "[.]",
                "processing" => "[~]",
                "executed" => "[+]",
                "failed" => "[X]",
                _ => "[?]",
            };
            let loan_short = if loan_id.len() > 16 { &loan_id[..16] } else { &loan_id };
            println!("  {} #{} {} - {}...", icon, id, cb_type, loan_short);
            if let Some(err) = error {
                let err_short = if err.len() > 60 { &err[..60] } else { &err };
                println!("      Error: {}...", err_short);
            }
        }
    }

    Ok(())
}

async fn process_callbacks(pool: &PgPool, limit: i64, dry_run: bool) -> Result<()> {
    let callbacks: Vec<PendingCallback> = sqlx::query_as(
        r#"
        SELECT id, loan_id, callback_type, target_contract,
               function_name, call_args, retry_count
        FROM pbtcfi_callbacks
        WHERE status = 'pending' AND retry_count < $1
        ORDER BY created_at ASC
        LIMIT $2
        "#
    )
    .bind(MAX_RETRIES)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    if callbacks.is_empty() {
        println!("No pending callbacks found.");
        return Ok(());
    }

    println!("Found {} pending callback(s)\n", callbacks.len());

    let mut success_count = 0;
    let mut fail_count = 0;

    for cb in callbacks {
        println!("Processing callback #{}:", cb.id);
        println!("  Type: {}", cb.callback_type);
        println!("  Loan: {}", cb.loan_id);
        println!("  Target: {}", cb.target_contract);
        println!("  Function: {}", cb.function_name);
        println!("  Retry: {}/{}", cb.retry_count, MAX_RETRIES);

        // Mark processing
        sqlx::query("UPDATE pbtcfi_callbacks SET status = 'processing', last_retry_at = NOW() WHERE id = $1")
            .bind(cb.id)
            .execute(pool)
            .await?;

        match execute_callback(&cb, dry_run).await {
            Ok(tx_hash) => {
                println!("  Result: SUCCESS");
                println!("  TX Hash: {}", tx_hash);
                sqlx::query("UPDATE pbtcfi_callbacks SET status = 'executed', tx_hash = $2, executed_at = NOW() WHERE id = $1")
                    .bind(cb.id)
                    .bind(&tx_hash)
                    .execute(pool)
                    .await?;
                success_count += 1;
            }
            Err(e) => {
                let error_msg = e.to_string();
                println!("  Result: FAILED");
                println!("  Error: {}", error_msg);
                sqlx::query(
                    r#"
                    UPDATE pbtcfi_callbacks
                    SET status = CASE WHEN retry_count + 1 >= $2 THEN 'failed' ELSE 'pending' END,
                        error_message = $3,
                        retry_count = retry_count + 1,
                        last_retry_at = NOW()
                    WHERE id = $1
                    "#
                )
                .bind(cb.id)
                .bind(MAX_RETRIES)
                .bind(&error_msg)
                .execute(pool)
                .await?;
                fail_count += 1;
            }
        }

        println!();
    }

    println!("{}", "=".repeat(60));
    println!("Summary: {} succeeded, {} failed", success_count, fail_count);
    println!("{}", "=".repeat(60));

    Ok(())
}

async fn execute_callback(cb: &PendingCallback, dry_run: bool) -> Result<String> {
    let rpc_url = env::var("STARKNET_RPC_URL").unwrap_or_else(|_| "http://localhost:5050".into());
    let account = env::var("STARKNET_ACCOUNT").ok();
    let keystore = env::var("STARKNET_KEYSTORE").ok();

    let mut cmd = Command::new("starkli");
    cmd.arg("invoke")
        .arg("--rpc").arg(&rpc_url)
        .arg(&cb.target_contract)
        .arg(&cb.function_name);

    // Add call arguments
    if let Some(args) = cb.call_args.as_array() {
        for arg in args {
            if let Some(s) = arg.as_str() {
                cmd.arg(s);
            } else {
                cmd.arg(arg.to_string().trim_matches('"'));
            }
        }
    }

    if let Some(acc) = account {
        cmd.arg("--account").arg(acc);
    }
    if let Some(ks) = keystore {
        cmd.arg("--keystore").arg(ks);
    }

    println!("  Command: starkli invoke --rpc {} {} {} [args]",
        rpc_url, cb.target_contract, cb.function_name);

    if dry_run {
        println!("  [DRY RUN] Would execute above command");
        return Ok(format!("dry_run_tx_{}", cb.id));
    }

    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let tx_hash = stdout.lines().last().unwrap_or("unknown").to_string();
        Ok(tx_hash)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(anyhow::anyhow!("{}{}", stderr, stdout))
    }
}
