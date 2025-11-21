# ZyberLink Frontend - Installation & Setup

## Quick Start

### 1. Install Dependencies
```bash
cd /home/deploy/experimental/zyberlink-zcash/webapp
npm install
```

### 2. Run Development Server
```bash
npm run dev
```

App will be available at: http://localhost:5173

### 3. Run Tests
```bash
npm test
```

### 4. Build for Production
```bash
npm run build
npm run preview  # Preview production build
```

---

## What Was Added (Agent B Frontend Track)

### New Components
- `PaymentMethodSelector.svelte` - Payment method selector (SOL/wZEC)
- `Toast.svelte` - Toast notification system
- `Tooltip.svelte` - Informative tooltips
- `Loading.svelte` - Loading spinners

### New Utilities
- `tokenAccountManager.js` - SPL Token account management
- `toast.js` - Toast store
- `api.js` (mocks) - Mock API client

### New Tests
- `paymentSelector.test.js` - Payment selector tests
- `tokenAccountManager.test.js` - Token manager tests

### Modified Files
- `CreateJob.svelte` - Integrated payment method selector
- `package.json` - Added Solana dependencies

---

## Dependencies Installed

### Production
- @solana/web3.js - Solana blockchain interaction
- @solana/spl-token - SPL Token operations
- @solana/wallet-adapter-base - Wallet adapter base
- @solana/wallet-adapter-wallets - Wallet implementations

### Development
- vitest - Testing framework
- @testing-library/svelte - Component testing
- jsdom - DOM environment for tests

---

## Environment Setup (Optional)

Create `.env` file for configuration:

```env
# Solana RPC Endpoint
VITE_SOLANA_RPC_URL=https://api.devnet.solana.com

# Backend API (Agent A)
VITE_API_BASE_URL=http://localhost:8080

# wZEC Mint Address
VITE_WZEC_MINT=sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ
```

---

## Verification Checklist

After installation, verify:

- [ ] `npm install` completes without errors
- [ ] `npm run dev` starts dev server
- [ ] App loads at http://localhost:5173
- [ ] Navigate to Create Job page
- [ ] Payment method selector visible in Step 2
- [ ] SOL and wZEC options clickable
- [ ] `npm test` runs tests successfully
- [ ] No console errors in browser

---

## Integration with Backend

When Agent A's backend is ready:

1. Update API calls in components
2. Replace mock imports with real fetch
3. Configure CORS if needed
4. Test with real Solana devnet

Example API call replacement:
```javascript
// Before (mock)
import { mockApi } from '$lib/mocks/api';
const response = await mockApi.createJob(data);

// After (real)
const response = await fetch('/api/jobs/create', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(data)
});
```

---

## Troubleshooting

### "Cannot find module @solana/web3.js"
Run: `npm install`

### Tests fail with "jsdom not found"
Run: `npm install --save-dev jsdom`

### Component not rendering
Check browser console for errors
Verify imports are correct

### Wallet connection issues
Ensure wallet adapter is installed
Check wallet extension is enabled

---

## Next Steps

1. Install dependencies: `npm install`
2. Start dev server: `npm run dev`
3. Test payment selector functionality
4. Run tests: `npm test`
5. Read component docs: `src/lib/components/README.md`
6. Review completion report: `AGENT_B_COMPLETION_REPORT.md`

---

## Support

For issues or questions:
- Check `AGENT_B_COMPLETION_REPORT.md`
- Review component README
- Check test files for usage examples

---

**Status:** Ready for development and testing
**Last Updated:** 2025-11-20
