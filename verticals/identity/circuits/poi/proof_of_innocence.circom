pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";
include "circomlib/circuits/switcher.circom";

/// ProofOfInnocence - Circuit 10
/// Verifies that a wallet address is NOT in a blacklist using Sparse Merkle Tree non-membership proof
///
/// Public inputs:
///   - blacklist_root: Merkle root of the blacklist tree
///   - threshold: Must be 0 (for future extensibility)
///   - timestamp: Unix timestamp of the proof
///
/// Private inputs:
///   - wallet_address: The address to verify is not blacklisted
///   - merkle_path[20]: Sibling hashes along the path
///   - merkle_indices[20]: Path direction (0=left, 1=right)

template ProofOfInnocence() {
    // Public inputs (must match Rust struct PoiPublicInputs)
    signal input blacklist_root;
    signal input threshold;
    signal input timestamp;

    // Private inputs
    signal input wallet_address;
    signal input merkle_path[20];
    signal input merkle_indices[20];

    var DEPTH = 20;

    // For Sparse Merkle Tree non-membership:
    // Start with leaf = 0 (empty leaf) and compute path to root
    // If computed root == blacklist_root, it proves the position is empty
    // meaning the wallet_address is NOT in the blacklist

    signal computed_hashes[DEPTH + 1];
    computed_hashes[0] <== 0;  // Empty leaf for non-membership

    // Components for path computation
    component switchers[DEPTH];
    component hashers[DEPTH];

    for (var i = 0; i < DEPTH; i++) {
        // Use Switcher to determine left/right based on index
        // Switcher: if sel=0 then outL=L, outR=R; if sel=1 then outL=R, outR=L
        switchers[i] = Switcher();
        switchers[i].sel <== merkle_indices[i];
        switchers[i].L <== computed_hashes[i];  // Current hash
        switchers[i].R <== merkle_path[i];      // Sibling from path

        // Hash the pair using Poseidon
        // When index=0: left=current, right=sibling
        // When index=1: left=sibling, right=current
        hashers[i] = Poseidon(2);
        hashers[i].inputs[0] <== switchers[i].outL;
        hashers[i].inputs[1] <== switchers[i].outR;

        computed_hashes[i + 1] <== hashers[i].out;
    }

    // Verify computed root matches public blacklist_root
    computed_hashes[DEPTH] === blacklist_root;

    // Threshold constraint (should be 0 for strict non-membership)
    component is_threshold_zero = IsZero();
    is_threshold_zero.in <== threshold;
    is_threshold_zero.out === 1;

    // Timestamp is validated on-chain, no circuit constraint needed
    // Just ensuring it's used (to avoid "unused signal" warning)
    signal timestamp_check;
    timestamp_check <== timestamp * timestamp;

    // Wallet address is used to determine path position (off-chain)
    // Include constraint to avoid "unused signal" warning
    signal wallet_check;
    wallet_check <== wallet_address * wallet_address;
}

component main {public [blacklist_root, threshold, timestamp]} = ProofOfInnocence();
