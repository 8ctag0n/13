# Prover Node TUI (Terminal User Interface)

Beautiful real-time monitoring dashboard for the Zyberlink prover node.

## Features

- 📊 **Live Statistics**: Jobs, earnings, reputation, uptime
- 🎯 **Recent Jobs List**: Last 10 jobs with status and timing
- 🎨 **Color-coded Output**: Green/Red/Yellow for different states
- ⚡ **Real-time Updates**: Refreshes every 100ms
- ⌨️ **Keyboard Controls**: Press 'q' or ESC to quit

## Usage

### Enable TUI Mode

Add the `--tui-mode` flag when starting the prover:

```bash
cargo run --release -- \
    --rpc-url http://localhost:8899 \
    --program-id <PROGRAM_ID> \
    --keypair ~/.config/solana/id.json \
    --tui-mode
```

### Screenshot

```
┌────────────────────────────────────────────────────────────┐
│        🔐 Zyberlink Prover Node - LIVE                     │
└────────────────────────────────────────────────────────────┘
┌─ System ───────────────────────────────────────────────────┐
│ Status: 🟢 ACTIVE | RPC: 12ms | Block: 458 | Uptime: 5m 23s│
└────────────────────────────────────────────────────────────┘
┌─ Jobs ─────────────────────────────────────────────────────┐
│ Jobs:  0 pending | 1 claimed | 15 done | 0 failed         │
└────────────────────────────────────────────────────────────┘
┌─ Performance ──────────────────────────────────────────────┐
│ Earnings: 0.0450 SOL | Reputation: 950/1000 | Avg Time: 12.1s │
└────────────────────────────────────────────────────────────┘
┌─ Recent Jobs (last 10) ────────────────────────────────────┐
│ ✅ #15   ZK Proof     12.1s  +0.0030 SOL                  │
│ ✅ #14   FHE Poly     18.4s  +0.0050 SOL                  │
│ 🔄 #16   ZK Proof     2.3s elapsed...                      │
│ ⏸  #17   Pending assignment                                │
└────────────────────────────────────────────────────────────┘
┌────────────────────────────────────────────────────────────┐
│             Press 'q' or ESC to quit                        │
└────────────────────────────────────────────────────────────┘
```

## Architecture

### Components

- **TUIState**: Shared state between prover and TUI
  - `ProverStats`: Live metrics (jobs, earnings, reputation)
  - `RecentJob`: Job history for display
  - `should_quit`: Signal for graceful shutdown

- **TUIApp**: Main TUI application
  - Renders UI using ratatui
  - Handles keyboard input
  - Updates display every 100ms

### Data Flow

```
ProverNode (async task)
    ↓ updates
TUIState (Arc<Mutex<>>)
    ↓ reads
TUIApp (main thread)
    ↓ renders
Terminal (crossterm)
```

## Stats Tracked

### System Stats
- Status: Active/Idle
- RPC latency
- Current blockchain slot
- Uptime

### Job Stats
- Pending jobs: Waiting for assignment
- Claimed jobs: In progress
- Completed jobs: Successfully finished
- Failed jobs: Errors or timeouts

### Performance Stats
- Total earnings (in SOL)
- Reputation score (0-1000)
- Average proof generation time
- Success rate

### Recent Jobs
- Job ID
- Job type (ZK Proof, FHE Poly, etc.)
- Status (completed, failed, claimed)
- Duration
- Earnings per job

## Development

### Adding New Stats

1. Update `ProverStats` struct in `src/tui/mod.rs`:
```rust
pub struct ProverStats {
    // ... existing fields
    pub new_stat: u64,
}
```

2. Update rendering in `TUIApp::render_stats()`:
```rust
let new_stat_text = format!("New Stat: {}", stats.new_stat);
```

3. Update stats from prover:
```rust
tui_state.update_stats(|stats| {
    stats.new_stat = value;
});
```

### Adding New Panels

1. Add constraint in `TUIApp::render()`:
```rust
let chunks = Layout::default()
    .constraints([
        // ... existing
        Constraint::Length(5),  // New panel height
    ])
    .split(size);
```

2. Create render function:
```rust
fn render_new_panel(&self, f: &mut Frame, area: Rect) {
    let widget = Paragraph::new("Content")
        .block(Block::default().borders(Borders::ALL).title(" New Panel "));
    f.render_widget(widget, area);
}
```

3. Call in render:
```rust
self.render_new_panel(f, chunks[N]);
```

## Dependencies

- `ratatui` (0.26): TUI framework
- `crossterm` (0.27): Terminal control

These are automatically included when building the prover node.

## Keyboard Shortcuts

Current:
- `q`: Quit
- `ESC`: Quit

Future (TODO):
- `p`: Pause job processing
- `r`: Reset stats
- `↑`/`↓`: Scroll job list
- `f`: Filter jobs by type

## Performance

- CPU usage: <1% when idle
- Memory: ~10MB for TUI structures
- Update frequency: 10 FPS (100ms refresh)
- No impact on proof generation performance

## Troubleshooting

### TUI doesn't render
```bash
# Check terminal supports colors
echo $TERM
# Should output: xterm-256color or similar

# Try with explicit TERM
TERM=xterm-256color cargo run --release -- --tui-mode ...
```

### Colors look wrong
```bash
# Use different color scheme
# Edit src/tui/mod.rs and change Color:: values
```

### Screen flickers
```bash
# Reduce refresh rate
# Edit TUIApp::run() poll timeout to 200ms instead of 100ms
```

### Can't quit with 'q'
```bash
# Try ESC key
# Or send SIGTERM: kill <PID>
```

## Future Enhancements

- [ ] Scrollable job list (currently fixed to 10)
- [ ] Interactive job selection (click to view details)
- [ ] Performance graphs (earnings over time)
- [ ] Network visualization (connected provers)
- [ ] Configuration hot-reload (change settings without restart)
- [ ] Multiple tab support (stats, logs, config)
- [ ] Mouse support (click to interact)
- [ ] Export stats to CSV/JSON

## Comparison with Web Dashboard

| Feature | TUI | Web Dashboard |
|---------|-----|---------------|
| Setup time | 0s (built-in) | ~30s (build + serve) |
| Resource usage | Minimal | 50MB+ (node + browser) |
| Remote access | SSH only | HTTP (any device) |
| Polish level | Good | Excellent |
| Interactivity | Keyboard | Mouse + keyboard |

**Use TUI for:**
- Local development
- SSH sessions
- Low-resource environments
- Quick monitoring

**Use Web Dashboard for:**
- Remote monitoring
- Presentations/demos
- Multiple viewers
- Advanced visualizations

---

**Last updated:** 2025-11-14
