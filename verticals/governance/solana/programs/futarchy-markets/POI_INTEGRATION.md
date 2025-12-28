# Proof of Innocence (PoI) Integration

## Overview

El programa futarchy-markets integra **Proof of Innocence (PoI)** para prevenir insider trading y cumplir con regulaciones de sanctions/blacklists.

### ¿Qué es PoI?

PoI (Circuit 10) es un zero-knowledge proof que demuestra:
- Un usuario NO está en una blacklist de direcciones sancionadas
- Sin revelar la identidad del usuario
- Usando sparse merkle tree non-membership proofs

### ¿Por qué?

En prediction markets, es crítico prevenir:
1. **Insider Trading**: Usuarios con información privilegiada
2. **Sanctioned Entities**: Direcciones en listas de sanciones (OFAC, etc.)
3. **Money Laundering**: Direcciones conocidas de actividades ilícitas

---

## Architecture

### State Accounts

#### UserEligibility (94 bytes)

```rust
pub struct UserEligibility {
    pub user: Pubkey,                  // User wallet
    pub poi_job_id: u64,               // ZK job ID (Circuit 10)
    pub blacklist_root: [u8; 32],      // Blacklist merkle root used
    pub registered_at: i64,            // Registration timestamp
    pub expires_at: i64,               // Expiry (24h validity)
    pub is_active: bool,               // Active flag
    pub blacklist_version: u32,        // Blacklist version
    pub bump: u8,                      // PDA bump seed
}
```

**PDA Derivation:**
```
seeds = ["user_eligibility", user_pubkey]
```

---

## Instructions

### 1. RegisterUser

Registers a user with PoI verification.

**Accounts:**
```
0. [writable, signer] User
1. [writable]         UserEligibility PDA
2. []                 ZK-generator program
3. []                 System program
4. []                 Clock sysvar
```

**Parameters:**
- `proof: Vec<u8>` - Circuit 10 ZK proof (256 bytes)
- `public_inputs: Vec<u8>` - Public inputs (64 bytes):
  - `user_commitment` (32 bytes): Hash of user identity
  - `blacklist_root` (32 bytes): Merkle root of blacklist
- `blacklist_root: [u8; 32]` - Same as in public inputs
- `blacklist_version: u32` - Version of blacklist used

**Flow:**
1. Verify user is signer
2. Verify PoI proof via CPI to zk-generator (Circuit 10)
3. Create UserEligibility PDA
4. Initialize with 24h validity window
5. User can now place bets with Circuit 31

**Example (TypeScript):**
```typescript
import { generatePoIProof } from './circuits/poi';

// Generate PoI proof off-chain
const { proof, publicInputs } = await generatePoIProof({
  // Private inputs
  userIdentity: wallet.publicKey.toBuffer(),
  identitySecret: userSecret,
  blacklistNonMembershipProof: merkleProof,

  // Public inputs
  blacklistRoot: currentBlacklistRoot,
});

// Derive UserEligibility PDA
const [eligibilityPda] = await PublicKey.findProgramAddress(
  [Buffer.from("user_eligibility"), wallet.publicKey.toBuffer()],
  program.programId
);

// Register user
await program.methods
  .registerUser({
    proof,
    publicInputs,
    blacklistRoot: currentBlacklistRoot,
    blacklistVersion: 1,
  })
  .accounts({
    user: wallet.publicKey,
    userEligibility: eligibilityPda,
    zkGeneratorProgram: ZK_GENERATOR_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
    clock: SYSVAR_CLOCK_PUBKEY,
  })
  .rpc();

console.log("User registered with PoI");
console.log("Valid for 24 hours");
```

---

### 2. PlaceBet (with Circuit 31)

Place bet with PoI verification.

**Two Circuit Options:**
- **Circuit 30 (MarketBet)**: Standard bet, no PoI required
- **Circuit 31 (MarketBetWithPoI)**: Requires PoI verification

**Additional Account (Circuit 31 only):**
```
7. [] UserEligibility PDA
```

**Flow (Circuit 31):**
1. Verify ZK proof (Circuit 31)
2. **Derive UserEligibility PDA**
3. **Load UserEligibility state**
4. **Verify eligibility is valid:**
   - `is_active == true`
   - `current_time < expires_at`
5. If invalid → Error: `UserNotEligible`
6. Continue with bet placement

**Example (TypeScript):**
```typescript
// Generate bet proof with PoI
const { proof, publicInputs } = await generateMarketBetWithPoIProof({
  // Private inputs
  bettorWallet: wallet.publicKey.toBuffer(),
  betAmount,
  position,
  blinding,
  userIdentitySecret,

  // Public inputs
  marketId,
  betCommitment,
  maxBet,
  timestamp,
  blacklistRoot, // Must match UserEligibility
});

// Derive PDAs
const [positionPda] = await PublicKey.findProgramAddress(
  [
    Buffer.from("position"),
    wallet.publicKey.toBuffer(),
    Buffer.from([marketId]),
    betCommitment,
  ],
  program.programId
);

const [eligibilityPda] = await PublicKey.findProgramAddress(
  [Buffer.from("user_eligibility"), wallet.publicKey.toBuffer()],
  program.programId
);

// Place bet with Circuit 31
await program.methods
  .placeBet({
    marketId,
    betCommitment,
    proof,
    publicInputs,
    amount: betAmount,
    circuitType: 31, // MarketBetWithPoI
    encryptedBetAmount: null,
    side: null,
  })
  .accounts({
    user: wallet.publicKey,
    market: marketPda,
    position: positionPda,
    escrow: escrowPda,
    zkGeneratorProgram: ZK_GENERATOR_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
    clock: SYSVAR_CLOCK_PUBKEY,
    // Additional account for Circuit 31
    userEligibility: eligibilityPda,
  })
  .rpc();

console.log("Bet placed with PoI verification");
```

---

## Security Model

### Blacklist Management

The blacklist is a **sparse merkle tree** maintained off-chain:

```
Blacklist Root (on-chain)
    ↓
Sparse Merkle Tree (off-chain)
    ├─ Sanctioned addresses
    ├─ OFAC list
    ├─ Known exploiters
    └─ Insider addresses
```

**Update Flow:**
1. Blacklist coordinator updates tree off-chain
2. New merkle root published on-chain (via governance)
3. Users must re-register with new proof if:
   - Proof expired (>24h)
   - Blacklist version changed

### Proof Expiry

- **Validity Window**: 24 hours
- **Reason**: Blacklists update frequently
- **Renewal**: Users must call `RegisterUser` again with new proof

### Privacy Guarantees

PoI proofs provide:
1. **Zero-Knowledge**: Blacklist contents never revealed
2. **Non-Membership**: Proves user NOT in blacklist
3. **Unlinkability**: Multiple proofs from same user unlinkable
4. **Forward Secrecy**: Old proofs can't be used after expiry

---

## Comparison: Circuit 30 vs Circuit 31

| Feature | Circuit 30 (MarketBet) | Circuit 31 (MarketBetWithPoI) |
|---------|------------------------|-------------------------------|
| **PoI Required** | ❌ No | ✅ Yes |
| **RegisterUser** | Not needed | Required (24h validity) |
| **Additional Account** | None | UserEligibility PDA |
| **Gas Cost** | ~0.00005 SOL | ~0.00006 SOL (+20%) |
| **Privacy** | Bet commitment only | Full identity privacy |
| **Compliance** | None | Sanctions/blacklist compliant |
| **Use Case** | Simple markets | Regulated/high-value markets |

---

## Error Handling

### Common Errors

#### `UserNotEligible`
```
Error: "User not eligible (PoI verification required)"
```

**Causes:**
- UserEligibility account doesn't exist → Call `RegisterUser`
- Proof expired (>24h) → Re-register with new proof
- Blacklist version outdated → Re-register with current version
- Account marked inactive → Check if revoked

**Fix:**
```typescript
// Check if UserEligibility exists
const eligibility = await program.account.userEligibility.fetchNullable(eligibilityPda);

if (!eligibility) {
  console.log("User not registered - calling RegisterUser");
  await registerUser();
}

// Check if expired
const now = Math.floor(Date.now() / 1000);
if (now >= eligibility.expiresAt) {
  console.log("Eligibility expired - renewing");
  await registerUser(); // Re-register
}

// Check blacklist version
const currentVersion = await getBlacklistVersion();
if (eligibility.blacklistVersion < currentVersion) {
  console.log("Blacklist updated - renewing");
  await registerUser(); // Re-register
}
```

---

## Monitoring & Analytics

### On-Chain Metrics

Track these metrics for compliance:

```typescript
// Total registered users
const eligibilityAccounts = await program.account.userEligibility.all();
console.log("Registered users:", eligibilityAccounts.length);

// Active users (not expired)
const now = Math.floor(Date.now() / 1000);
const activeUsers = eligibilityAccounts.filter(acc =>
  acc.account.isActive && acc.account.expiresAt > now
);
console.log("Active eligible users:", activeUsers.length);

// Expiring soon (next 6h)
const expiringSoon = activeUsers.filter(acc =>
  acc.account.expiresAt < now + 6 * 60 * 60
);
console.log("Users needing renewal:", expiringSoon.length);
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_eligibility_validity() {
        let eligibility = UserEligibility {
            user: Pubkey::new_unique(),
            poi_job_id: 1,
            blacklist_root: [0; 32],
            registered_at: 1000,
            expires_at: 1000 + UserEligibility::VALIDITY_WINDOW,
            is_active: true,
            blacklist_version: 1,
            bump: 255,
        };

        // Valid within window
        assert!(eligibility.is_valid(1000 + 3600)); // 1h later

        // Invalid after expiry
        assert!(!eligibility.is_valid(1000 + UserEligibility::VALIDITY_WINDOW + 1));

        // Invalid if inactive
        let mut inactive = eligibility.clone();
        inactive.is_active = false;
        assert!(!inactive.is_valid(1000));
    }

    #[test]
    fn test_needs_renewal() {
        let eligibility = UserEligibility {
            registered_at: 1000,
            expires_at: 2000,
            blacklist_version: 1,
            // ... other fields
        };

        // Need renewal if expired
        assert!(eligibility.needs_renewal(2001, 1));

        // Need renewal if blacklist updated
        assert!(eligibility.needs_renewal(1500, 2));

        // No renewal needed
        assert!(!eligibility.needs_renewal(1500, 1));
    }
}
```

### Integration Tests

```typescript
describe('PoI Integration', () => {
  it('should prevent betting without PoI (Circuit 31)', async () => {
    // Try to bet with Circuit 31 without registering
    await expect(
      program.methods.placeBet({
        // ... bet params
        circuitType: 31,
      })
      .accounts({
        // Missing userEligibility
      })
      .rpc()
    ).to.be.rejectedWith('UserNotEligible');
  });

  it('should allow betting after registration', async () => {
    // 1. Register user
    await registerUser();

    // 2. Place bet with Circuit 31
    await program.methods.placeBet({
      circuitType: 31,
      // ... params
    })
    .accounts({
      userEligibility: eligibilityPda,
      // ... other accounts
    })
    .rpc();

    // Success
  });

  it('should reject expired eligibility', async () => {
    // Register user
    await registerUser();

    // Fast-forward 25 hours (beyond validity window)
    await sleep(25 * 60 * 60 * 1000);

    // Try to bet
    await expect(
      program.methods.placeBet({
        circuitType: 31,
        // ...
      }).rpc()
    ).to.be.rejectedWith('UserNotEligible');
  });
});
```

---

## Production Checklist

Before deploying to mainnet:

- [ ] **Blacklist Source**: Determine official blacklist source (OFAC, TRM, etc.)
- [ ] **Merkle Tree Infrastructure**: Deploy off-chain blacklist tree builder
- [ ] **Root Publishing**: Set up governance for blacklist root updates
- [ ] **Monitoring**: Deploy metrics dashboard for eligibility tracking
- [ ] **Auto-Renewal**: Build frontend notifications for expiring proofs
- [ ] **Proof Generation**: Optimize Circuit 10 & 31 proving time (<5s target)
- [ ] **Backup Plan**: Circuit 30 fallback if PoI service down
- [ ] **Legal Review**: Ensure compliance with jurisdiction regulations
- [ ] **Documentation**: User-facing docs on PoI registration flow

---

## Future Enhancements

### Planned Features

1. **Auto-Renewal Service**
   - Backend service to notify users of expiring proofs
   - One-click renewal in frontend

2. **Batch Registration**
   - Register multiple users in single transaction
   - Reduce gas costs for onboarding

3. **Delegated Provers**
   - Allow third-party services to generate proofs
   - User only signs final transaction

4. **Grace Period**
   - 1-hour grace period after expiry
   - Warning instead of hard rejection

5. **Tiered Compliance**
   - Different blacklist requirements by market tier
   - Low-stakes markets: Circuit 30 OK
   - High-stakes markets: Circuit 31 required

---

## Resources

- **Circuit 10 (PoI)**: `/circuits/poi/proof_of_innocence.circom`
- **Circuit 31 (MarketBetWithPoI)**: `/circuits/market/market_bet_poi.circom`
- **ZK-Generator**: `/programs/zk-generator/` (proof verification)
- **UserEligibility State**: `/programs/futarchy-markets/src/state/user_eligibility.rs`
- **RegisterUser Processor**: `/programs/futarchy-markets/src/processor/register_user.rs`

---

## Support

For questions about PoI integration:
- Review Circuit 10 documentation
- Check zk-generator CPI examples
- Test on devnet first
- Monitor logs for detailed error messages
