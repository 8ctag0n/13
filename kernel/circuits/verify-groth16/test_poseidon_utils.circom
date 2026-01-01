pragma circom 2.1.6;

include "../lib/poseidon_utils.circom";

template TestPU() {
    signal input a;
    signal input b;
    signal output c;
    
    component h = PoseidonG1();
    h.p[0] <== a;
    h.p[1] <== b;
    c <== h.out;
}

component main = TestPU();
