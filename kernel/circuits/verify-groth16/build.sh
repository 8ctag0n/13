#!/bin/bash
# Build script para VERIFY_GROTH16

set -e

CIRCUIT_NAME="verify_groth16"
CIRCUIT_FILE="circuit.circom"
BUILD_DIR="./build"
PTAU_FILE="../ptau/powersOfTau28_hez_final_16.ptau"

echo "=== Building VERIFY_GROTH16 Circuit ==="

# Crear directorio de build
mkdir -p $BUILD_DIR

# Paso 1: Compilar circuito
echo "[1/6] Compilando circuito..."
circom $CIRCUIT_FILE --r1cs --wasm --sym -o $BUILD_DIR

# Paso 2: Ver información del circuito
echo "[2/6] Información del circuito:"
snarkjs r1cs info $BUILD_DIR/${CIRCUIT_NAME}.r1cs

# Paso 3: Exportar R1CS a JSON (opcional, para debug)
echo "[3/6] Exportando R1CS..."
snarkjs r1cs export json $BUILD_DIR/${CIRCUIT_NAME}.r1cs $BUILD_DIR/${CIRCUIT_NAME}.r1cs.json

# Paso 4: Calcular witness (con input de ejemplo)
echo "[4/6] Calculando witness..."
node $BUILD_DIR/${CIRCUIT_NAME}_js/generate_witness.js \
    $BUILD_DIR/${CIRCUIT_NAME}_js/${CIRCUIT_NAME}.wasm \
    input.json \
    $BUILD_DIR/witness.wtns

# Paso 5: Trusted Setup (Phase 2)
if [ -f "$PTAU_FILE" ]; then
    echo "[5/6] Ejecutando trusted setup (Phase 2)..."
    snarkjs groth16 setup $BUILD_DIR/${CIRCUIT_NAME}.r1cs $PTAU_FILE $BUILD_DIR/${CIRCUIT_NAME}_0000.zkey

    # Contribución (en producción sería ceremonia multi-party)
    echo "Contribuyendo a la ceremonia..."
    snarkjs zkey contribute $BUILD_DIR/${CIRCUIT_NAME}_0000.zkey $BUILD_DIR/${CIRCUIT_NAME}_final.zkey \
        --name="ZyberLink Dev" -v -e="random entropy for development"

    # Exportar verification key
    snarkjs zkey export verificationkey $BUILD_DIR/${CIRCUIT_NAME}_final.zkey $BUILD_DIR/verification_key.json

    # Paso 6: Generar proof de prueba
    echo "[6/6] Generando proof de prueba..."
    snarkjs groth16 prove $BUILD_DIR/${CIRCUIT_NAME}_final.zkey $BUILD_DIR/witness.wtns \
        $BUILD_DIR/proof.json $BUILD_DIR/public.json

    # Verificar proof
    echo "Verificando proof..."
    snarkjs groth16 verify $BUILD_DIR/verification_key.json $BUILD_DIR/public.json $BUILD_DIR/proof.json

    echo ""
    echo "=== Build completado exitosamente ==="
    echo "Archivos generados en $BUILD_DIR/:"
    echo "  - ${CIRCUIT_NAME}.r1cs (circuito compilado)"
    echo "  - ${CIRCUIT_NAME}_final.zkey (proving key)"
    echo "  - verification_key.json (verification key)"
    echo "  - proof.json (proof de prueba)"
    echo "  - public.json (public inputs)"
else
    echo "[5/6] PTAU file no encontrado en $PTAU_FILE"
    echo "      Descargarlo de: https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_16.ptau"
    echo "      O ejecutar: ./setup_ptau.sh"
    echo ""
    echo "=== Build parcial completado ==="
    echo "Circuito compilado en $BUILD_DIR/${CIRCUIT_NAME}.r1cs"
fi
