use anyhow::Result;
use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Print the Zyberlink ASCII banner
pub fn print_banner() {
    let banner = r#"
┌────────────────────────────────────────────────────────────┐
│                                                            │
│    ███████╗██╗   ██╗██████╗ ███████╗██████╗               │
│    ╚══███╔╝╚██╗ ██╔╝██╔══██╗██╔════╝██╔══██╗              │
│      ███╔╝  ╚████╔╝ ██████╔╝█████╗  ██████╔╝              │
│     ███╔╝    ╚██╔╝  ██╔══██╗██╔══╝  ██╔══██╗              │
│    ███████╗   ██║   ██████╔╝███████╗██║  ██║              │
│    ╚══════╝   ╚═╝   ╚═════╝ ╚══════╝╚═╝  ╚═╝              │
│                                                            │
│              LINK                                          │
│                                                            │
│         Prover Node Setup Wizard v1.0                      │
│                                                            │
└────────────────────────────────────────────────────────────┘
"#;

    println!("{}", style(banner).cyan().bold());
}

/// Print welcome message
pub fn print_welcome_message() {
    println!("\n{}", style("Welcome to Zyberlink Prover Node Setup!").bold());
    println!("\nThis wizard will guide you through:");
    println!("  {} System requirements validation", style("✓").green());
    println!("  {} Solana keypair configuration", style("✓").green());
    println!("  {} Network setup", style("✓").green());
    println!("  {} Wallet funding (if needed)", style("✓").green());
    println!("  {} On-chain prover registration", style("✓").green());
    println!("\n{}", style("Estimated time: 5-10 minutes").dim());
}

/// Print step header
pub fn print_step_header(current: usize, total: usize, title: &str) {
    println!("\n{}", style(format!("[{}/{}] {}", current, total, title)).bold().cyan());
    println!("{}", style("━".repeat(60)).cyan());
}

/// Print success message
pub fn print_success(msg: &str) {
    println!("{} {}", style("✓").green().bold(), style(msg).green());
}

/// Print error message
pub fn print_error(msg: &str) {
    println!("{} {}", style("✗").red().bold(), style(msg).red());
}

/// Print warning message
pub fn print_warning(msg: &str) {
    println!("{} {}", style("⚠").yellow().bold(), style(msg).yellow());
}

/// Print info message
pub fn print_info(msg: &str) {
    println!("  {}", style(msg).dim());
}

/// Wait for user to press ENTER
pub fn wait_for_enter() -> Result<()> {
    println!("\n{}", style("Press ENTER to continue...").dim());
    let term = Term::stdout();
    term.read_line()?;
    Ok(())
}

/// Ask user to select from options
pub fn select(prompt: &str, items: &[&str]) -> Result<usize> {
    use dialoguer::Select;
    Ok(Select::new()
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()?)
}

/// Ask user for confirmation
pub fn confirm(prompt: &str) -> Result<bool> {
    use dialoguer::Confirm;
    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()?)
}

/// Ask user for text input
pub fn input(prompt: &str, default: Option<&str>) -> Result<String> {
    use dialoguer::Input;
    let mut input = Input::<String>::new().with_prompt(prompt);

    if let Some(def) = default {
        input = input.default(def.to_string());
    }

    Ok(input.interact()?)
}

/// Ask user for text input with validation and retry logic
///
/// # Arguments
/// * `prompt` - The prompt to display
/// * `default` - Optional default value
/// * `validator` - Function that validates the input, returns Ok(()) if valid or Err with error message
/// * `max_retries` - Maximum number of retry attempts (default: 3)
///
/// # Returns
/// The validated input string, or error after max retries exceeded
pub fn input_with_validation<F>(
    prompt: &str,
    default: Option<&str>,
    validator: F,
    max_retries: usize,
) -> Result<String>
where
    F: Fn(&str) -> Result<()>,
{
    let mut attempts = 0;

    loop {
        attempts += 1;

        // Get input from user
        let value = input(prompt, default)?;

        // Validate input
        match validator(&value) {
            Ok(()) => return Ok(value),
            Err(e) if attempts < max_retries => {
                // Show error and allow retry
                print_error(&format!("{}", e));
                println!();
                print_warning(&format!(
                    "Please try again ({}/{} attempts remaining)",
                    max_retries - attempts,
                    max_retries
                ));
                println!();
            }
            Err(e) => {
                // Max retries exceeded
                print_error(&format!("{}", e));
                return Err(anyhow::anyhow!(
                    "Maximum retry attempts ({}) exceeded. Please restart the wizard.",
                    max_retries
                ));
            }
        }
    }
}

/// Print validation check result
pub fn print_check_result(name: &str, passed: bool, details: Option<&str>) {
    let icon = if passed { "✓" } else { "✗" };
    let color = if passed { style(icon).green() } else { style(icon).red() };

    if let Some(det) = details {
        println!("{} {} {}", color, name, style(det).dim());
    } else {
        println!("{} {}", color, name);
    }
}

/// Print summary box
pub fn print_summary_box(title: &str, lines: &[(&str, &str)]) {
    println!("\n{}", style(format!("📋 {}", title)).bold());
    println!("{}", "─".repeat(60));

    for (key, value) in lines {
        println!("  {:<24} {}", style(key).dim(), style(value).cyan());
    }

    println!("{}", "─".repeat(60));
}

/// Create a spinner with message
pub struct Spinner {
    pb: ProgressBar,
}

impl Spinner {
    pub fn new(msg: &str) -> Self {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("Static progress spinner template should be valid"),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(Duration::from_millis(100));

        Self { pb }
    }

    pub fn update(&self, msg: String) {
        self.pb.set_message(msg);
    }

    pub fn success(self, msg: &str) {
        self.pb.finish_with_message(format!("{} {}", style("✓").green(), msg));
    }

    pub fn error(self, msg: &str) {
        self.pb.finish_with_message(format!("{} {}", style("✗").red(), msg));
    }
}

/// Print final completion screen
pub fn print_completion_screen(config_path: &str, prover_pubkey: &str) {
    println!("\n{}", style("━".repeat(60)).green());
    println!("{}", style("🎉 Setup Complete! Your prover is ready!").green().bold());
    println!("{}", style("━".repeat(60)).green());

    println!("\n{}", style("Configuration saved to:").bold());
    println!("  {}", style(config_path).cyan());

    println!("\n{}", style("Prover Authority:").bold());
    println!("  {}", style(prover_pubkey).cyan());

    println!("\n{}", style("Next steps:").bold());
    println!("  1. Start your prover:  {}", style("zyberlink-prover run").green());
    println!("  2. Monitor with TUI:   {}", style("zyberlink-prover run --tui-mode").green());

    println!("\n{}", style("Need help? Visit https://docs.zyberlink.io").dim());
    println!();
}

/// Generate Solana Pay URL for funding request
pub fn generate_solana_pay_url(
    recipient: &str,
    amount_lamports: u64,
    label: &str,
    message: &str,
) -> String {
    let amount_sol = amount_lamports as f64 / 1_000_000_000.0;
    format!(
        "solana:{}?amount={}&label={}&message={}",
        recipient,
        amount_sol,
        urlencoding::encode(label),
        urlencoding::encode(message)
    )
}

/// Display Solana Pay QR code in terminal
pub fn print_solana_pay_qr(url: &str) -> anyhow::Result<()> {
    use qr2term::print_qr;

    println!("\n{}", style("Scan QR code with Solana wallet:").bold());
    println!();

    print_qr(url)?;

    println!();
    Ok(())
}

/// Generate Solana Action/Blink URL for funding request
pub fn generate_blink_url(
    recipient: &str,
    amount_lamports: u64,
) -> String {
    let amount_sol = amount_lamports as f64 / 1_000_000_000.0;
    let blink_server = std::env::var("BLINK_SERVER_URL")
        .unwrap_or_else(|_| "https://blink.zyberlink.io".to_string());

    let action_url = format!(
        "{}/api/actions/fund-prover?pubkey={}&amount={}",
        blink_server,
        recipient,
        amount_sol
    );

    // Encode action URL for dial.to
    format!(
        "https://dial.to/?action={}",
        urlencoding::encode(&action_url)
    )
}

/// Print Solana Pay funding instructions with Blink
pub fn print_funding_instructions(
    solana_pay_url: &str,
    blink_url: &str,
    recipient: &str,
    amount_sol: f64,
) {
    println!("\n{}", style("━".repeat(60)).cyan());
    println!("{}", style("💰 Fund Your Prover Wallet").cyan().bold());
    println!("{}", style("━".repeat(60)).cyan());

    println!("\n{}", style("Option 1: Scan QR Code").bold());
    println!("  Use any Solana wallet app to scan the QR code above");

    println!("\n{}", style("Option 2: Use Solana Blink (Recommended)").bold());
    println!("  {}", style(blink_url).cyan());
    println!("  {}", style("  → Works in any browser with wallet extension").dim());
    println!("  {}", style("  → Shareable on Twitter, Discord, etc.").dim());

    println!("\n{}", style("Option 3: Solana Pay Link").bold());
    println!("  {}", style(solana_pay_url).cyan());
    println!("  {}", style("  → Direct wallet app link").dim());

    println!("\n{}", style("Option 4: Manual Transfer").bold());
    println!("  Recipient: {}", style(recipient).cyan());
    println!("  Amount:    {} SOL", style(format!("{:.4}", amount_sol)).green().bold());

    println!("\n{}", style("━".repeat(60)).cyan());
    println!();
}
