# ZyberLink Localnet Testing Guide

Quick guide for running and testing ZyberLink on localnet with dynamic pricing.

---

## 🚀 Quick Start

### Option 1: One Command (Recommended)
Start everything with a single command:

```bash
./localnet-start-all.sh
```

This will:
1. ✅ Setup infrastructure (validator, DB, program deploy, prover wallets)
2. ✅ Start backend API server
3. ✅ Start 3 prover nodes
4. ✅ Run API validation tests

**Time**: ~90 seconds

### Option 2: Step by Step

```bash
# 1. Infrastructure setup
./localnet-setup.sh

# 2. Start backend
./localnet-start-backend.sh

# 3. Start provers
./localnet-start-provers.sh

# 4. Test API
./localnet-test-api.sh
```

---

## 📜 Available Scripts

| Script | Purpose | Time |
|--------|---------|------|
| `localnet-setup.sh` | Bootstrap infrastructure | ~60s |
| `localnet-start-all.sh` | Start all services + tests | ~90s |
| `localnet-start-backend.sh` | Start backend API only | ~5s |
| `localnet-start-provers.sh` | Start 3 prover nodes | ~10s |
| `localnet-stop-all.sh` | Stop all services | ~3s |
| `localnet-test-api.sh` | Run backend API tests | ~2s |
| `localnet-test-e2e-flow.sh` | E2E flow validation | ~5s |

---

## 🧪 Testing

### Backend API Tests (Automated)
```bash
# Run 14 E2E tests for /api/estimate-cost endpoint
./localnet-test-api.sh
```

**Coverage:**
- ✅ All 5 complexity tiers
- ✅ All 8 operation types (Add, Multiply, Sum, Threshold, etc.)
- ✅ Pricing scaling with prover count
- ✅ Error handling

### E2E Flow Test
```bash
# Validate dynamic pricing flow
./localnet-test-e2e-flow.sh
```

**Validates:**
- Cost estimation API
- All 5 complexity tiers
- Multi-prover cost aggregation
- ROI calculator logic

### Manual Frontend Testing
```bash
cd webapp
npm run dev
# Open: http://localhost:5173
```

---

## 📊 Service URLs

Once running:

- **Validator RPC**: http://localhost:8899
- **Backend API**: http://127.0.0.1:8080
  - Health: http://127.0.0.1:8080/health
  - Estimate Cost: POST http://127.0.0.1:8080/api/estimate-cost
- **Database**: postgresql://localhost:5432/zyberlink
- **Frontend**: http://localhost:5173 (after `npm run dev`)

---

## 📝 Logs

View logs in real-time:

```bash
# Validator
tail -f /tmp/solana-validator.log

# Backend
tail -f /tmp/blink-server.log

# Provers
tail -f /tmp/prover-1.log
tail -f /tmp/prover-2.log
tail -f /tmp/prover-3.log

# All provers at once
tail -f /tmp/prover-*.log
```

---

## 🛑 Stopping Services

```bash
./localnet-stop-all.sh
```

---

## 💡 Example: Test Dynamic Pricing

```bash
# Estimate cost for Tier 1 operation (Add)
curl -X POST http://127.0.0.1:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "add",
    "operation_value": 42,
    "required_provers": 3
  }' | jq

# Expected response:
# {
#   "operation": "Add",
#   "complexity_tier": 1,
#   "min_payment_lamports": 1000000,      # 0.001 SOL per prover
#   "total_min_payment_lamports": 3000000, # 0.003 SOL total
#   "timeout_seconds": 60
# }
```

```bash
# Estimate cost for Tier 5 operation (Histogram)
curl -X POST http://127.0.0.1:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "histogram",
    "bins": 10,
    "required_provers": 3
  }' | jq

# Expected response:
# {
#   "operation": "Histogram",
#   "complexity_tier": 5,
#   "min_payment_lamports": 732050808,       # ~0.73 SOL per prover
#   "total_min_payment_lamports": 2196152424, # ~2.19 SOL total
#   "timeout_seconds": 1600
# }
```

---

## 🔍 Troubleshooting

### Validator not starting
```bash
# Check if port 8899 is in use
lsof -i :8899

# Clean old ledger data
rm -rf ~/.zyberlink-localnet-ledger
./localnet-setup.sh
```

### Backend not responding
```bash
# Check logs
tail -20 /tmp/blink-server.log

# Restart
./localnet-start-backend.sh
```

### Database connection failed
```bash
# Check container
podman ps | grep postgres

# Restart container
podman-compose down
podman-compose up -d
```

---

## 📦 What Gets Created

### Environment Files
- `blink-server/.env` - Backend configuration
- `webapp/.env.local` - Frontend configuration

### Prover Wallets
- `/tmp/prover-1-keypair.json` (5 SOL)
- `/tmp/prover-2-keypair.json` (5 SOL)
- `/tmp/prover-3-keypair.json` (5 SOL)

### Logs
- `/tmp/solana-validator.log`
- `/tmp/blink-server.log`
- `/tmp/prover-{1,2,3}.log`
- `/tmp/program-deploy.log`

---

## ✅ Test Status

| Component | Tests | Status |
|-----------|-------|--------|
| Solana Program | 54 | ✅ Passing |
| SDK Instructions | 11 | ✅ Complete |
| Backend API E2E | 14 | ✅ Passing |
| Dynamic Pricing | 5 tiers | ✅ Validated |

---

## 🎯 Next Steps

1. **Run automated tests**: `./localnet-start-all.sh`
2. **Test frontend**: `cd webapp && npm run dev`
3. **Create test job**: Use frontend or API
4. **Monitor provers**: `tail -f /tmp/prover-*.log`

---

**Last Updated**: 2025-11-22
**Phase**: 8 - E2E Testing
**Status**: Backend API validated ✅
