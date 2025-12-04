# Blink Server Deployment Guide

## Overview

The ZyberLink Blink Server provides [Solana Actions/Blinks](https://solana.com/docs/advanced/actions) endpoints for prover funding via QR codes and shareable links. This enables seamless onboarding of new provers through mobile wallets and social media.

**What are Solana Blinks?**
Blockchain Links (Blinks) are shareable URLs that trigger on-chain transactions directly from web/social platforms without leaving the context. Users can fund prover wallets by scanning a QR code or clicking a link.

---

## Prerequisites

### Required

- Rust 1.75+
- Solana CLI 2.1+
- Access to Solana RPC endpoint (devnet or mainnet)

### Recommended

- Reverse proxy (Nginx/Caddy) for HTTPS
- Domain name for production deployment
- Process manager (systemd or PM2) for production

---

## Local Development

### 1. Build the Server

```bash
cd /path/to/zyberlink/blink-server

# Build in development mode
cargo build

# Or build optimized for production
cargo build --release
```

### 2. Configure Environment

Create a `.env` file (optional, defaults shown):

```bash
# Server configuration
HOST=0.0.0.0
PORT=8080
BASE_URL=http://localhost:8080

# Logging level
RUST_LOG=info

# Solana configuration (optional, for custom RPC)
SOLANA_RPC_URL=https://api.devnet.solana.com
```

### 3. Run the Server

```bash
# Development mode
cargo run

# Production mode (optimized binary)
cargo run --release
```

### 4. Test Endpoints

```bash
# Health check
curl http://localhost:8080/health

# Actions manifest
curl http://localhost:8080/actions.json

# Prover funding action metadata
curl http://localhost:8080/api/actions/fund-prover
```

**Expected health check response:**
```json
{
  "status": "ok",
  "service": "zyberlink-blink-server"
}
```

---

## Production Deployment

### Option 1: Traditional VPS (DigitalOcean, Linode, AWS EC2)

#### 1. Provision Server

- Minimum specs: 1 vCPU, 1GB RAM
- Ubuntu 22.04 LTS recommended
- Open ports: 80 (HTTP), 443 (HTTPS)

#### 2. Install Dependencies

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install build essentials
sudo apt install -y build-essential pkg-config libssl-dev
```

#### 3. Clone and Build

```bash
# Clone repository
git clone https://github.com/yourusername/zyberlink.git
cd zyberlink/blink-server

# Build optimized binary
cargo build --release

# Binary location: target/release/blink-server
```

#### 4. Configure Environment

Create `/etc/zyberlink-blink.env`:

```bash
HOST=127.0.0.1
PORT=8080
BASE_URL=https://blink.yourdomain.com
RUST_LOG=info
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
```

#### 5. Create Systemd Service

Create `/etc/systemd/system/zyberlink-blink.service`:

```ini
[Unit]
Description=ZyberLink Blink Server
After=network.target

[Service]
Type=simple
User=deploy
WorkingDirectory=/home/deploy/zyberlink/blink-server
EnvironmentFile=/etc/zyberlink-blink.env
ExecStart=/home/deploy/zyberlink/target/release/blink-server
Restart=always
RestartSec=10

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/zyberlink

[Install]
WantedBy=multi-user.target
```

#### 6. Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable on boot
sudo systemctl enable zyberlink-blink

# Start service
sudo systemctl start zyberlink-blink

# Check status
sudo systemctl status zyberlink-blink

# View logs
sudo journalctl -u zyberlink-blink -f
```

#### 7. Configure Nginx Reverse Proxy

Create `/etc/nginx/sites-available/zyberlink-blink`:

```nginx
server {
    listen 80;
    server_name blink.yourdomain.com;

    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name blink.yourdomain.com;

    # SSL certificates (Let's Encrypt)
    ssl_certificate /etc/letsencrypt/live/blink.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/blink.yourdomain.com/privkey.pem;

    # Security headers
    add_header X-Content-Type-Options nosniff;
    add_header X-Frame-Options DENY;
    add_header X-XSS-Protection "1; mode=block";

    # CORS headers for Solana Actions
    add_header Access-Control-Allow-Origin *;
    add_header Access-Control-Allow-Methods "GET, POST, PUT, OPTIONS";
    add_header Access-Control-Allow-Headers "Content-Type, Authorization, Content-Encoding, Accept-Encoding";

    # Proxy to blink-server
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Enable site and reload Nginx:

```bash
# Install Certbot for SSL
sudo apt install -y certbot python3-certbot-nginx

# Get SSL certificate
sudo certbot --nginx -d blink.yourdomain.com

# Enable site
sudo ln -s /etc/nginx/sites-available/zyberlink-blink /etc/nginx/sites-enabled/

# Test configuration
sudo nginx -t

# Reload Nginx
sudo systemctl reload nginx
```

#### 8. Verify Deployment

```bash
# Test HTTPS endpoint
curl https://blink.yourdomain.com/health

# Test Blink action
curl https://blink.yourdomain.com/api/actions/fund-prover
```

---

### Option 2: Railway.app (Recommended for Quick Deploy)

Railway provides free tier with automatic HTTPS and zero-config deployment.

#### 1. Create `railway.toml`

In `/blink-server/railway.toml`:

```toml
[build]
builder = "nixpacks"
buildCommand = "cargo build --release"

[deploy]
startCommand = "./target/release/blink-server"
restartPolicyType = "always"

[env]
PORT = "3000"
HOST = "0.0.0.0"
BASE_URL = "https://your-app.railway.app"
RUST_LOG = "info"
```

#### 2. Deploy

```bash
# Install Railway CLI
npm install -g @railway/cli

# Login
railway login

# Initialize project
cd blink-server
railway init

# Deploy
railway up

# Get deployment URL
railway open
```

#### 3. Configure Environment Variables

In Railway dashboard:
- `BASE_URL`: Your railway.app URL
- `SOLANA_RPC_URL`: (optional) Custom RPC endpoint

---

### Option 3: Fly.io

Fly.io provides global edge deployment with automatic HTTPS.

#### 1. Create `fly.toml`

In `/blink-server/fly.toml`:

```toml
app = "zyberlink-blink"
primary_region = "sjc"

[build]
  dockerfile = "Dockerfile"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 1

[env]
  PORT = "8080"
  HOST = "0.0.0.0"
  RUST_LOG = "info"
```

#### 2. Create `Dockerfile`

In `/blink-server/Dockerfile`:

```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/blink-server /usr/local/bin/

EXPOSE 8080

CMD ["blink-server"]
```

#### 3. Deploy

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Login
fly auth login

# Launch app
fly launch

# Deploy
fly deploy

# Get deployment URL
fly open
```

---

## Integration with Prover Wizard

The prover setup wizard generates Blink QR codes for funding. Configuration:

### 1. Set Blink Server URL

In `prover-node` wizard, the server URL is referenced:

```rust
// prover-node/src/wizard/mod.rs
let blink_url = env::var("BLINK_SERVER_URL")
    .unwrap_or_else(|_| "https://blink.zyberlink.io".to_string());
```

### 2. Environment Variable

Set before running wizard:

```bash
export BLINK_SERVER_URL=https://your-blink-server.com
cd prover-node
cargo run --release -- wizard
```

### 3. QR Code Generation

The wizard will:
1. Generate funding URL: `https://your-blink-server.com/api/actions/fund-prover?prover={pubkey}`
2. Display QR code in terminal
3. User scans with Phantom/Solflare wallet
4. Transaction auto-populates with funding instruction

---

## Monitoring & Maintenance

### Logs

```bash
# Systemd logs (VPS deployment)
sudo journalctl -u zyberlink-blink -f

# Railway logs
railway logs

# Fly.io logs
fly logs
```

### Health Monitoring

Set up external monitoring (UptimeRobot, Pingdom):

- Endpoint: `https://your-server.com/health`
- Expected response: `{"status":"ok"}`
- Check interval: 5 minutes

### Updating

```bash
# VPS deployment
cd /home/deploy/zyberlink
git pull origin main
cd blink-server
cargo build --release
sudo systemctl restart zyberlink-blink

# Railway/Fly.io
git push origin main  # Auto-deploys on push
```

---

## Security Considerations

### 1. CORS Configuration

The server allows all origins (`*`) for Blinks to work across platforms. This is safe because:
- No authentication required
- Only serves transaction templates
- No sensitive data exposed

### 2. Rate Limiting

Recommend adding Nginx rate limiting:

```nginx
limit_req_zone $binary_remote_addr zone=blink_limit:10m rate=10r/s;

location /api/actions/ {
    limit_req zone=blink_limit burst=20 nodelay;
    # ... proxy config
}
```

### 3. DDoS Protection

Use Cloudflare or similar CDN with:
- DDoS protection enabled
- Bot management
- Rate limiting rules

### 4. HTTPS Only

Always use HTTPS in production (handled by Nginx/Railway/Fly.io).

---

## Troubleshooting

### Issue: Server won't start

**Check:**
1. Port 8080 available: `sudo lsof -i :8080`
2. Binary executable: `chmod +x target/release/blink-server`
3. Environment variables set correctly

**Solution:**
```bash
# Kill process on port
sudo kill $(sudo lsof -t -i:8080)

# Or change PORT in .env
PORT=8081 cargo run --release
```

### Issue: CORS errors in browser

**Check:**
1. CORS headers present: `curl -I https://your-server.com/api/actions/fund-prover`
2. Nginx proxy properly configured

**Solution:**
Ensure Nginx config includes:
```nginx
add_header Access-Control-Allow-Origin *;
```

### Issue: QR codes not working

**Check:**
1. BASE_URL matches deployed URL
2. Action endpoint returns valid JSON:
   ```bash
   curl https://your-server.com/api/actions/fund-prover
   ```

**Solution:**
Update BASE_URL environment variable and restart service.

### Issue: Wallet can't parse transaction

**Check:**
1. Solana RPC endpoint accessible
2. Transaction builder logic valid
3. Wallet app version supports Blinks

**Solution:**
Test with latest Phantom wallet, check RPC health.

---

## Performance Tuning

### Recommended Settings

For production under load:

```bash
# In .env or systemd EnvironmentFile
RUST_LOG=warn  # Reduce logging verbosity
```

Nginx caching (optional):

```nginx
proxy_cache_path /var/cache/nginx levels=1:2 keys_zone=blink_cache:10m inactive=60m;
proxy_cache_key "$scheme$request_method$host$request_uri";

location /api/actions/ {
    proxy_cache blink_cache;
    proxy_cache_valid 200 5m;
    # ... proxy config
}
```

---

## Support & Resources

- **Solana Actions Documentation:** https://solana.com/docs/advanced/actions
- **Blinks Specification:** https://github.com/solana-developers/solana-actions
- **Railway Docs:** https://docs.railway.app
- **Fly.io Docs:** https://fly.io/docs

---

## Changelog

- **2025-11-16:** Initial deployment guide created
- **2025-11-15:** Blink server implemented

---

**Questions or Issues?**
Open a GitHub issue or discussion in the main repository.
