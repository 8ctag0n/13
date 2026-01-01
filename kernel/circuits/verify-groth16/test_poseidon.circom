pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";

template TestPoseidon() {
    signal input a;
    signal input b;
    signal output c;
    
    component hasher = Poseidon(2);
    hasher.inputs[0] <== a;
    hasher.inputs[1] <== b;
    c <== hasher.out;
}

component main = TestPoseidon();
