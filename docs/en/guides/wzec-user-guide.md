# wZEC Payment User Guide

A complete guide to using wrapped Zcash (wZEC) for privacy-focused payments in the ZyberLink marketplace.

## Table of Contents

- [What is wZEC?](#what-is-wzec)
- [Why Use wZEC?](#why-use-wzec)
- [Getting Started](#getting-started)
- [Making Your First wZEC Payment](#making-your-first-wzec-payment)
- [Understanding Token Accounts](#understanding-token-accounts)
- [Transaction Costs](#transaction-costs)
- [Troubleshooting](#troubleshooting)
- [FAQ](#faq)

## What is wZEC?

**wZEC (Wrapped Zcash)** is an SPL token on Solana that represents Zcash (ZEC), the privacy-focused cryptocurrency. Each wZEC token is backed 1:1 by actual ZEC held in reserve.

### Key Properties

- **Token Mint Address**: `7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf`
- **Decimals**: 8 (1 wZEC = 100,000,000 zatoshis)
- **Standard**: SPL Token (Solana Program Library)
- **Bridge**: Multi-signature custody solution

## Why Use wZEC?

### Benefits

1. **Privacy Alignment**: Zcash's privacy features align with ZyberLink's confidential computing mission
2. **Diversified Payment**: Alternative to SOL for users holding Zcash
3. **Cross-Chain Integration**: Enables Zcash ecosystem participation in Solana-based FHE computations
4. **Future-Ready**: Prepares infrastructure for shielded payments and advanced privacy features

### Trade-offs

| Feature | SOL Payment | wZEC Payment |
|---------|-------------|--------------|
| Transaction Speed | Instant | Instant |
| Setup Required | None | Token account needed |
| Privacy | Standard | Enhanced (Zcash-backed) |
| Fees | Lower | Slightly higher (token transfer) |
| Availability | Native | Requires token purchase |

## Getting Started

### Prerequisites

1. **Solana Wallet**: Phantom, Solflare, or any Solana-compatible wallet
2. **wZEC Tokens**: Purchase from supported exchanges
3. **SOL for Fees**: ~0.01 SOL for transaction fees

### Acquiring wZEC

**Option 1: DEX Swap**
```bash
# Use Raydium, Orca, or Jupiter to swap SOL for wZEC
# Search for token: 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf
```

**Option 2: Bridge from Zcash**
```bash
# Use the official wZEC bridge (if available)
# Deposit ZEC → Receive wZEC on Solana
```

**Option 3: Centralized Exchange**
```bash
# Some exchanges may offer direct wZEC withdrawals to Solana
# Check supported networks when withdrawing
```

### Verifying Your Balance

```bash
# Using Solana CLI
solana balance --token 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf

# Using SPL Token CLI
spl-token balance 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf
```

## Making Your First wZEC Payment

### Step 1: Access ZyberLink Marketplace

Navigate to the ZyberLink web application:
```
https://marketplace.zyberlink.io
```

### Step 2: Connect Your Wallet

1. Click **"Connect Wallet"** in the top right
2. Select your wallet provider (Phantom, Solflare, etc.)
3. Approve the connection request

### Step 3: Create a Job

1. Click **"Create New Job"**
2. Fill in job parameters:
   - **Operation**: Choose computation type (Add, Multiply, etc.)
   - **Encrypted Data**: Upload your FHE-encrypted input
   - **Server Key**: Provide TFHE server key
   - **Price**: Set payment amount (in zatoshis)

### Step 4: Select Payment Method

**This is where wZEC comes in!**

```
┌─────────────────────────────────────────────────────────┐
│  PAYMENT_METHOD: wZEC_SELECTED                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ○ SOL                          ● wZEC                 │
│    Native Solana                  Wrapped Zcash        │
│    Fast & low fees                Private payments     │
│    [RECOMMENDED]                                        │
│                                                         │
│  💡 PAYING_WITH_wZEC_TOKEN                             │
│  Requires wZEC token account. System will check         │
│  and create if needed.                                  │
└─────────────────────────────────────────────────────────┘
```

- Select the **wZEC** radio button
- The system automatically checks if you have a token account
- If missing, it will be created during the transaction

### Step 5: Review Transaction

```
┌─────────────────────────────────────────────────────────┐
│  Transaction Summary                                    │
├─────────────────────────────────────────────────────────┤
│  Payment Method:  wZEC                                  │
│  Amount:          5 wZEC (500,000,000 zatoshis)        │
│  Network Fee:     ~0.001 SOL                           │
│  Recipient:       Escrow PDA (auto-release)            │
│                                                         │
│  Accounts:                                              │
│    - Your wallet (signer)                               │
│    - Your wZEC token account (debit)                   │
│    - Escrow token account (credit)                     │
│    - Job PDA (job metadata)                            │
└─────────────────────────────────────────────────────────┘
```

### Step 6: Sign & Confirm

1. Click **"Create Job"**
2. Your wallet will prompt for approval
3. Review the transaction details carefully
4. Click **"Approve"** in your wallet
5. Wait for confirmation (~400ms on Solana)

### Step 7: Track Your Job

```
┌─────────────────────────────────────────────────────────┐
│  Job #12345 - Status: PENDING                          │
├─────────────────────────────────────────────────────────┤
│  Payment:     ✅ 5 wZEC locked in escrow               │
│  Provers:     ⏳ 1/3 claimed                           │
│  Consensus:   ⏳ Waiting for results                   │
│  Estimated:   ~30 seconds                               │
└─────────────────────────────────────────────────────────┘
```

Once consensus is reached, wZEC is automatically distributed to provers, and you receive your encrypted result!

## Understanding Token Accounts

### What is a Token Account?

In Solana's SPL token system, each user needs a dedicated **Associated Token Account (ATA)** for each token type they hold.

```
┌───────────────────────────────────────────────────────────┐
│  Your Wallet                                             │
│  HxL4...7Zf (base public key)                           │
│                                                           │
│  ├── SOL Balance: 2.5 SOL (native)                      │
│  │                                                        │
│  ├── wZEC Token Account: 8kJ2...3mP                     │
│  │   └── Balance: 10 wZEC                               │
│  │                                                        │
│  └── Other Token Accounts: ...                          │
└───────────────────────────────────────────────────────────┘
```

### Automatic Creation

**Good news:** ZyberLink automatically handles token account creation!

**When you pay with wZEC:**
1. System checks if you have a wZEC token account
2. If missing, it includes a creation instruction in the transaction
3. Account rent (~0.002 SOL) is deducted from your SOL balance
4. Payment proceeds normally

**Manual Creation (Optional):**
```bash
# Create wZEC token account manually
spl-token create-account 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf

# Check your token account address
spl-token accounts
```

### Account Rent

Token accounts require rent-exempt balance:
- **Cost**: ~0.002 SOL (one-time, refundable if account closed)
- **Purpose**: Prevents spam by requiring stake
- **Refund**: Close account to reclaim rent

## Transaction Costs

### Breakdown

```
Payment with wZEC:
├── Job Price:           5.0 wZEC  (set by you)
├── Network Fee:         ~0.001 SOL (Solana transaction)
├── Token Account Rent:  ~0.002 SOL (one-time, if needed)
└── Platform Fee:        10% of job price (0.5 wZEC)

Total Cost:
  - 5.0 wZEC (from your wZEC balance)
  - ~0.003 SOL (from your SOL balance for fees)
```

### Cost Comparison

| Scenario | SOL Payment | wZEC Payment |
|----------|-------------|--------------|
| Job Price | 5 SOL | 5 wZEC |
| Network Fee | ~0.0005 SOL | ~0.001 SOL |
| Token Account | N/A | ~0.002 SOL (first time) |
| Platform Fee | 0.5 SOL (10%) | 0.5 wZEC (10%) |
| **Total** | **5.0005 SOL** | **5 wZEC + 0.003 SOL** |

## Troubleshooting

### Issue: "Insufficient wZEC Balance"

**Problem:** Transaction fails with insufficient funds error.

**Solutions:**
1. Check your wZEC balance:
   ```bash
   spl-token balance 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf
   ```
2. Verify token account exists:
   ```bash
   spl-token accounts
   ```
3. Purchase more wZEC from a DEX or exchange

### Issue: "Token Account Creation Failed"

**Problem:** System can't create your wZEC token account.

**Solutions:**
1. Ensure you have at least 0.005 SOL for rent + fees
2. Check wallet permissions (approve token account creation)
3. Manually create account:
   ```bash
   spl-token create-account 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf
   ```

### Issue: "Transaction Timeout"

**Problem:** Transaction doesn't confirm within expected time.

**Solutions:**
1. Check Solana network status: https://status.solana.com
2. Increase transaction priority fee (advanced users)
3. Retry transaction after 30 seconds
4. Switch to SOL payment if urgent

### Issue: "Wrong Token Mint"

**Problem:** You sent tokens to the wrong address.

**Solutions:**
1. **Double-check mint address**: `7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf`
2. **Verify on Solana Explorer**: Search for the mint and confirm it's wZEC
3. **Don't use random tokens**: Only official wZEC is accepted

### Issue: "Escrow Not Releasing Funds"

**Problem:** Job completed but provers didn't receive payment.

**Solutions:**
1. Check job status on-chain
2. Verify consensus was reached (2-of-3 provers agreed)
3. Contact support with job ID
4. Check transaction signature on Solana Explorer

## FAQ

### Q: Is wZEC payment more private than SOL?

**A:** On Solana, all transactions are public regardless of token type. However, wZEC represents Zcash, which has strong privacy features. Future integrations may leverage Zcash's shielded pools for enhanced privacy.

### Q: Can I get my wZEC back if I cancel a job?

**A:** Yes! If you cancel a pending job (before provers claim it), your wZEC is returned from escrow to your token account minus network fees.

### Q: What happens if provers disagree on the result?

**A:** The consensus mechanism requires 2-of-3 (or configured threshold) matching results. If consensus fails:
1. Job is marked as failed
2. Your wZEC is refunded from escrow
3. Dishonest provers may be penalized

### Q: Can I pay partially in SOL and partially in wZEC?

**A:** Not currently. Each job must use a single payment method. However, you can create multiple jobs with different payment methods.

### Q: How do I convert wZEC back to ZEC?

**A:** Use the official wZEC bridge to unwrap your tokens:
1. Send wZEC to bridge contract
2. Provide your ZEC receiving address
3. Wait for bridge confirmation (varies by bridge)
4. Receive ZEC in your Zcash wallet

### Q: Is there a minimum wZEC amount for jobs?

**A:** No hard minimum, but consider:
- Network fees (~0.001 SOL) don't scale with payment size
- Very small jobs may not attract provers
- Recommended minimum: 0.1 wZEC (~$5-10)

### Q: Can I use testnet wZEC?

**A:** Yes! For testing:
- **Devnet mint**: Use faucet to get test wZEC
- **Testnet explorer**: https://explorer.solana.com?cluster=devnet
- **No real value**: Testnet tokens have no market value

### Q: What if the wZEC mint address changes?

**A:** The mint address is fixed in the protocol. If it changes:
1. System administrators will announce migration
2. Documentation will be updated
3. Old tokens may need to be swapped
4. Follow official channels for updates

## Next Steps

- **[wZEC Developer Guide](wzec-developer-guide.md)** - Integrate wZEC payments in your application
- **[wZEC API Reference](wzec-api-reference.md)** - Complete API documentation
- **[wZEC Architecture](../architecture/wzec-architecture.md)** - Technical deep dive
- **[Testing Guide](wzec-testing-guide.md)** - Run E2E tests locally

## Support

Need help with wZEC payments?

- **Documentation**: [docs.zyberlink.io](https://docs.zyberlink.io)
- **GitHub Issues**: [Report bugs](https://github.com/zyberlink/zyberlink/issues)
- **Discord**: [Join community](https://discord.gg/zyberlink)
- **Email**: support@zyberlink.io

---

**Last Updated**: 2025-11-21
**Version**: 1.0.0
**wZEC Mint**: `7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf`
