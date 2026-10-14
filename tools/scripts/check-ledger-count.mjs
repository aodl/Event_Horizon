#!/usr/bin/env node
// Opt-in, anonymous, read-only production-verification smoke; never part of CI.
import { createLedgerCounterClient } from '../../canisters/frontend/public/ledger-counter-client.js';
import { INSTANCES } from '../../canisters/frontend/public/instances.js';
import { ICP_API_HOST } from '../../canisters/frontend/public/pricing-client.js';
if (!process.argv.includes('--live')) throw new Error('Pass --live to authorize two read-only ICP ledger queries.');
const entry = INSTANCES[0], requests = [];
const client = createLedgerCounterClient({ fetchImpl: (url, options) => {
  requests.push({ utc: new Date().toISOString(), url: String(url) }); return fetch(url, options);
} });
const samples = [];
try {
  for (let i = 0; i < 2; i++) {
    const abort = new AbortController(), deadline = setTimeout(() => abort.abort(), 10000);
    try {
      const total = await client.query(entry, { signal: abort.signal });
      samples.push({ utc: new Date().toISOString(), total: total.toString(), success: true });
    } catch (error) {
      samples.push({ utc: new Date().toISOString(), success: false, error: error.message });
      process.exitCode = 1;
    } finally { clearTimeout(deadline); }
    if (i === 0) await new Promise(resolve => setTimeout(resolve, 2000));
  }
} finally { client.dispose(); }
console.log(JSON.stringify({ host: ICP_API_HOST, principal: entry.observedLedgerCanisterId,
  method: 'query_blocks', request: { start: '0 : nat64', length: '0 : nat64' },
  verification: 'Pinned SDK mainnet root key and default query-signature/certificate verification', samples, requests }, null, 2));
