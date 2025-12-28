use starknet::ContractAddress;

// ============================================================================
// FHE Verifier - ECDSA Signature Verification for FHE Results
// ============================================================================
// Sprint 2: Implements cryptographic verification of prover-signed FHE results
// to prevent result manipulation and ensure data integrity

#[starknet::interface]
pub trait IFheVerifier<TContractState> {
    /// Verify that a prover signed the FHE result
    ///
    /// # Arguments
    /// * `ciphertext_hash` - Keccak256 hash of original FHE ciphertext
    /// * `result_hash` - Keccak256 hash of computed result
    /// * `signature_r` - ECDSA signature component r
    /// * `signature_s` - ECDSA signature component s
    /// * `prover` - Address of the prover who signed
    ///
    /// # Returns
    /// * `true` if signature is valid, `false` otherwise
    fn verify_fhe_result(
        self: @TContractState,
        ciphertext_hash: felt252,
        result_hash: felt252,
        signature_r: felt252,
        signature_s: felt252,
        prover: ContractAddress,
    ) -> bool;

    /// Register a prover's ECDSA public key
    ///
    /// Must be called before prover can submit signed results.
    /// Typically called during prover registration.
    fn register_prover_pubkey(
        ref self: TContractState,
        prover: ContractAddress,
        pubkey_x: felt252,
        pubkey_y: felt252,
    );

    /// Get a registered prover's public key
    fn get_prover_pubkey(
        self: @TContractState,
        prover: ContractAddress,
    ) -> (felt252, felt252);
}

#[starknet::contract]
pub mod FheVerifier {
    use super::IFheVerifier;
    use starknet::ContractAddress;
    use core::ecdsa::check_ecdsa_signature;
    use core::keccak::keccak_u256s_be_inputs;
    use starknet::storage::{Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess};

    #[storage]
    struct Storage {
        /// Map: prover address -> (pubkey_x, pubkey_y)
        prover_pubkeys: Map<ContractAddress, (felt252, felt252)>,
    }

    #[abi(embed_v0)]
    impl FheVerifierImpl of IFheVerifier<ContractState> {
        fn verify_fhe_result(
            self: @ContractState,
            ciphertext_hash: felt252,
            result_hash: felt252,
            signature_r: felt252,
            signature_s: felt252,
            prover: ContractAddress,
        ) -> bool {
            // 1. Retrieve prover's public key (x-coordinate for STARK curve)
            let (pubkey_x, _pubkey_y) = self.prover_pubkeys.entry(prover).read();

            // Check if prover is registered
            if pubkey_x == 0 {
                return false;
            }

            // 2. Compute message hash: keccak256(ciphertext_hash || result_hash)
            let message = compute_message_hash(ciphertext_hash, result_hash);

            // 3. Verify ECDSA signature using STARK curve (4 params)
            check_ecdsa_signature(
                message,
                pubkey_x,
                signature_r,
                signature_s
            )
        }

        fn register_prover_pubkey(
            ref self: ContractState,
            prover: ContractAddress,
            pubkey_x: felt252,
            pubkey_y: felt252,
        ) {
            // MVP: No ownership validation (simplify)
            // Production: Only prover can register their own pubkey
            // Or contract owner (for admin)
            self.prover_pubkeys.entry(prover).write((pubkey_x, pubkey_y));
        }

        fn get_prover_pubkey(
            self: @ContractState,
            prover: ContractAddress,
        ) -> (felt252, felt252) {
            self.prover_pubkeys.entry(prover).read()
        }
    }

    /// Helper: Compute keccak256(ciphertext_hash || result_hash)
    fn compute_message_hash(ciphertext_hash: felt252, result_hash: felt252) -> felt252 {
        // Convert felts to u256 for keccak
        let ciphertext_u256: u256 = ciphertext_hash.into();
        let result_u256: u256 = result_hash.into();

        // Keccak of both values concatenated
        let hash = keccak_u256s_be_inputs(
            array![ciphertext_u256, result_u256].span()
        );

        // Convert u256 hash to felt252 (take low part)
        hash.low.into()
    }
}
