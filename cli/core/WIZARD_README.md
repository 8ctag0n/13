# Zyb CLI - Interactive Wizard

## Overview

The `zyb wizard` command provides an interactive, user-friendly interface for creating ZK proof jobs and FHE computations without needing to remember complex command-line arguments.

## Features

- **Interactive menu system** using dialoguer for intuitive navigation
- **Visual feedback** with colored output and progress indicators
- **Step-by-step guidance** for creating ZK proof jobs
- **Smart defaults** for common configuration options
- **Error handling** with helpful error messages

## Usage

### Starting the Wizard

```bash
zyb wizard
```

### Main Menu Options

1. **Create a ZK proof job** - Guided workflow for creating zero-knowledge proof jobs
2. **Create an FHE computation job** - Coming soon
3. **Check job status** - Query the status of an existing job
4. **List available circuits** - View all supported ZK circuits
5. **Exit** - Close the wizard

## Creating a ZK Proof Job

The wizard will guide you through the following steps:

### 1. Circuit Selection

Choose from 8 available circuits:
- **ProofOfInnocence (10)** - Prove you're not on a blacklist
- **PrivateVote (20)** - Anonymous voting
- **PrivateVoteWithPoI (21)** - Voting with innocence check
- **MarketBet (30)** - Private market betting
- **MarketBetWithPoI (31)** - Market bet with innocence check
- **MarketClaim (32)** - Claim market winnings
- **PortfolioCompliance (40)** - Verify portfolio rules
- **PortfolioNetWorth (41)** - Prove net worth range

### 2. Witness File

Provide the path to your witness JSON file. The wizard will:
- Validate the file exists
- Calculate the witness commitment (SHA3-256 hash)
- Extract public inputs
- Show file size and metadata

**Example witness file:**
```json
{
  "publicInputs": [
    "0",
    "1234567890"
  ],
  "privateInputs": {
    "userSecret": "42",
    "commitment": "0x1a2b3c4d5e6f7890"
  }
}
```

### 3. Solana Keypair

Enter the path to your Solana keypair for payment:
- Default: `~/.config/solana/id.json`
- The wizard will derive your public key automatically
- Supports tilde expansion (`~`)

### 4. Server Configuration

Configure connection settings:
- **Server URL**: Default `http://localhost:3000`
- **RPC URL**: Default `https://api.devnet.solana.com`

### 5. Payment Quote

The wizard will:
- Fetch the current price for your selected circuit
- Display payment details (recipient, amount in SOL and lamports)
- Ask for confirmation before proceeding

### 6. Payment & Job Creation

After confirmation:
- Execute the Solana payment transaction
- Display transaction signature
- Create the ZK proof job on the server
- Show job details (ID, status, circuit type)

### 7. Next Steps

Choose what to do next:
- Create another job
- Check the status of the job you just created
- Exit to main menu

## Example Workflow

```
╭─────────────────────────────────────╮
│     Welcome to Zyberlink CLI        │
│     Privacy-Preserving Compute      │
╰─────────────────────────────────────╯

? What would you like to do?
  > Create a ZK proof job
    Create an FHE computation job
    Check job status
    List available circuits
    Exit

? Select circuit type
  > ProofOfInnocence (10) - Proof that a prover is not part of a blacklist

✓ Selected: ProofOfInnocence (10)
  Category: Core
  Description: Proof that a prover is not part of a blacklist

? Enter path to witness file: ./example_witness.json

✓ Witness file loaded (0.13 KB)
  Commitment: 1a2b3c4d5e6f7890...
  Public inputs: 2 values

? Enter Solana keypair path: ~/.config/solana/id.json
✓ Keypair path: /home/user/.config/solana/id.json

? Server URL: http://localhost:3000
? Solana RPC URL: https://api.devnet.solana.com

✓ Creator pubkey: 7xKp...3mNq

Fetching price quote...

╭────────────────────────────────────╮
│ Payment Required                    │
│ Circuit: ProofOfInnocence          │
│ Price: 0.0500 SOL                  │
│ Recipient: ZYBR...xy12             │
╰────────────────────────────────────╯

? Proceed with payment? Yes

Processing payment...
✓ Payment successful!
  Transaction: 5KJp8qn3...

✓ Job created successfully!

╭────────────────────────────────────╮
│ Job Details                         │
│ ID: 1702200000000                  │
│ Status: active                     │
│ Circuit: ProofOfInnocence (10)     │
╰────────────────────────────────────╯

Check status: zyb zk status 1702200000000

? What would you like to do next?
  > Create another job
    Check this job's status
    Exit
```

## Dependencies

The wizard uses:
- **dialoguer** - Interactive prompts (Select, Input, Confirm)
- **console** - Terminal styling and colors
- **colored** - Text coloring
- **indicatif** - Progress indicators

## Files

- **src/ui/mod.rs** - Module exports
- **src/ui/wizard.rs** - Wizard implementation
- **example_witness.json** - Sample witness file for testing

## Benefits Over CLI Arguments

### Before (Manual CLI):
```bash
zyb zk create \
  --circuit-type 10 \
  --witness ./witness.json \
  --creator 7xKp...3mNq \
  --server http://localhost:3000 \
  --keypair ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com
```

### After (Interactive Wizard):
```bash
zyb wizard
# Follow the prompts
```

## Error Handling

The wizard provides helpful error messages for:
- Missing or invalid witness files
- Invalid keypair files
- Network connectivity issues
- Payment failures
- Server errors

## Future Enhancements

Planned features:
- FHE job creation wizard
- Batch job creation
- Job monitoring and polling
- Configuration profiles (save/load common settings)
- Witness file templates

## Tips

1. **Prepare your witness file** before starting the wizard
2. **Ensure your Solana keypair has sufficient SOL** for payment
3. **Use the example_witness.json** as a template for your own witness files
4. **Check server connectivity** before creating jobs
5. **Keep transaction signatures** for tracking payments

## Troubleshooting

### Wizard exits unexpectedly
- Check that your terminal supports interactive input
- Ensure you're not piping input to the command

### Payment fails
- Verify your keypair has sufficient SOL
- Check the RPC URL is reachable
- Confirm the keypair file is valid

### Witness file not found
- Use absolute paths or ensure the file is in your current directory
- Check file permissions

### Server connection errors
- Verify the server URL is correct
- Ensure the server is running
- Check firewall/network settings
