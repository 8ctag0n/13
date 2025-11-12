# CypherLink Mobile Wallet - Plan de Implementación Completo

## CONTEXTO

Esta wallet mobile necesita integración COMPLETA con Solana para:
1. Pagar prover fees (SOL balance real)
2. Crear jobs on-chain (transacción Solana)
3. Interactuar con programa CypherLink (marketplace)
4. Polling de job status on-chain
5. Manejo de claves derivadas (Zcash + Solana desde una seed)

## STACK TECNOLÓGICO CONFIRMADO

### Mobile
- **Framework:** Flutter 3.24+
- **Language:** Dart 3.5+
- **State Management:** Riverpod 2.5+
- **Navigation:** go_router 14.x
- **Secure Storage:** flutter_secure_storage 9.x
- **FFI:** flutter_rust_bridge (via uniffi-rs)

### Rust FFI
- **Core:** cypherlink-sdk (reusando código existente)
- **Solana:** solana-sdk 2.1, solana-client 2.1
- **Crypto:** bip39, bip32, ed25519-dalek, x25519-dalek
- **FFI Binding:** uniffi 0.27+
- **Async:** tokio 1.47

### Backend (ya existente)
- Solana Program (deployed)
- Witness Storage (HTTP server)
- Prover Node (desktop)

## DECISIONES TÉCNICAS CLAVE

### 1. Solana Transaction Signing
**Decisión:** Embedded keypair con solana-sdk completo en Rust FFI

**Razón:**
- Cumple "one seed phrase" UX
- Reutiliza `MarketplaceClient` existente
- Control total sobre transactions
- No dependencias externas (MWA)

**Trade-off aceptado:**
- Binary size más grande (~5-8 MB adicionales)
- Compilación más lenta
- Pero: mejor UX, menos complejidad

### 2. Balance Management
**Enfoque dual:**

**Devnet (demo):**
```dart
// Auto-airdrop al crear wallet
if (network == Network.devnet) {
  await walletClient.requestAirdrop(1_000_000_000); // 1 SOL
}
```

**Mainnet (futuro):**
```dart
// Usuario deposita manualmente
showQRCode(walletClient.getSolanaAddress());
```

### 3. Job Status Polling
**Implementación:**

```rust
// Rust FFI
pub async fn poll_job_status(
    &self,
    job_pda: String,
) -> Result<JobStatus> {
    let job_pubkey = Pubkey::from_str(&job_pda)?;

    loop {
        let account = self.rpc_client.get_account(&job_pubkey).await?;
        let job: JobAccount = borsh::from_slice(&account.data)?;

        match job.status {
            JobStatus::Completed => {
                return Ok(JobStatus::Completed);
            }
            JobStatus::Failed | JobStatus::Cancelled => {
                return Err(anyhow!("Job failed or cancelled"));
            }
            _ => {
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }
}
```

```dart
// Flutter
Stream<JobStatus> watchJobStatus(String jobPda) async* {
  while (true) {
    final status = await walletApi.getJobStatus(jobPda);
    yield status;

    if (status.isTerminal) break;

    await Future.delayed(Duration(seconds: 3));
  }
}
```

### 4. RPC Endpoints

**Configuración:**
```rust
pub enum Network {
    Devnet,
    Mainnet,
}

impl Network {
    pub fn rpc_url(&self) -> String {
        match self {
            Network::Devnet => "https://api.devnet.solana.com".to_string(),
            // Opción Premium:
            // Network::Devnet => "https://devnet.helius-rpc.com/?api-key=XXX".to_string(),
            Network::Mainnet => "https://api.mainnet-beta.solana.com".to_string(),
        }
    }
}
```

**Recomendación para demo:**
- Usar Helius free tier (100k req/day)
- Mejor performance que public RPC
- Gratis para hackathon/demo

## ESTRUCTURA DEL PROYECTO

```
cypherlink-wallet/
├── lib/                          # Flutter app
│   ├── main.dart
│   ├── app.dart                  # App root with Riverpod
│   ├── models/
│   │   ├── wallet_state.dart     # Wallet model
│   │   ├── job.dart              # Job model
│   │   └── transaction.dart      # Transaction model
│   ├── providers/
│   │   ├── wallet_provider.dart  # Wallet state
│   │   ├── jobs_provider.dart    # Jobs state
│   │   └── settings_provider.dart
│   ├── screens/
│   │   ├── home_screen.dart      # Main screen (balances)
│   │   ├── send_screen.dart      # Create job flow
│   │   ├── job_status_screen.dart # Job tracking
│   │   ├── receive_screen.dart   # Show QR for deposits
│   │   └── settings_screen.dart  # Seed backup, network
│   ├── widgets/
│   │   ├── balance_card.dart
│   │   ├── job_card.dart
│   │   └── transaction_list.dart
│   └── services/
│       └── wallet_api.dart       # FFI wrapper
├── rust/                         # Rust FFI library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                # uniffi exports
│   │   ├── wallet/
│   │   │   ├── mod.rs
│   │   │   ├── keys.rs           # BIP-39/32 key derivation
│   │   │   └── storage.rs        # Secure storage helpers
│   │   ├── solana/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs         # MarketplaceClient wrapper
│   │   │   └── transaction.rs    # Tx builders
│   │   ├── zcash/
│   │   │   ├── mod.rs
│   │   │   └── mock.rs           # Mock Zcash for demo
│   │   ├── cypherlink/
│   │   │   ├── mod.rs
│   │   │   ├── witness.rs        # Witness encryption
│   │   │   └── storage_client.rs # Backend HTTP client
│   │   └── ffi.udl               # uniffi interface definition
│   └── build.rs                  # uniffi build script
├── android/                      # Android config
├── ios/                          # iOS config (future)
├── pubspec.yaml                  # Flutter dependencies
└── README.md
```

## FASES DE IMPLEMENTACIÓN

---

### FASE 0: Setup Inicial (Día 1) - 6-8h

**Objetivo:** Proyecto Flutter + Rust FFI funcionando con "Hello World"

#### Tasks:

**0.1 Inicializar proyecto Flutter**
```bash
cd cypherlink-wallet
flutter create . --org com.cypherlink --platforms android
flutter pub add riverpod flutter_riverpod go_router flutter_secure_storage
```

**0.2 Configurar Rust FFI**
```bash
cd rust
cargo add uniffi --features cli
cargo add solana-sdk solana-client --features 2.1
cargo add bip39 bip32 ed25519-dalek
cargo add cypherlink-sdk --path ../../sdk
```

**0.3 Crear uniffi interface básica**
```idl
// rust/src/ffi.udl
namespace cypherlink_wallet {
    string hello_world();
};
```

**0.4 Build y test FFI**
```bash
# Generate bindings
cargo run --features=uniffi/cli \
    --bin uniffi-bindgen generate \
    src/ffi.udl \
    --language kotlin \
    --out-dir ../android/app/src/main/kotlin/

# Test from Flutter
flutter run
```

**Entregable:** App Flutter que llama función Rust y muestra "Hello from Rust"

**Tiempo:** 6-8 horas

---

### FASE 1: Wallet Core (Días 2-4) - 20-24h

**Objetivo:** Generación y manejo de claves (Zcash + Solana)

#### Tasks:

**1.1 BIP-39/32 Key Derivation (Rust)**

Archivo: `rust/src/wallet/keys.rs`

```rust
use bip39::{Mnemonic, Language};
use bip32::{XPrv, DerivationPath};
use solana_sdk::signature::Keypair;
use ed25519_dalek::SigningKey;

pub struct DerivedKeys {
    pub solana_keypair: Keypair,
    pub zcash_spending_key: [u8; 32], // Mock for now
}

pub fn generate_mnemonic() -> Mnemonic {
    Mnemonic::generate(Language::English, 12)
}

pub fn derive_keys_from_mnemonic(mnemonic: &str) -> Result<DerivedKeys> {
    let mnemonic = Mnemonic::parse_normalized(mnemonic)?;
    let seed = mnemonic.to_seed("");

    // Derive Solana keypair (m/44'/501'/0'/0')
    let solana_path = "m/44'/501'/0'/0'".parse::<DerivationPath>()?;
    let solana_xprv = XPrv::derive_from_path(&seed, &solana_path)?;
    let solana_keypair = Keypair::from_bytes(&solana_xprv.private_key().to_bytes())?;

    // Derive Zcash key (m/44'/133'/0'/0) - mock for demo
    let zcash_path = "m/44'/133'/0'/0".parse::<DerivationPath>()?;
    let zcash_xprv = XPrv::derive_from_path(&seed, &zcash_path)?;
    let zcash_spending_key = zcash_xprv.private_key().to_bytes();

    Ok(DerivedKeys {
        solana_keypair,
        zcash_spending_key,
    })
}
```

**1.2 Wallet State Management (Rust)**

Archivo: `rust/src/wallet/mod.rs`

```rust
use cypherlink_sdk::MarketplaceClient;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::pubkey::Pubkey;

pub struct Wallet {
    mnemonic: String,
    solana_keypair: Keypair,
    zcash_spending_key: [u8; 32],
    marketplace_client: MarketplaceClient,
}

impl Wallet {
    pub fn create_new(network: Network, program_id: Pubkey) -> Result<Self> {
        let mnemonic = generate_mnemonic();
        let keys = derive_keys_from_mnemonic(&mnemonic.to_string())?;

        let marketplace_client = MarketplaceClient::new(
            network.rpc_url(),
            program_id,
        );

        Ok(Self {
            mnemonic: mnemonic.to_string(),
            solana_keypair: keys.solana_keypair,
            zcash_spending_key: keys.zcash_spending_key,
            marketplace_client,
        })
    }

    pub fn from_mnemonic(
        mnemonic: String,
        network: Network,
        program_id: Pubkey,
    ) -> Result<Self> {
        let keys = derive_keys_from_mnemonic(&mnemonic)?;

        let marketplace_client = MarketplaceClient::new(
            network.rpc_url(),
            program_id,
        );

        Ok(Self {
            mnemonic,
            solana_keypair: keys.solana_keypair,
            zcash_spending_key: keys.zcash_spending_key,
            marketplace_client,
        })
    }

    pub fn get_mnemonic(&self) -> String {
        self.mnemonic.clone()
    }

    pub fn get_solana_address(&self) -> String {
        self.solana_keypair.pubkey().to_string()
    }

    pub fn get_solana_balance(&self) -> Result<u64> {
        self.marketplace_client
            .rpc_client
            .get_balance(&self.solana_keypair.pubkey())
            .map_err(Into::into)
    }
}
```

**1.3 FFI Exports (Rust)**

Archivo: `rust/src/ffi.udl`

```idl
namespace cypherlink_wallet {
    WalletHandle create_wallet(Network network, string program_id);
    WalletHandle restore_wallet(string mnemonic, Network network, string program_id);
};

enum Network {
    "Devnet",
    "Mainnet",
};

interface WalletHandle {
    constructor(Network network, string program_id);

    string get_mnemonic();
    string get_solana_address();
    string get_zcash_address(); // Mock
    u64 get_solana_balance();
    f64 get_zcash_balance(); // Mock, returns 10.0

    // Devnet only
    string request_airdrop(u64 lamports);
};
```

**1.4 Flutter Wallet Provider**

Archivo: `lib/providers/wallet_provider.dart`

```dart
import 'package:riverpod/riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import '../services/wallet_api.dart'; // FFI wrapper

final walletProvider = StateNotifierProvider<WalletNotifier, WalletState>((ref) {
  return WalletNotifier();
});

class WalletState {
  final bool isInitialized;
  final String? solanaAddress;
  final String? zcashAddress;
  final double solBalance;
  final double zcashBalance;

  WalletState({
    this.isInitialized = false,
    this.solanaAddress,
    this.zcashAddress,
    this.solBalance = 0.0,
    this.zcashBalance = 0.0,
  });

  WalletState copyWith({
    bool? isInitialized,
    String? solanaAddress,
    String? zcashAddress,
    double? solBalance,
    double? zcashBalance,
  }) {
    return WalletState(
      isInitialized: isInitialized ?? this.isInitialized,
      solanaAddress: solanaAddress ?? this.solanaAddress,
      zcashAddress: zcashAddress ?? this.zcashAddress,
      solBalance: solBalance ?? this.solBalance,
      zcashBalance: zcashBalance ?? this.zcashBalance,
    );
  }
}

class WalletNotifier extends StateNotifier<WalletState> {
  WalletNotifier() : super(WalletState());

  final _storage = FlutterSecureStorage();
  WalletHandle? _walletHandle;

  Future<void> createNewWallet() async {
    final wallet = WalletApi.createWallet(Network.devnet, PROGRAM_ID);
    final mnemonic = wallet.getMnemonic();

    // Save to secure storage
    await _storage.write(key: 'mnemonic', value: mnemonic);

    // Auto-airdrop in devnet
    await wallet.requestAirdrop(1000000000); // 1 SOL

    _walletHandle = wallet;
    await _updateState();
  }

  Future<void> restoreWallet(String mnemonic) async {
    final wallet = WalletApi.restoreWallet(mnemonic, Network.devnet, PROGRAM_ID);

    await _storage.write(key: 'mnemonic', value: mnemonic);

    _walletHandle = wallet;
    await _updateState();
  }

  Future<void> loadWallet() async {
    final mnemonic = await _storage.read(key: 'mnemonic');
    if (mnemonic != null) {
      await restoreWallet(mnemonic);
    }
  }

  Future<void> refreshBalances() async {
    await _updateState();
  }

  Future<void> _updateState() async {
    if (_walletHandle == null) return;

    final solBalance = await _walletHandle!.getSolanaBalance();
    final zcashBalance = await _walletHandle!.getZcashBalance();

    state = state.copyWith(
      isInitialized: true,
      solanaAddress: _walletHandle!.getSolanaAddress(),
      zcashAddress: _walletHandle!.getZcashAddress(),
      solBalance: solBalance / 1e9, // lamports to SOL
      zcashBalance: zcashBalance,
    );
  }

  WalletHandle? get handle => _walletHandle;
}
```

**1.5 Home Screen UI**

Archivo: `lib/screens/home_screen.dart`

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../providers/wallet_provider.dart';

class HomeScreen extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final walletState = ref.watch(walletProvider);

    if (!walletState.isInitialized) {
      return _buildOnboarding(context, ref);
    }

    return Scaffold(
      appBar: AppBar(title: Text('CypherWallet')),
      body: RefreshIndicator(
        onRefresh: () => ref.read(walletProvider.notifier).refreshBalances(),
        child: ListView(
          padding: EdgeInsets.all(16),
          children: [
            _buildBalanceCard(
              'Solana',
              walletState.solBalance,
              'SOL',
              walletState.solanaAddress ?? '',
            ),
            SizedBox(height: 16),
            _buildBalanceCard(
              'Zcash',
              walletState.zcashBalance,
              'ZEC',
              walletState.zcashAddress ?? '',
            ),
            SizedBox(height: 24),
            ElevatedButton.icon(
              icon: Icon(Icons.send),
              label: Text('Send Private Transaction'),
              onPressed: () => Navigator.pushNamed(context, '/send'),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildBalanceCard(String name, double balance, String symbol, String address) {
    return Card(
      child: Padding(
        padding: EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(name, style: TextStyle(fontSize: 14, color: Colors.grey)),
            SizedBox(height: 8),
            Text(
              '${balance.toStringAsFixed(4)} $symbol',
              style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold),
            ),
            SizedBox(height: 8),
            Text(
              address,
              style: TextStyle(fontSize: 10, fontFamily: 'monospace'),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildOnboarding(BuildContext context, WidgetRef ref) {
    return Scaffold(
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text('Welcome to CypherWallet', style: TextStyle(fontSize: 24)),
            SizedBox(height: 32),
            ElevatedButton(
              onPressed: () => ref.read(walletProvider.notifier).createNewWallet(),
              child: Text('Create New Wallet'),
            ),
            SizedBox(height: 16),
            TextButton(
              onPressed: () => _showRestoreDialog(context, ref),
              child: Text('Restore from Seed'),
            ),
          ],
        ),
      ),
    );
  }
}
```

**Entregables Fase 1:**
- ✅ Wallet crea/restaura desde seed phrase
- ✅ Deriva Solana + Zcash keys correctamente
- ✅ Muestra balances (SOL real, ZEC mock)
- ✅ Airdrop funciona en devnet
- ✅ UI básica con balance cards

**Tiempo:** 20-24 horas

---

### FASE 2: Witness Generation (Días 5-6) - 10-14h

**Objetivo:** Generar y encriptar witness (mock Zcash para demo)

#### Tasks:

**2.1 Mock Zcash Witness (Rust)**

Archivo: `rust/src/zcash/mock.rs`

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct MockOrchardWitness {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub memo: String,
    pub timestamp: i64,
}

impl MockOrchardWitness {
    pub fn generate_for_transaction(
        recipient: &str,
        amount: u64,
        memo: &str,
    ) -> Self {
        Self {
            sender: "mock_sender_address".to_string(),
            recipient: recipient.to_string(),
            amount,
            memo: memo.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }
}
```

**2.2 Witness Encryption (Rust)**

Archivo: `rust/src/cypherlink/witness.rs`

```rust
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct EncryptedWitness {
    pub ephemeral_public_key: [u8; 32],
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

pub fn encrypt_witness(
    witness_data: &[u8],
    prover_encryption_pubkey: &[u8; 32],
) -> Result<EncryptedWitness> {
    // Generate ephemeral keypair
    let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_public = PublicKey::from(&ephemeral_secret);

    // Derive shared secret
    let prover_pubkey = PublicKey::from(*prover_encryption_pubkey);
    let shared_secret = ephemeral_secret.diffie_hellman(&prover_pubkey);

    // Encrypt witness
    let cipher = ChaCha20Poly1305::new(shared_secret.as_bytes().into());
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher.encrypt(&nonce, witness_data)
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    Ok(EncryptedWitness {
        ephemeral_public_key: *ephemeral_public.as_bytes(),
        nonce: nonce.into(),
        ciphertext,
    })
}
```

**2.3 Witness Upload (Rust)**

Archivo: `rust/src/cypherlink/storage_client.rs`

```rust
use reqwest::Client;
use blake2::{Blake2b512, Digest};

pub struct WitnessStorageClient {
    backend_url: String,
    http_client: Client,
}

impl WitnessStorageClient {
    pub fn new(backend_url: String) -> Self {
        Self {
            backend_url,
            http_client: Client::new(),
        }
    }

    pub async fn upload_witness(
        &self,
        encrypted_witness: &EncryptedWitness,
    ) -> Result<[u8; 32]> {
        let witness_bytes = borsh::to_vec(encrypted_witness)?;

        // Compute commitment
        let mut hasher = Blake2b512::new();
        hasher.update(&witness_bytes);
        let hash_result = hasher.finalize();
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&hash_result[..32]);

        // Upload to backend
        let response = self.http_client
            .post(format!("{}/witness", self.backend_url))
            .header("Content-Type", "application/octet-stream")
            .body(witness_bytes)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to upload witness: {}", response.status());
        }

        Ok(commitment)
    }

    pub async fn download_proof(
        &self,
        commitment: &[u8; 32],
    ) -> Result<Vec<u8>> {
        let commitment_hex = hex::encode(commitment);

        let response = self.http_client
            .get(format!("{}/proof/{}", self.backend_url, commitment_hex))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.bytes().await?.to_vec())
        } else {
            anyhow::bail!("Proof not found or not ready yet");
        }
    }
}
```

**2.4 FFI Exports**

```idl
// Add to ffi.udl
interface WalletHandle {
    // ... existing methods ...

    // Witness generation (mock)
    bytes generate_mock_witness(string recipient, u64 amount_zats, string memo);

    // Encrypt witness for prover
    bytes encrypt_witness(bytes witness_data, bytes prover_pubkey);

    // Upload to storage backend
    bytes upload_encrypted_witness(bytes encrypted_witness, string backend_url);
};
```

**Entregables Fase 2:**
- ✅ Mock witness generation funciona
- ✅ Witness encryption (X25519 + ChaCha20)
- ✅ Upload a backend storage funciona
- ✅ Commitment calculation correcta

**Tiempo:** 10-14 horas

---

### FASE 3: CypherLink Job Creation (Días 7-9) - 24-30h

**Objetivo:** Crear job on-chain desde mobile

#### Tasks:

**3.1 Solana Transaction Builder (Rust)**

Archivo: `rust/src/solana/transaction.rs`

```rust
use cypherlink_sdk::MarketplaceClient;
use cypherlink_types::CircuitType;
use solana_sdk::{
    signature::{Keypair, Signature, Signer},
    pubkey::Pubkey,
};

pub struct JobCreator {
    marketplace_client: MarketplaceClient,
    wallet_keypair: Keypair,
    job_counter: u64,
}

impl JobCreator {
    pub fn new(
        marketplace_client: MarketplaceClient,
        wallet_keypair: Keypair,
    ) -> Self {
        Self {
            marketplace_client,
            wallet_keypair,
            job_counter: 0,
        }
    }

    pub async fn create_job(
        &mut self,
        witness_commitment: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
    ) -> Result<(Pubkey, Signature)> {
        let job_id = self.job_counter;
        self.job_counter += 1;

        // Build CreateJob instruction (reusing SDK!)
        let create_job_ix = self.marketplace_client.create_job_instruction(
            &self.wallet_keypair.pubkey(),
            job_id,
            CircuitType::ZcashOrchard,
            witness_commitment,
            witness_size,
            price_lamports,
            600, // 10 min timeout
        )?;

        // Send transaction
        let signature = self.marketplace_client.send_and_confirm_transaction(
            &[create_job_ix],
            &[&self.wallet_keypair],
        )?;

        // Derive job PDA
        let (job_pda, _) = self.marketplace_client.get_job_pda(
            &self.wallet_keypair.pubkey(),
            job_id,
        );

        Ok((job_pda, signature))
    }
}
```

**3.2 Job Status Polling (Rust)**

Archivo: `rust/src/solana/client.rs`

```rust
use cypherlink_types::JobStatus;
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;
use tokio::time::sleep;

pub struct JobPoller {
    marketplace_client: MarketplaceClient,
}

impl JobPoller {
    pub fn new(marketplace_client: MarketplaceClient) -> Self {
        Self { marketplace_client }
    }

    pub async fn poll_until_complete(
        &self,
        job_pda: &Pubkey,
    ) -> Result<JobAccount> {
        loop {
            let account = self.marketplace_client
                .rpc_client
                .get_account(job_pda)?;

            let job: JobAccount = borsh::from_slice(&account.data)?;

            match job.status {
                JobStatus::Completed => {
                    log::info!("Job completed successfully");
                    return Ok(job);
                }
                JobStatus::Failed => {
                    anyhow::bail!("Job failed");
                }
                JobStatus::Cancelled => {
                    anyhow::bail!("Job was cancelled");
                }
                _ => {
                    log::info!("Job status: {:?}, waiting...", job.status);
                    sleep(Duration::from_secs(3)).await;
                }
            }
        }
    }

    pub async fn get_job_status(&self, job_pda: &Pubkey) -> Result<JobStatus> {
        let account = self.marketplace_client
            .rpc_client
            .get_account(job_pda)?;

        let job: JobAccount = borsh::from_slice(&account.data)?;
        Ok(job.status)
    }
}
```

**3.3 Complete Wallet API (Rust)**

Archivo: `rust/src/lib.rs`

```rust
use cypherlink_sdk::MarketplaceClient;

pub struct Wallet {
    // ... existing fields ...
    job_creator: JobCreator,
    job_poller: JobPoller,
    storage_client: WitnessStorageClient,
}

impl Wallet {
    // ... existing methods ...

    /// Complete flow: Generate witness -> Encrypt -> Upload -> CreateJob
    pub async fn send_private_transaction(
        &mut self,
        recipient_zcash: String,
        amount_zats: u64,
        memo: String,
        prover_pubkey: [u8; 32],
        backend_url: String,
    ) -> Result<JobInfo> {
        // 1. Generate mock witness
        let witness = MockOrchardWitness::generate_for_transaction(
            &recipient_zcash,
            amount_zats,
            &memo,
        );
        let witness_bytes = witness.to_bytes()?;

        // 2. Encrypt witness
        let encrypted = encrypt_witness(&witness_bytes, &prover_pubkey)?;

        // 3. Upload to backend
        let commitment = self.storage_client.upload_witness(&encrypted).await?;
        let witness_size = borsh::to_vec(&encrypted)?.len() as u32;

        // 4. Create job on-chain
        let price = 20_000_000; // 0.02 SOL
        let (job_pda, signature) = self.job_creator
            .create_job(commitment, witness_size, price)
            .await?;

        Ok(JobInfo {
            job_pda: job_pda.to_string(),
            signature: signature.to_string(),
            commitment: hex::encode(commitment),
            price_sol: price as f64 / 1e9,
        })
    }

    pub async fn get_job_status(&self, job_pda: String) -> Result<String> {
        let pubkey = Pubkey::from_str(&job_pda)?;
        let status = self.job_poller.get_job_status(&pubkey).await?;
        Ok(format!("{:?}", status))
    }

    pub async fn wait_for_proof(&self, job_pda: String) -> Result<Vec<u8>> {
        let pubkey = Pubkey::from_str(&job_pda)?;

        // Poll until complete
        let job = self.job_poller.poll_until_complete(&pubkey).await?;

        // Download proof
        let proof = self.storage_client
            .download_proof(&job.witness_commitment)
            .await?;

        Ok(proof)
    }
}
```

**3.4 FFI Complete Interface**

```idl
// ffi.udl
namespace cypherlink_wallet {
    WalletHandle create_wallet(Network network, string program_id);
};

dictionary JobInfo {
    string job_pda;
    string signature;
    string commitment;
    double price_sol;
};

interface WalletHandle {
    // Wallet basics
    constructor(Network network, string program_id);
    string get_mnemonic();
    string get_solana_address();
    u64 get_solana_balance();
    string request_airdrop(u64 lamports);

    // Send flow
    [Async]
    JobInfo send_private_transaction(
        string recipient_zcash,
        u64 amount_zats,
        string memo,
        bytes prover_pubkey,
        string backend_url
    );

    // Job tracking
    [Async]
    string get_job_status(string job_pda);

    [Async]
    bytes wait_for_proof(string job_pda);
};
```

**3.5 Send Screen UI (Flutter)**

Archivo: `lib/screens/send_screen.dart`

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../providers/wallet_provider.dart';

class SendScreen extends ConsumerStatefulWidget {
  @override
  _SendScreenState createState() => _SendScreenState();
}

class _SendScreenState extends ConsumerState<SendScreen> {
  final _recipientController = TextEditingController();
  final _amountController = TextEditingController();
  final _memoController = TextEditingController();
  bool _isSending = false;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text('Send Private Transaction')),
      body: Padding(
        padding: EdgeInsets.all(16),
        child: Column(
          children: [
            TextField(
              controller: _recipientController,
              decoration: InputDecoration(
                labelText: 'Recipient (Zcash address)',
                hintText: 'zs1...',
              ),
            ),
            SizedBox(height: 16),
            TextField(
              controller: _amountController,
              decoration: InputDecoration(
                labelText: 'Amount (ZEC)',
              ),
              keyboardType: TextInputType.numberWithOptions(decimal: true),
            ),
            SizedBox(height: 16),
            TextField(
              controller: _memoController,
              decoration: InputDecoration(
                labelText: 'Memo (optional)',
              ),
              maxLines: 3,
            ),
            SizedBox(height: 24),
            if (_isSending)
              CircularProgressIndicator()
            else
              ElevatedButton(
                onPressed: _sendTransaction,
                child: Text('Send Transaction'),
                style: ElevatedButton.styleFrom(
                  minimumSize: Size(double.infinity, 48),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Future<void> _sendTransaction() async {
    setState(() => _isSending = true);

    try {
      final wallet = ref.read(walletProvider.notifier).handle;
      if (wallet == null) throw Exception('Wallet not initialized');

      // TODO: Get real prover pubkey from on-chain
      final mockProverPubkey = List<int>.filled(32, 1);

      final recipient = _recipientController.text;
      final amount = (double.parse(_amountController.text) * 1e8).toInt();
      final memo = _memoController.text;

      final jobInfo = await wallet.sendPrivateTransaction(
        recipient,
        amount,
        memo,
        mockProverPubkey,
        'http://localhost:3030', // TODO: Config
      );

      // Navigate to job status screen
      Navigator.pushNamed(
        context,
        '/job-status',
        arguments: jobInfo.jobPda,
      );
    } catch (e) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Error: $e')),
      );
    } finally {
      setState(() => _isSending = false);
    }
  }
}
```

**3.6 Job Status Screen (Flutter)**

Archivo: `lib/screens/job_status_screen.dart`

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../providers/wallet_provider.dart';

class JobStatusScreen extends ConsumerStatefulWidget {
  final String jobPda;

  const JobStatusScreen({required this.jobPda});

  @override
  _JobStatusScreenState createState() => _JobStatusScreenState();
}

class _JobStatusScreenState extends ConsumerState<JobStatusScreen> {
  String _status = 'Loading...';
  bool _isPolling = true;

  @override
  void initState() {
    super.initState();
    _startPolling();
  }

  Future<void> _startPolling() async {
    final wallet = ref.read(walletProvider.notifier).handle;
    if (wallet == null) return;

    while (_isPolling && mounted) {
      try {
        final status = await wallet.getJobStatus(widget.jobPda);
        setState(() => _status = status);

        if (status == 'Completed' || status == 'Failed') {
          setState(() => _isPolling = false);
          if (status == 'Completed') {
            _onJobCompleted();
          }
          break;
        }

        await Future.delayed(Duration(seconds: 3));
      } catch (e) {
        setState(() => _status = 'Error: $e');
        break;
      }
    }
  }

  Future<void> _onJobCompleted() async {
    final wallet = ref.read(walletProvider.notifier).handle;
    if (wallet == null) return;

    try {
      final proof = await wallet.waitForProof(widget.jobPda);

      // In real app: use proof to sign Zcash transaction
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Proof received! (${proof.length} bytes)'),
          backgroundColor: Colors.green,
        ),
      );
    } catch (e) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Error downloading proof: $e')),
      );
    }
  }

  @override
  void dispose() {
    _isPolling = false;
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text('Job Status')),
      body: Padding(
        padding: EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Job PDA:', style: TextStyle(fontWeight: FontWeight.bold)),
            SizedBox(height: 4),
            SelectableText(
              widget.jobPda,
              style: TextStyle(fontFamily: 'monospace', fontSize: 12),
            ),
            SizedBox(height: 24),
            Text('Status:', style: TextStyle(fontWeight: FontWeight.bold)),
            SizedBox(height: 8),
            Row(
              children: [
                if (_isPolling) CircularProgressIndicator(),
                SizedBox(width: 16),
                Text(
                  _status,
                  style: TextStyle(fontSize: 18),
                ),
              ],
            ),
            SizedBox(height: 32),
            _buildStatusTimeline(),
          ],
        ),
      ),
    );
  }

  Widget _buildStatusTimeline() {
    final steps = [
      'Pending',
      'Claimed',
      'Proving',
      'Completed',
    ];

    final currentIndex = steps.indexOf(_status);

    return Column(
      children: steps.asMap().entries.map((entry) {
        final index = entry.key;
        final step = entry.value;
        final isActive = index <= currentIndex;

        return Row(
          children: [
            Icon(
              isActive ? Icons.check_circle : Icons.circle_outlined,
              color: isActive ? Colors.green : Colors.grey,
            ),
            SizedBox(width: 8),
            Text(
              step,
              style: TextStyle(
                color: isActive ? Colors.black : Colors.grey,
                fontWeight: isActive ? FontWeight.bold : FontWeight.normal,
              ),
            ),
          ],
        );
      }).toList(),
    );
  }
}
```

**Entregables Fase 3:**
- ✅ Create job on-chain funciona desde mobile
- ✅ Transaction firmada y enviada correctamente
- ✅ Job PDA se deriva bien
- ✅ Polling muestra status real
- ✅ UI con timeline de progreso
- ✅ Download proof cuando completa

**Tiempo:** 24-30 horas

---

### FASE 4: Integration & Testing (Días 10-12) - 16-20h

**Objetivo:** E2E funcional con prover node real

#### Tasks:

**4.1 Configuración de prover real**

```bash
# Terminal 1: Witness storage
cd witness-storage
RUST_LOG=info cargo run

# Terminal 2: Register prover
cd prover-node
cargo run -- register \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/prover.json \
  --stake-amount 10000000000

# Save encryption pubkey output
cargo run -- show-pubkey --keypair ~/.config/solana/prover.json

# Terminal 3: Run prover daemon
cargo run -- run \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/prover.json \
  --witness-backend-url http://localhost:3030
```

**4.2 Fetch prover pubkey on-chain (Rust)**

```rust
impl Wallet {
    pub async fn get_available_prover(&self) -> Result<ProverInfo> {
        // Get all prover accounts
        let accounts = self.marketplace_client
            .rpc_client
            .get_program_accounts(&self.marketplace_client.program_id)?;

        for (pubkey, account) in accounts {
            if let Ok(prover) = borsh::from_slice::<ProverAccount>(&account.data) {
                if prover.is_active && prover.reputation_score > 500 {
                    return Ok(ProverInfo {
                        pubkey: pubkey.to_string(),
                        encryption_pubkey: prover.encryption_pubkey,
                        reputation: prover.reputation_score,
                    });
                }
            }
        }

        anyhow::bail!("No available provers found")
    }
}
```

**4.3 Update Send Screen to use real prover**

```dart
// In SendScreen
final proverInfo = await wallet.getAvailableProver();

final jobInfo = await wallet.sendPrivateTransaction(
  recipient,
  amount,
  memo,
  proverInfo.encryptionPubkey, // Real prover pubkey
  'http://10.0.2.2:3030', // Android emulator -> localhost
);
```

**4.4 E2E Test Flow**

1. Start all services (validator, witness storage, prover)
2. Open mobile app
3. Create wallet (auto-airdrop 1 SOL)
4. Click "Send Transaction"
5. Fill form, submit
6. Watch job status change:
   - Pending → Claimed → Proving → Completed
7. Proof downloads successfully
8. Balance updated

**4.5 Error Handling**

```rust
// Add to Wallet
pub async fn send_private_transaction_with_retries(
    &mut self,
    // ... params
) -> Result<JobInfo> {
    let mut retries = 3;

    loop {
        match self.send_private_transaction(...).await {
            Ok(job_info) => return Ok(job_info),
            Err(e) if retries > 0 => {
                log::warn!("Transaction failed, retrying: {}", e);
                retries -= 1;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

**Entregables Fase 4:**
- ✅ E2E flow funciona con prover real
- ✅ Witness encrypted correctamente
- ✅ Prover descifra y genera proof
- ✅ Mobile recibe proof
- ✅ Error handling robusto

**Tiempo:** 16-20 horas

---

### FASE 5: Polish & Demo Mode (Días 13-14) - 12-16h

**Objetivo:** UI pulida, demo mode, video recording

#### Tasks:

**5.1 UI Improvements**
- Animations (job status transitions)
- Better error messages
- Loading states
- Solana Explorer links
- Copy to clipboard buttons

**5.2 Demo Mode**
```dart
// Config for demo
const DEMO_CONFIG = {
  'auto_fill_forms': true,
  'fast_polling': true, // 1s instead of 3s
  'mock_prover': false, // Use real prover
};
```

**5.3 Settings Screen**
- Network selector (Devnet/Mainnet)
- RPC endpoint config
- Witness backend URL
- Export seed phrase
- Clear wallet data

**5.4 Transaction History**
- List recent jobs
- Status badges
- Solana Explorer links
- Filter by status

**5.5 Demo Script**
```markdown
1. Show home screen (balances)
2. Click "Send Private Transaction"
3. Fill: recipient, amount, memo
4. Submit → Navigate to Job Status
5. Show status changes (Pending → Claimed → Proving)
6. Completed! Show proof received
7. (Optional) Show Solana Explorer
8. Back to home, balance updated
```

**Entregables Fase 5:**
- ✅ UI profesional
- ✅ Demo script funcional
- ✅ Settings completo
- ✅ Transaction history
- ✅ Video demo grabado

**Tiempo:** 12-16 horas

---

## ESTIMACIÓN TOTAL

| Fase | Descripción | Tiempo |
|------|-------------|--------|
| 0 | Setup | 6-8h |
| 1 | Wallet Core | 20-24h |
| 2 | Witness Gen | 10-14h |
| 3 | Job Creation | 24-30h |
| 4 | Integration | 16-20h |
| 5 | Polish | 12-16h |
| **TOTAL** | | **88-112h** |

**Timeline:**
- 1 dev full-time: **11-14 días** (~8h/día)
- 2 devs: **7-9 días**
- 3 devs: **5-6 días**

## DEPENDENCIAS RUST (Cargo.toml)

```toml
[package]
name = "cypherlink-wallet-ffi"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib"]

[dependencies]
# Solana (reusing SDK)
cypherlink-sdk = { path = "../../sdk" }
cypherlink-types = { path = "../../shared/types" }
solana-sdk = "2.1"
solana-client = "2.1"

# Crypto
bip39 = "2.0"
bip32 = "0.5"
ed25519-dalek = "2.1"
x25519-dalek = { version = "2.0", features = ["static_secrets"] }
chacha20poly1305 = "0.10"

# Serialization
borsh = "1.5"
bincode = "1.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# FFI
uniffi = { version = "0.27", features = ["cli"] }

# Async
tokio = { version = "1.47", features = ["full"] }

# HTTP
reqwest = { version = "0.12", features = ["json"] }

# Utils
anyhow = "1.0"
log = "0.4"
hex = "0.4"
blake2 = "0.10"

[build-dependencies]
uniffi = { version = "0.27", features = ["build"] }
```

## DEPENDENCIAS FLUTTER (pubspec.yaml)

```yaml
name: cypherlink_wallet
description: Private Zcash wallet with decentralized proving
version: 1.0.0+1

environment:
  sdk: '>=3.5.0 <4.0.0'

dependencies:
  flutter:
    sdk: flutter

  # State management
  flutter_riverpod: ^2.5.0
  riverpod_annotation: ^2.3.0

  # Navigation
  go_router: ^14.0.0

  # Storage
  flutter_secure_storage: ^9.2.0
  shared_preferences: ^2.2.3

  # UI
  google_fonts: ^6.2.1
  flutter_svg: ^2.0.10
  qr_flutter: ^4.1.0

  # Utils
  intl: ^0.19.0
  url_launcher: ^6.3.0

  # FFI (generated by uniffi)
  ffi: ^2.1.0

dev_dependencies:
  flutter_test:
    sdk: flutter
  flutter_lints: ^4.0.0
  riverpod_generator: ^2.4.0
  build_runner: ^2.4.0
```

## RIESGOS Y MITIGACIONES

### Riesgo 1: Solana RPC Rate Limits
**Probabilidad:** Alta
**Impacto:** Medium
**Mitigación:**
- Usar Helius free tier (100k req/day)
- Implementar retry with backoff
- Cache account data localmente

### Riesgo 2: Transaction Failures
**Probabilidad:** Media
**Impacto:** Alto
**Mitigación:**
- Pre-flight balance check
- Recent blockhash retry logic
- Clear error messages
- Transaction simulation antes de enviar

### Riesgo 3: Mobile Compilation Issues
**Probabilidad:** Media
**Impacto:** Alto
**Mitigación:**
- Usar uniffi (más maduro que flutter_rust_bridge)
- Testear compilación temprano (Fase 0)
- Documentar setup steps
- Docker container para build reproducible

### Riesgo 4: Prover Node Downtime
**Probabilidad:** Baja
**Impacto:** Alto
**Mitigación:**
- Verificar prover activo antes de crear job
- Timeout handling en polling
- UI muestra "No provers available" si aplica
- Mock mode para demos

### Riesgo 5: Devnet Reset
**Probabilidad:** Baja
**Impacto:** Medium
**Mitigación:**
- Scripts de re-deployment rápido
- Video backup del demo
- Poder correr en localnet

## NOTAS DE IMPLEMENTACIÓN

### Android Emulator Networking
```dart
// Android emulator -> host machine
const LOCALHOST_FROM_ANDROID = 'http://10.0.2.2';

// Config
final backendUrl = Platform.isAndroid
  ? '$LOCALHOST_FROM_ANDROID:3030'
  : 'http://localhost:3030';
```

### Solana Explorer Links
```dart
String getExplorerUrl(String signature, {bool isDevnet = true}) {
  final cluster = isDevnet ? 'devnet' : '';
  return 'https://explorer.solana.com/tx/$signature?cluster=$cluster';
}
```

### Secure Storage Best Practices
```dart
// NEVER log or expose mnemonic
const storage = FlutterSecureStorage(
  aOptions: AndroidOptions(
    encryptedSharedPreferences: true,
  ),
);

// Always validate before storing
Future<void> saveMnemonic(String mnemonic) async {
  if (!isValidMnemonic(mnemonic)) {
    throw Exception('Invalid mnemonic');
  }
  await storage.write(key: 'mnemonic', value: mnemonic);
}
```

## PRÓXIMOS PASOS

Una vez aceptes este plan, generaré:

1. **Estructura completa de archivos** (todos los `.dart`, `.rs`, configs)
2. **uniffi interface definition** completa
3. **Scripts de build** (Android, iOS future)
4. **Testing checklist** para cada fase
5. **Demo script** detallado con screenshots

¿Confirmás el approach? ¿Algún ajuste antes de empezar implementación?
