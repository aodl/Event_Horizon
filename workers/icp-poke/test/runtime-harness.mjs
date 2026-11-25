// Standalone test-only entry. The production service never imports this file.
import assert from 'node:assert/strict';
import { createPublicKey, verify } from 'node:crypto';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { stat } from 'node:fs/promises';
import { resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { Cbor, IC_REQUEST_DOMAIN_SEPARATOR, requestIdOf } from '@icp-sdk/core/agent';
import { IDL } from '@icp-sdk/core/candid';
import { readConfig } from '../src/config.mjs';
import { initialState, runPass } from '../src/core.mjs';
import { createFileStore } from '../src/file-store.mjs';
import { createBoundedFetch, createIcpClient } from '../src/icp.mjs';
import { loadCheckpoint } from '../src/runtime.mjs';
import { pollLoop } from '../src/poll-loop.mjs';

const config = readConfig({
  EH_IDENTITY_SEED: Buffer.alloc(32, 42).toString('base64'),
  EH_START_BLOCK: '9007199254740993',
});
const store = createFileStore(resolve(process.argv[3]));

async function checkRuntime() {
  let envelope;
  const client = await createIcpClient({ ...config, pokeTimeoutMs: 500 }, {
    fetchImpl: async (url, options) => {
      if (url.pathname.endsWith('/read_state')) {
        await delay(5000, undefined, { signal: options.signal });
        assert.fail('Deadline did not interrupt the status request');
      }
      assert.ok(url.pathname.endsWith('/call'));
      assert.equal(envelope, undefined);
      envelope = Cbor.decode(options.body);
      return new Response(null, { status: 202 });
    },
  });

  // A synthetic acceptance response cannot satisfy the certified update.
  const state = { ...initialState(config), globalInterest: true };
  await assert.rejects(runPass({
    config, state, client: {
      readPage: async () => ({
        chainLength: config.startBlock + 1n,
        blocks: [{ transaction: { operation: [] } }],
      }),
      poke: () => client.poke(),
    },
    persist: next => store.save(next, null),
  }), /abort|timeout/i);
  assert.equal((await store.load()).etag, null);

  const { content, sender_pubkey: der, sender_sig: signature } = envelope;
  assert.equal(content.method_name, 'poke');
  assert.deepEqual(IDL.decode([], content.arg), []);
  assert.ok(verify(null, Buffer.concat([IC_REQUEST_DOMAIN_SEPARATOR, requestIdOf(content)]),
    createPublicKey({ key: Buffer.from(der), format: 'der', type: 'spki' }), signature));

  const controller = new AbortController();
  const received = Promise.withResolvers();
  const server = createServer(() => { received.resolve(); }).listen(0, '127.0.0.1');
  await once(server, 'listening');
  try {
    const fetch = createBoundedFetch(globalThis.fetch, { signal: controller.signal });
    const request = fetch(`http://127.0.0.1:${server.address().port}/`);
    const aborted = assert.rejects(request, /abort/i);
    await received.promise;
    controller.abort();
    await aborted;
  } finally {
    server.closeAllConnections();
    await new Promise(done => server.close(done));
  }

  await store.save(state, null);
  assert.equal((await stat(resolve(process.argv[3]))).mode & 0o777, 0o600);
  const loaded = await loadCheckpoint(store, config);
  assert.deepEqual(loaded.state, state);
  await assert.rejects(store.save(state, null), /stale write/);
  await store.save(null, loaded.etag);
  await assert.rejects(loadCheckpoint(store, config), /Checkpoint/);
  console.log('Compiled modules passed signed SDK request, Candid, cancellation, and checkpoint checks');
}

async function measure(workload) {
  // Four full Candid-decoded 1,000-block pages per catch-up pass, or one
  // empty ledger reply when idle. The fixture uses the production ledger DID.
  const { ledgerIdl } = await import('../src/idl.mjs');
  const replyType = ledgerIdl({ IDL })._fields[0][1].retTypes[0];
  const shutdown = new AbortController();
  const begin = performance.now();
  const block = {
    parent_hash: [], timestamp: { timestamp_nanos: 1n }, transaction: {
      memo: 0n, icrc1_memo: [], operation: [], created_at_time: { timestamp_nanos: 1n },
    },
  };
  const transfer = {
    ...block,
    transaction: { ...block.transaction, operation: [{ Transfer: {
      from: new Uint8Array(32).fill(1), to: new Uint8Array(32).fill(2),
      amount: { e8s: 100_000_000n }, fee: { e8s: 10_000n }, spender: [],
    } }] },
  };
  const idle = workload === 'idle';
  const limit = idle ? 10 : 8;
  const tip = config.startBlock + (idle ? 0n : BigInt(limit * 4_000));
  // Build the payload once, as if it were the gateway's response.
  const data = IDL.encode([replyType], [{
    blocks: idle ? [] : Array.from({ length: 1000 }, (_, i) => i % 4 === 0 ? block : transfer),
    chain_length: tip, first_block_index: 0n, archived_blocks: [],
  }]);
  let passes = 0;
  let blocks = 0;
  await pollLoop({
    signal: shutdown.signal,
    onError: error => { throw error; },
    run: async () => {
      await createIcpClient(config); // Builds the real identity and SDK actors each pass.
      const loaded = await loadCheckpoint(store, config);
      const result = await runPass({
        ...loaded, config,
        client: {
          readPage: async () => {
            const [reply] = IDL.decode([replyType], data);
            return { chainLength: reply.chain_length, blocks: reply.blocks };
          },
        },
        persist: next => store.save(next, loaded.etag),
      });
      assert.equal(result.blocksRead, idle ? 0 : 4_000);
      blocks += result.blocksRead;
      if (++passes === limit) shutdown.abort();
    },
  });
  console.log(JSON.stringify({
    workload, passes, blocks, elapsedMs: Math.round(performance.now() - begin),
    maxRssKiB: process.resourceUsage().maxRSS, cpu: process.cpuUsage(),
  }));
}

const mode = process.argv[2];
if (mode === 'check') await checkRuntime();
else if (mode === 'idle' || mode === 'catch-up') await measure(mode);
else throw new Error('Unknown test fixture');
