pragma circom 2.1.6;

include "../lib/bn254.circom";

template TestBn() {
    signal input x;
    signal output y;
    y <== x * bn254_g1_x();
}

component main = TestBn();
