# Demo Script

## Overview

**Duration:** 3 minutes total (90 seconds demo, 90 seconds explanation)
**Format:** Live side-by-side comparison
**Objective:** Visceral demonstration of 10x speed improvement

---

## Pre-Demo Preparation

### Equipment Checklist

**Phones (2x):**
- [ ] Identical models (same specs for fair comparison)
- [ ] Fully charged (100% battery)
- [ ] Screen recording enabled (for backup video)
- [ ] Battery percentage visible in status bar
- [ ] Screen brightness identical
- [ ] Notifications disabled

**Left Phone - Traditional Zcash Wallet:**
- [ ] Reference wallet installed (Nighthawk, Zecwallet Lite, or similar)
- [ ] Wallet synced and funded
- [ ] Test transaction prepared
- [ ] Previous transactions cleared (clean history)

**Right Phone - CypherWallet:**
- [ ] CypherWallet installed
- [ ] Wallet synced and funded
- [ ] Connected to devnet marketplace
- [ ] Same test transaction prepared

**Desktop Prover Node:**
- [ ] Running and visible on large screen/projector
- [ ] TUI showing live stats (jobs claimed, proofs generated)
- [ ] Wallet funded with stake
- [ ] Registered on marketplace

**Screens/Projector:**
- [ ] Phone screen mirroring (both phones visible)
- [ ] Prover node visible
- [ ] Solana Explorer open (show state changes)
- [ ] Timer visible (for precise measurement)

**Backup:**
- [ ] Pre-recorded video (in case live demo fails)
- [ ] Backup phones (in case of technical issues)
- [ ] Local validator running (in case of network issues)

---

## Demo Script

### Introduction (15 seconds)

**[Show both phones side-by-side on screen]**

**Narration:**
> "I'm going to send the same Zcash shielded transaction from two phones. On the left, a traditional Zcash wallet that generates proofs locally. On the right, CypherWallet, which uses our decentralized marketplace. Watch what happens."

**[Hold up both phones, show they're real devices]**

---

### Act 1: Setup (15 seconds)

**[Show wallet balances on both phones]**

**Left Phone:**
- Open traditional wallet
- Navigate to "Send" screen
- Enter recipient address
- Enter amount (e.g., 0.01 ZEC)
- Show "Send" button ready

**Right Phone:**
- Open CypherWallet
- Navigate to "Send" screen
- Enter same recipient address
- Enter same amount (0.01 ZEC)
- Show "Send" button ready

**Narration:**
> "Same transaction, same amount, same recipient. Everything identical except the proving method. Let's see the difference."

---

### Act 2: The Race (90 seconds)

**[Start timer visible on screen: 00:00]**

**Action:**
**Simultaneously press "Send" on both phones**

---

#### Time: 00:00 - 00:05

**Both phones:**
- Building transaction
- Generating witness

**Narration:**
> "Both phones are generating the witness—the private data needed for the proof. This is fast on both."

**[Show loading indicators on both phones]**

---

#### Time: 00:05 - 00:08

**Left Phone (Traditional):**
- "Generating proof..." (progress bar at 0%)
- Phone starts warming up

**Right Phone (CypherWallet):**
- "Creating marketplace job..." (smooth animation)
- "Encrypting witness..."
- "Posting to Solana..."

**Narration:**
> "Now the difference begins. The left phone starts generating the proof locally—this will take a while. The right phone encrypts the witness and posts a job to our marketplace."

**[Switch screen to show Solana Explorer]**

---

#### Time: 00:08 - 00:10

**Solana Explorer:**
- Show job creation transaction confirmed
- Show compressed state (job details)

**Prover Node (visible on desktop):**
- TUI updates: "New job detected"
- "Claiming job..."
- "Job claimed successfully"

**Right Phone:**
- "Prover found! (Desktop-Pro-Node-42)"
- "Proof generation in progress..."
- Progress indicator (smooth, with estimated time)

**Narration:**
> "Job posted! A prover node—running on someone's desktop—claims the job within a second. Now that powerful CPU starts generating the proof."

**[Switch back to both phones side-by-side]**

---

#### Time: 00:10 - 00:20

**Left Phone (Traditional):**
- Progress bar: ~10%
- "Generating proof..." (still)
- Phone noticeably warm to touch
- Battery: 100% → 99%

**Right Phone (CypherWallet):**
- "Proof generation in progress..."
- Countdown: "~12 seconds remaining"
- Shows prover info (reputation, past jobs)

**Prover Node (Desktop, visible if space allows):**
- TUI showing: "Generating proof for Job #1234"
- CPU usage graph (high utilization)
- Progress indicator

**Narration:**
> "The left phone is working hard—you can feel it heating up. Meanwhile, the desktop prover is doing the heavy lifting for the right phone. The mobile device stays cool."

**[Hold up left phone to camera, show it's warm]**

---

#### Time: 00:20 - 00:22

**Right Phone (CypherWallet):**
- "Proof received!"
- "Verifying proof..." (fast, <1 second)
- "Proof valid!"
- "Broadcasting transaction..."
- Success animation (checkmark, confetti)
- "Transaction sent! ✓"

**Left Phone (Traditional):**
- Progress bar: ~15%
- Still generating proof...
- Battery: 99% → 98%

**Narration:**
> "And done! CypherWallet completed the transaction in 22 seconds. The traditional wallet? Still generating the proof. Let's wait for it to finish so you can see the full comparison."

**[Zoom in on right phone showing success screen]**

---

#### Time: 00:22 - 02:30 (Traditional wallet catching up)

**[Optional: Speed up time-lapse if demo time is limited]**

**Right Phone:**
- Show transaction confirmation screen
- Battery: Still at 100% (used 0.3%)
- Show transaction in Zcash Explorer (confirmed)

**Left Phone:**
- Progress bar slowly climbing (20%, 30%, 40%...)
- Battery draining (98%, 97%, 96%...)
- Phone hot to touch

**Narration (during wait):**
> "While we wait, let me show you what happened under the hood."

**[Switch to architecture diagram on screen]**

> "The mobile client generated the witness and encrypted it with the prover's public key using post-quantum cryptography. The prover never saw the private data—just the encrypted witness. After generating the proof, the prover submitted it to Solana, the marketplace verified it, and payment was automatically released."

**[Switch back to phones]**

> "And here we are, still waiting for the traditional wallet..."

---

#### Time: 02:30 - 02:35

**Left Phone (Traditional):**
- Progress bar: 95%, 98%, 100%
- "Proof complete!"
- "Broadcasting transaction..."
- Success screen
- Battery: 100% → 97% (3% used)

**Narration:**
> "Finally! The traditional wallet completes the transaction after 2 minutes and 35 seconds."

---

### Act 3: The Comparison (20 seconds)

**[Show final comparison on screen]**

```
┌─────────────────────┬──────────────────┬──────────────────┐
│ Metric              │ Traditional      │ CypherWallet     │
├─────────────────────┼──────────────────┼──────────────────┤
│ Total Time          │ 155 seconds      │ 22 seconds       │
│ Battery Used        │ 3.0%             │ 0.3%             │
│ Device Temperature  │ Hot              │ Cool             │
│ User Experience     │ Frustrating      │ Smooth           │
└─────────────────────┴──────────────────┴──────────────────┘
```

**Narration:**
> "The results: CypherWallet is **7x faster** with **90% less battery usage**. Same security guarantees, same privacy level, but actually usable on mobile."

**[Show both phones side-by-side, CypherWallet on success screen, traditional just finishing]**

---

### Act 4: The Proof (15 seconds)

**[Switch to Solana Explorer]**

**Show:**
1. Job creation transaction
2. Job claim transaction
3. Proof submission transaction
4. Payment release transaction

**Narration:**
> "Everything is on-chain and verifiable. Here's the job creation, the prover claiming it, the proof submission, and the automatic payment. Fully decentralized, no centralized server."

**[Switch to prover node desktop]**

**Show:**
- Prover dashboard (TUI)
- "Jobs completed: 1" (or higher if multiple demos)
- "Earnings today: 0.018 SOL" (or equivalent in dollars, e.g., "$0.018")
- Reputation score (e.g., "⭐ 4.9/5.0")

**Narration:**
> "The prover earned money for their work—passive income for idle hardware. And their reputation increased, making them more likely to be chosen for future jobs."

---

### Act 5: Verification (10 seconds)

**[Switch to Zcash Explorer]**

**Show:**
- Both transactions confirmed on Zcash network
- Same block height (or close)
- Both shielded (privacy preserved)

**Narration:**
> "Both transactions are confirmed on the Zcash network. Same privacy guarantees, both fully shielded. The only difference is user experience."

---

### Conclusion (15 seconds)

**[Return to both phones side-by-side]**

**Narration:**
> "This is CypherLink: decentralized ZK compute that makes privacy practical. We built this in 21 days. Imagine what's possible when every mobile ZK application uses this infrastructure. Thank you."

**[Final shot: CypherWallet success screen]**

---

## Narration Notes

### Tone & Delivery
- **Confident but not arrogant:** "Here's what we built" not "We're the best"
- **Enthusiastic but professional:** Show excitement without being gimmicky
- **Clear and concise:** Short sentences, avoid jargon
- **Conversational:** Like explaining to a friend, not a textbook

### Emphasis Points
- **"10x faster"** - Pause after saying this, let it sink in
- **"Same security guarantees"** - Reassure judges we didn't cheat
- **"Decentralized"** - Core value proposition
- **"21 days"** - Emphasize execution speed

### Body Language
- **Hold up phones** - Make it tangible, not just abstract
- **Point to screens** - Direct attention where you want it
- **Smile during success** - Show genuine excitement
- **Stay calm during wait** - Don't fidget, use time to explain

---

## Backup Plans

### If Live Demo Fails

**Scenario 1: Network issues**
- Switch to local validator (pre-prepared)
- Explain: "Using local testnet for reliability"
- Continue demo as planned

**Scenario 2: Phone crashes**
- Switch to backup phone (identical setup)
- Explain: "Technical difficulties, switching to backup"
- Restart demo from beginning

**Scenario 3: Prover node offline**
- Show fallback to local proving in CypherWallet
- Explain: "No provers available, falling back to local"
- Still faster than traditional (optimized circuit)

**Scenario 4: Complete failure**
- Play pre-recorded video
- Narrate over video (same script)
- Explain: "This was recorded yesterday, here's what happens"
- Show code and architecture after video

### If Time is Short

**90-second version:**
- Skip Act 1 setup (phones already on send screen)
- Start timer immediately
- Skip waiting for traditional wallet to finish
  - Show pre-recorded final time instead
- Skip detailed explanation
- Jump straight to comparison

**60-second version:**
- Play pre-recorded video at 2x speed
- Narrate key points only
- Show final comparison
- "Happy to answer questions or show live demo after"

---

## Post-Demo Q&A Preparation

### Likely Questions

**Q: Can you show the code?**
**A:** [Have GitHub repo open in background]
- Show program code (Solana smart contract)
- Show SDK integration
- Show wallet FFI bridge
- Explain key components

**Q: How does encryption work?**
**A:** [Have architecture diagram ready]
- Show key exchange (ML-KEM)
- Explain encrypted witness
- Show prover never sees plaintext
- Verify with code walkthrough

**Q: What if I don't trust the prover?**
**A:**
- Explain verification happens locally
- Prover can't fake proof (cryptographically impossible)
- Worst case: prover refuses, you fall back to local
- Show reputation system (slashing for bad behavior)

**Q: How do you handle prover selection?**
**A:** [Show marketplace UI or code]
- Reputation-based selection
- Users can choose (automatic or manual)
- SLAs for enterprise
- Show prover dashboard

### Technical Deep-Dive Option

If judges want details, have ready:
- Code walkthrough (5 minutes)
- Architecture deep-dive (5 minutes)
- Security model explanation (5 minutes)
- Performance profiling results (3 minutes)

---

## Rehearsal Checklist

### Pre-Rehearsal
- [ ] Script memorized (not reading)
- [ ] Equipment tested (all devices working)
- [ ] Backup video recorded
- [ ] Timing practiced (3 minutes exactly)

### During Rehearsal
- [ ] Record yourself (watch for awkward moments)
- [ ] Time each section (ensure no overruns)
- [ ] Practice with distractions (simulate real environment)
- [ ] Get feedback (from someone who hasn't seen it)

### Post-Rehearsal
- [ ] Identify weak points (unclear explanations)
- [ ] Simplify jargon (make it accessible)
- [ ] Polish transitions (smooth flow)
- [ ] Prepare for Q&A (anticipate questions)

**Target: 10+ rehearsals before live presentation**

---

## Day-Of Checklist

### Morning (4 hours before)
- [ ] Charge all devices (100%)
- [ ] Test all equipment
- [ ] Deploy fresh to devnet (if changes made)
- [ ] Fund all wallets (prover, mobile)
- [ ] Clear transaction history (clean slate)
- [ ] Run through demo 2-3 times

### 1 Hour Before
- [ ] Re-charge devices (ensure 100%)
- [ ] Connect to venue WiFi (or use hotspot)
- [ ] Test screen mirroring
- [ ] Position cameras (if recording)
- [ ] Final rehearsal (abbreviated)

### 15 Minutes Before
- [ ] Calm down (breathe, relax)
- [ ] Review key points (mental checklist)
- [ ] Set phones to Do Not Disturb
- [ ] Open all necessary screens
- [ ] Ready backup video (just in case)

### Immediately Before
- [ ] Smile
- [ ] Confidence check
- [ ] "This is going to be great"

---

## Success Criteria

**Demo succeeds if:**
- ✅ Both transactions complete (even if traditional takes long)
- ✅ Side-by-side comparison visible and clear
- ✅ Timing difference obvious (ideally 10x, minimum 5x)
- ✅ No crashes or major bugs
- ✅ Judges say "wow" or equivalent

**Demo exceeds expectations if:**
- ✅ Judges ask to try it themselves
- ✅ Audience claps spontaneously
- ✅ Judges ask detailed technical questions (shows interest)
- ✅ Video goes viral after event
- ✅ Partnership conversations start immediately

---

## Final Thoughts

**Remember:**
- The demo sells itself—the contrast is visceral
- Don't oversell—let the results speak
- Stay calm during the wait—it builds tension
- Be honest about limitations—integrity matters
- Have fun—this is cool technology!

**The goal:** Make judges feel the pain of the slow wallet, then the relief of the fast one.

**The outcome:** "I want this. When can I use it?"

---

Good luck! You've got this.
