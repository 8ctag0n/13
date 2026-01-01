pragma circom 2.1.6;

include "../lib/bn254.circom";
include "../lib/poseidon_utils.circom";
include "../lib/pairing.circom";

var TEST_VAR = 10;

template TestAll() {
    signal input a;
    signal output b;
    b <== a;
}

component main = TestAll();
