# wZEC Payment Architecture

Technical architecture documentation for the wZEC (Wrapped Zcash) payment integration in ZyberLink marketplace.

## Table of Contents

- [System Overview](#system-overview)
- [Component Architecture](#component-architecture)
- [Data Flow](#data-flow)
- [Payment Methods Comparison](#payment-methods-comparison)
- [Transaction Structure](#transaction-structure)
- [Security Model](#security-model)
- [Performance Considerations](#performance-considerations)
- [Future Enhancements](#future-enhancements)

## System Overview

### High-Level Architecture

```mermaid
graph TB
    subgraph "Client Layer"
        UI[Web UI / Mobile App]
        SDK[ZyberLink SDK]
    end

    subgraph "Backend Layer"
        API[API Server<br/>Rust/Actix-Web]
        Validator[Signature Validator]
        TxBuilder[Transaction Builder]
        DB[(PostgreSQL<br/>Job Data)]
    end

    subgraph "Blockchain Layer"
        Program[Solana Program<br/>Native Rust]
        SOL_Escrow[SOL Escrow PDA]
        Token_Escrow[wZEC Escrow PDA]
        SPL[SPL Token Program]
    end

    subgraph "Token Layer"
        WZEC[wZEC Mint<br/>7gGG...7Zf]
        ATA[User Token Account]
    end

    UI --> SDK
    SDK --> API
    API --> Validator
    Validator --> TxBuilder
    TxBuilder --> DB
    TxBuilder --> Program
    Program --> SOL_Escrow
    Program --> Token_Escrow
    Program --> SPL
    SPL --> WZEC
    WZEC --> ATA
```

### Key Design Principles

1. **Backwards Compatibility**: SOL payments continue to work unchanged
2. **Automatic Handling**: Token accounts created automatically if missing
3. **Type Safety**: Rust type system prevents payment method confusion
4. **Signature Security**: Ed25519 signatures with replay protection
5. **TFHE Support**: Handles 156 MB ServerKeys efficiently

## Component Architecture

### Frontend Components

```mermaid
graph LR
    A[PaymentMethodSelector] --> B{Payment Method}
    B -->|SOL| C[SOL Path]
    B -->|wZEC| D[Token Account Manager]
    D --> E[Check Token Account]
    E -->|Exists| F[Validate Balance]
    E -->|Missing| G[Auto-Create Flag]
    F --> H[Build Request]
    G --> H
    H --> I[API Client]
    I --> J[Sign Transaction]
    J --> K[Submit to Solana]
```

**PaymentMethodSelector.svelte:**
- Radio button UI for SOL vs wZEC
- Visual indicators and tooltips
- Accessibility-compliant (WCAG AA)
- Responsive design

**TokenAccountManager:**
- Checks for wZEC Associated Token Account
- Validates balance before transaction
- Flags missing accounts for auto-creation
- Error handling for edge cases

### Backend Architecture

```mermaid
graph TB
    API[POST /api/jobs/validate-and-build]
    API --> Parse[Parse & Validate Request]
    Parse --> SigVerify[Verify Ed25519 Signature]
    SigVerify --> NonceCheck[Check Nonce Uniqueness]
    NonceCheck --> PaymentRoute{Payment Method?}

    PaymentRoute -->|SOL| SOL_Build[Build SOL Transaction<br/>5 accounts]
    PaymentRoute -->|wZEC| WZEC_Build[Build wZEC Transaction<br/>9 accounts]

    SOL_Build --> SOL_Ix[CreateJob Instruction]
    WZEC_Build --> WZEC_Ix[CreateJobWithToken Instruction]

    SOL_Ix --> Serialize[Serialize Transaction]
    WZEC_Ix --> Serialize

    Serialize --> Store[Store in PostgreSQL]
    Store --> Response[Return {job_id, transaction}]
```

**Key Backend Files:**

```
blink-server/
├── src/
│   ├── api_handlers.rs          # HTTP endpoint handlers
│   ├── validators.rs            # Signature & input validation
│   ├── tx_builder.rs            # Transaction construction
│   ├── db/
│   │   ├── models.rs            # Database models
│   │   └── queries.rs           # SQL queries
│   └── main.rs                  # Server entry point
└── migrations/
    └── 20250120000007_add_wzec_payment_support.sql
```

### On-Chain Program Structure

```mermaid
graph TB
    Processor[Instruction Processor]
    Processor --> Route{Instruction Type}

    Route -->|0x00| Init[Initialize Config]
    Route -->|0x01| RegProver[Register Prover]
    Route -->|0x02| CreateJob[CreateJob<br/>SOL Payment]
    Route -->|0x08| CreateJobToken[CreateJobWithToken<br/>wZEC Payment]

    CreateJob --> Validate1[Validate Accounts]
    CreateJobToken --> Validate2[Validate Accounts + Token]

    Validate1 --> Transfer1[Transfer SOL<br/>Creator → Escrow]
    Validate2 --> Transfer2[Transfer wZEC<br/>Token Account → Escrow]

    Transfer1 --> Store1[Store Job State]
    Transfer2 --> Store2[Store Job State + Mint]
```

**Program Files:**

```
programs/zyberlink/
├── src/
│   ├── processor/
│   │   ├── mod.rs                        # Main processor
│   │   ├── create_job.rs                 # SOL payment (existing)
│   │   └── create_job_with_token.rs      # wZEC payment (NEW)
│   ├── instruction.rs                    # Instruction enum
│   └── lib.rs                            # Program entrypoint
```

## Data Flow

### wZEC Payment Flow

```mermaid
sequenceDiagram
    participant User
    participant Wallet
    participant Frontend
    participant Backend
    participant Solana
    participant Escrow

    User->>Frontend: Select wZEC payment
    Frontend->>Wallet: Check wZEC token account
    Wallet-->>Frontend: Account info

    alt Token account missing
        Frontend->>Frontend: Flag for auto-creation
    end

    Frontend->>User: Request signature
    User->>Wallet: Sign message
    Wallet-->>Frontend: Ed25519 signature (base58)

    Frontend->>Backend: POST /api/jobs/validate-and-build
    Backend->>Backend: Verify signature
    Backend->>Backend: Validate TFHE keys
    Backend->>Backend: Build CreateJobWithToken TX

    Backend-->>Frontend: {job_id, transaction}

    Frontend->>Wallet: Sign transaction
    Wallet-->>Frontend: Signed TX
    Frontend->>Solana: Submit transaction

    Solana->>Solana: Create token account (if needed)
    Solana->>Escrow: Transfer wZEC to escrow PDA
    Solana->>Solana: Store job state

    Solana-->>Frontend: TX signature
    Frontend->>Backend: POST /api/jobs/:id/confirm
    Backend-->>Frontend: Confirmation

    Frontend->>User: Job created successfully!
```

### Message Signing Flow

```mermaid
sequenceDiagram
    participant App
    participant Wallet
    participant Backend

    App->>App: Generate nonce<br/>(e2e_wzec_12345_1732104000)
    App->>App: Build message<br/>(create_job:12345:1732104000:nonce)
    App->>Wallet: signMessage(bytes)
    Wallet->>Wallet: Ed25519 sign with private key
    Wallet-->>App: signature (64 bytes)
    App->>App: Encode to base58 (88 chars)

    App->>Backend: {message, signature, ...}
    Backend->>Backend: Decode base58 to bytes
    Backend->>Backend: Extract pubkey from signature
    Backend->>Backend: Verify signature(message, pubkey)
    Backend->>Backend: Check timestamp < 5 min
    Backend->>Backend: Check nonce not used

    alt Validation passes
        Backend-->>App: Success
    else Validation fails
        Backend-->>App: 401 Unauthorized
    end
```

## Payment Methods Comparison

### SOL vs wZEC

```mermaid
graph LR
    subgraph "SOL Payment"
        direction TB
        A1[Creator] -->|Native SOL| B1[Escrow PDA]
        B1 -->|On completion| C1[Provers]
        style A1 fill:#8B5CF6
        style B1 fill:#8B5CF6
        style C1 fill:#8B5CF6
    end

    subgraph "wZEC Payment"
        direction TB
        A2[Creator Token Account] -->|SPL Token| B2[Token Escrow PDA]
        B2 -->|On completion| C2[Prover Token Accounts]
        style A2 fill:#06B6D4
        style B2 fill:#06B6D4
        style C2 fill:#06B6D4
    end
```

### Technical Differences

| Aspect | SOL Payment | wZEC Payment |
|--------|-------------|--------------|
| **Instruction** | CreateJob (0x02) | CreateJobWithToken (0x08) |
| **Accounts** | 5 | 9 |
| **Token Account** | Not required | Required (auto-created) |
| **Escrow Type** | SOL PDA | SPL Token PDA |
| **Transfer Program** | System Program | SPL Token Program |
| **Price Unit** | Lamports | Zatoshis |
| **Mint Address** | N/A | 7gGG...7Zf |

### Account Structures

**SOL Payment (5 accounts):**
```rust
pub struct CreateJobAccounts<'a> {
    pub creator: &'a AccountInfo<'a>,        // Signer
    pub job: &'a AccountInfo<'a>,            // PDA (writable)
    pub config: &'a AccountInfo<'a>,         // PDA (readonly)
    pub escrow: &'a AccountInfo<'a>,         // PDA (writable)
    pub system_program: &'a AccountInfo<'a>, // Program
}
```

**wZEC Payment (9 accounts):**
```rust
pub struct CreateJobWithTokenAccounts<'a> {
    pub creator: &'a AccountInfo<'a>,              // Signer
    pub job: &'a AccountInfo<'a>,                  // PDA (writable)
    pub config: &'a AccountInfo<'a>,               // PDA (writable)
    pub token_escrow: &'a AccountInfo<'a>,         // PDA (writable)
    pub creator_token_account: &'a AccountInfo<'a>, // ATA (writable)
    pub token_mint: &'a AccountInfo<'a>,           // Mint (readonly)
    pub system_program: &'a AccountInfo<'a>,       // Program
    pub token_program: &'a AccountInfo<'a>,        // Program
    pub rent: &'a AccountInfo<'a>,                 // Sysvar
}
```

## Transaction Structure

### wZEC Transaction Anatomy

```
Transaction {
  signatures: [null],  // Unsigned, filled by wallet
  message: {
    header: {
      num_required_signatures: 1,
      num_readonly_signed_accounts: 0,
      num_readonly_unsigned_accounts: 4
    },
    account_keys: [
      creator_pubkey,           // Signer
      job_pda,                  // Writable
      config_pda,               // Writable
      token_escrow_pda,         // Writable
      creator_token_account,    // Writable
      wzec_mint,                // Readonly
      system_program,           // Readonly
      token_program,            // Readonly
      rent_sysvar              // Readonly
    ],
    recent_blockhash: "...",
    instructions: [
      {
        program_id_index: 7,  // Token Program
        accounts: [...],      // Account indices
        data: [0x08, ...]     // CreateJobWithToken data
      }
    ]
  }
}
```

### PDA Derivation

```rust
// Job PDA
let (job_pda, job_bump) = Pubkey::find_program_address(
    &[
        b"job",
        creator.as_ref(),
        &job_id.to_le_bytes()
    ],
    &program_id
);

// SOL Escrow PDA
let (sol_escrow_pda, escrow_bump) = Pubkey::find_program_address(
    &[
        b"escrow",
        job_pda.as_ref()
    ],
    &program_id
);

// Token Escrow PDA
let (token_escrow_pda, token_bump) = Pubkey::find_program_address(
    &[
        b"token_escrow",
        job_pda.as_ref(),
        token_mint.as_ref()
    ],
    &program_id
);

// Associated Token Account (ATA)
let ata = spl_associated_token_account::get_associated_token_address(
    &creator,
    &wzec_mint
);
```

## Security Model

### Authentication

```mermaid
graph TD
    Request[API Request]
    Request --> Extract[Extract: message, signature, pubkey]
    Extract --> Decode[Decode signature from base58]
    Decode --> Verify{Verify Ed25519<br/>signature?}
    Verify -->|Invalid| Reject1[401 Unauthorized]
    Verify -->|Valid| ParseMsg[Parse message]
    ParseMsg --> CheckTime{Timestamp within<br/>5 minutes?}
    CheckTime -->|No| Reject2[401 Expired]
    CheckTime -->|Yes| CheckNonce{Nonce already<br/>used?}
    CheckNonce -->|Yes| Reject3[401 Replay]
    CheckNonce -->|No| Accept[Accept Request]
```

### Signature Format (CRITICAL)

**Correct Format (base58):**
```javascript
const signature = bs58.encode(signatureBytes);
// Output: "5J7XqG3K8H9L..." (88 chars)
```

**Incorrect Format (base64):**
```javascript
const signature = Buffer.from(signatureBytes).toString('base64');
// Output: "BQYHCAkKCw..." (wrong!)
```

### Anti-Replay Protection

```rust
// Check timestamp
let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
let request_time = parse_timestamp_from_message(&message)?;

if current_time - request_time > 300 {  // 5 minutes
    return Err("Timestamp expired");
}

// Check nonce uniqueness
if db.nonce_exists(&nonce).await? {
    return Err("Nonce already used");
}

// Store nonce
db.store_nonce(&nonce).await?;
```

### Token Account Security

**Automatic Creation Safety:**
```rust
// Check if creator has enough SOL for rent
let creator_balance = rpc_client.get_balance(&creator)?;
let rent_exempt_balance = rent.minimum_balance(TOKEN_ACCOUNT_SIZE);

if creator_balance < rent_exempt_balance {
    return Err("Insufficient SOL for token account rent");
}

// Create ATA instruction (idempotent)
let create_ata_ix = create_associated_token_account_idempotent(
    &creator,  // payer
    &creator,  // owner
    &token_mint
);
```

## Performance Considerations

### Payload Sizes

```
Request Components:
├── JSON overhead:       ~500 bytes
├── encrypted_data:      ~1 MB (1,048,576 bytes max)
├── server_key:          ~156 MB (typical TFHE ServerKey)
├── Other fields:        ~300 bytes
└── Total:               ~157 MB

Response:
├── JSON overhead:       ~100 bytes
├── transaction:         ~1-2 KB (serialized TX)
└── Total:               ~2 KB
```

### Network Optimization

```rust
// Backend: Stream large payloads
#[post("/api/jobs/validate-and-build")]
async fn create_job(
    payload: web::Payload,
    max_size: web::Data<MaxSize>
) -> Result<HttpResponse> {
    // Stream payload instead of loading into memory
    let body = payload
        .limit(max_size.0)  // 200 MB limit
        .fold(BytesMut::new(), |mut acc, chunk| {
            acc.extend_from_slice(&chunk?);
            Ok::<_, PayloadError>(acc)
        })
        .await?;

    // Process...
}
```

### Database Indexing

```sql
-- Optimize lookups
CREATE INDEX idx_temp_job_data_payment_method
ON temp_job_data(payment_method);

CREATE INDEX idx_temp_job_data_nonce
ON temp_job_data(nonce);

CREATE INDEX idx_temp_job_data_creator_timestamp
ON temp_job_data(creator_pubkey, created_at DESC);
```

### Transaction Cost Analysis

```
SOL Payment:
├── Transaction fee:     ~0.0005 SOL
├── Escrow rent:         ~0.002 SOL (refundable)
└── Total upfront:       ~0.0025 SOL

wZEC Payment:
├── Transaction fee:     ~0.001 SOL (more accounts)
├── Token escrow rent:   ~0.002 SOL (refundable)
├── ATA rent (if new):   ~0.002 SOL (one-time)
└── Total upfront:       ~0.005 SOL (worst case)
```

## Future Enhancements

### Phase 1: Current State

- [x] Dual payment support (SOL/wZEC)
- [x] Automatic token account creation
- [x] Base58 signature format
- [x] 156 MB TFHE ServerKey support
- [x] E2E test coverage

### Phase 2: Optimization (Q1 2025)

- [ ] Batch transaction support
- [ ] Token account pre-creation UI
- [ ] Signature caching
- [ ] WebSocket real-time updates
- [ ] GraphQL API

### Phase 3: Advanced Features (Q2 2025)

- [ ] Multi-token payment support
- [ ] Partial payments / installments
- [ ] wZEC staking for provers
- [ ] Cross-chain bridges (ZEC ↔ wZEC)
- [ ] Shielded payment integration

### Phase 4: Privacy Enhancements (Q3 2025)

- [ ] Zcash shielded pool integration
- [ ] Private transaction amounts
- [ ] Zero-knowledge payment proofs
- [ ] Confidential job pricing
- [ ] Anonymous prover selection

## Diagram: Complete System Flow

```mermaid
graph TB
    subgraph "Frontend"
        UI[User Interface]
        PM[Payment Method Selector]
        TAM[Token Account Manager]
        MS[Message Signer]
    end

    subgraph "Backend API"
        EP[API Endpoint]
        Val[Validator]
        TB[Transaction Builder]
        DB[(PostgreSQL)]
    end

    subgraph "Solana Blockchain"
        Program[ZyberLink Program]
        SOL_E[SOL Escrow PDA]
        WZEC_E[wZEC Escrow PDA]
        SPL[SPL Token Program]
    end

    subgraph "Token Infrastructure"
        Mint[wZEC Mint]
        ATA1[Creator ATA]
        ATA2[Prover ATAs]
    end

    UI --> PM
    PM -->|wZEC| TAM
    TAM --> MS
    MS --> EP
    EP --> Val
    Val --> TB
    TB --> DB
    TB --> Program
    Program -->|SOL| SOL_E
    Program -->|wZEC| WZEC_E
    WZEC_E --> SPL
    SPL --> Mint
    Mint --> ATA1
    Mint --> ATA2
```

## Related Documentation

- **[User Guide](../guides/wzec-user-guide.md)** - End-user documentation
- **[Developer Guide](../guides/wzec-developer-guide.md)** - Integration guide
- **[API Reference](../guides/wzec-api-reference.md)** - Complete API docs
- **[Testing Guide](../guides/wzec-testing-guide.md)** - Testing procedures

---

**Last Updated**: 2025-11-21
**Architecture Version**: 1.0.0
**wZEC Mint**: `7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf`
