#!/usr/bin/env python3
"""
Simple deployment script for pBTCFi contracts to Katana devnet
Uses default Katana predeployed account and RPC JSON calls
"""
import json
import requests
import subprocess
import sys

# Katana RPC URL
RPC_URL = "http://localhost:5050"

# Katana default account (predeployed account #0)
# These are the default values from Katana
ACCOUNT_ADDRESS = "0x517ececd29116499f4a1b64b094da79ba08dfd54a3edaa316134c41f8160973"
ACCOUNT_PRIVATE_KEY = "0x1800000000300000180000000000030000000000003006001800006600"

# Contract file
CONTRACT_FILE = "target/dev/pbtcfi_core_PBTCFiCore.contract_class.json"

def rpc_call(method, params):
    """Make RPC call to Katana"""
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    }
    response = requests.post(RPC_URL, json=payload)
    result = response.json()

    if "error" in result:
        print(f"RPC Error: {result['error']}", file=sys.stderr)
        return None

    return result.get("result")

def get_block_number():
    """Get current block number"""
    return rpc_call("starknet_blockNumber", [])

def get_nonce(address):
    """Get account nonce"""
    return rpc_call("starknet_getNonce", [{
        "block_id": "latest"
    }, address])

def main():
    print("=== pBTCFi Contract Deployment to Katana ===\n")

    # Check Katana connection
    print("1. Checking Katana connection...")
    block = get_block_number()
    if block is None:
        print("ERROR: Cannot connect to Katana", file=sys.stderr)
        return 1
    print(f"   Connected! Current block: {block}\n")

    # Load contract class
    print("2. Loading contract class...")
    try:
        with open(CONTRACT_FILE, 'r') as f:
            contract_class = json.load(f)
        print(f"   Loaded: {CONTRACT_FILE}\n")
    except Exception as e:
        print(f"ERROR: Cannot load contract: {e}", file=sys.stderr)
        return 1

    # For MVP testing, we'll use a simplified approach:
    # Just print the deployment command that should be run manually
    print("=" * 60)
    print("MANUAL DEPLOYMENT REQUIRED")
    print("=" * 60)
    print("\nFor E2E testing, you can:")
    print("\n1. Use a mock/hardcoded contract address for testing:")
    print("   export PBTCFI_CONTRACT_ADDRESS=\"0x1234567890abcdef\"")
    print("\n2. OR deploy using starkli (if available):")
    print(f"   starkli declare {CONTRACT_FILE} --rpc {RPC_URL}")
    print("   starkli deploy <CLASS_HASH> --rpc " + RPC_URL)
    print("\n3. OR use the Katana pre-deployed account for manual testing")
    print(f"   Account: {ACCOUNT_ADDRESS}")
    print("\nFor this E2E test, we'll use a MOCK contract address.")
    print("The sync code will attempt to fetch events from this address.")
    print("\n" + "=" * 60)

    # For testing purposes, return a mock address
    mock_address = "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"
    print(f"\nUsing mock contract address: {mock_address}")
    print("\nSet this in your environment:")
    print(f"export PBTCFI_CONTRACT_ADDRESS=\"{mock_address}\"")
    print(f"export STARKNET_RPC_URL=\"{RPC_URL}\"")

    return 0

if __name__ == "__main__":
    sys.exit(main())
