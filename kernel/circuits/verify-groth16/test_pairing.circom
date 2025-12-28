pragma circom 2.1.6;

include "../lib/pairing.circom";

template TestPairing() {
    signal input a;
    signal output b;
    b <== a;
}

component main = TestPairing();
