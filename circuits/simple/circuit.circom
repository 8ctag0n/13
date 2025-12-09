pragma circom 2.0.0;

// Circuito simple: Multiplier
// Demuestra que conoces a y b tal que a * b = c
template Multiplier() {
    signal input a;
    signal input b;
    signal output c;

    c <== a * b;
}

component main = Multiplier();
