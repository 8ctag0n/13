#!/bin/bash
# Setup Powers of Tau para los circuitos de verificación

set -e

PTAU_DIR="../ptau"
mkdir -p $PTAU_DIR

echo "=== Descargando Powers of Tau ==="
echo ""

# Para circuitos de ~50K constraints necesitamos al menos 2^16 (65536)
# Usamos el archivo de Hermez (ceremonia completada y verificada)

PTAU_16="powersOfTau28_hez_final_16.ptau"
PTAU_URL="https://hermez.s3-eu-west-1.amazonaws.com/$PTAU_16"

if [ -f "$PTAU_DIR/$PTAU_16" ]; then
    echo "PTAU file ya existe: $PTAU_DIR/$PTAU_16"
else
    echo "Descargando $PTAU_16 (~1.3GB)..."
    echo "URL: $PTAU_URL"
    curl -L -o "$PTAU_DIR/$PTAU_16" "$PTAU_URL"
fi

echo ""
echo "=== Verificando integridad ==="
echo "Verificando PTAU..."
snarkjs powersoftau verify "$PTAU_DIR/$PTAU_16"

echo ""
echo "=== Setup completado ==="
echo "PTAU disponible en: $PTAU_DIR/$PTAU_16"
echo ""
echo "Capacidad: 2^16 = 65,536 constraints"
echo "Suficiente para:"
echo "  - VERIFY_GROTH16 (~50K constraints)"
echo "  - VERIFY_PLONK (~80K constraints) - necesitará 2^17"
echo "  - VERIFY_HALO2 (~100K constraints) - necesitará 2^17"
echo ""
echo "Para circuitos más grandes, descargar powersOfTau28_hez_final_17.ptau"
