#!/usr/bin/env python3
"""Sign a message with Solana keypair (raw, no prefix)"""

import sys
import json
import base58
from nacl.signing import SigningKey

if len(sys.argv) != 3:
    print("Usage: sign-message-raw.py <keypair_path> <message>", file=sys.stderr)
    sys.exit(1)

keypair_path = sys.argv[1]
message = sys.argv[2]

try:
    # Load keypair
    with open(keypair_path, 'r') as f:
        keypair_bytes = json.load(f)

    # Create signing key from first 32 bytes (secret key)
    signing_key = SigningKey(bytes(keypair_bytes[:32]))

    # Sign message (raw bytes, no prefix)
    message_bytes = message.encode('utf-8')
    signed = signing_key.sign(message_bytes)

    # Extract signature (first 64 bytes of signed message)
    signature_bytes = signed.signature

    # Encode to base58 (Solana standard)
    signature_b58 = base58.b58encode(signature_bytes).decode('ascii')

    # Output just the signature
    print(signature_b58)

except Exception as e:
    print(f"Error: {e}", file=sys.stderr)
    sys.exit(1)
