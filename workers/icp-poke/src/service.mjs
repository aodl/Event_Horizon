import { createServer } from 'node:http';
import { pollLoop } from './poll-loop.mjs';
import { publicError, runWorker, WorkerPassError } from './runtime.mjs';
import packageInfo from '../package.json' with { type: 'json' };

/** SIGTERM stops future passes and closes HTTP. The active bounded pass can
 * finish its acknowledged poke and checkpoint before process exit. */
export async function runService({
  env, httpPort, run = runWorker, log = event => console.log(JSON.stringify(event)),
}) {
  const shutdown = new AbortController();
  const status = { version: packageInfo.version, state: 'starting', lastSuccessfulPass: null };
  let server;
  let closed;
  let listenerFailed = false;
  const sockets = new Set();
  function closeListener() {
    if (!server?.listening || closed) return;
    closed = new Promise(resolve => server.close(resolve));
    for (const socket of sockets) socket.destroy();
  }
  function stop() {
    status.state = 'stopping';
    shutdown.abort();
    closeListener();
  }

  process.on('SIGTERM', stop);
  process.on('SIGINT', stop);
  try {
    if (httpPort !== undefined) {
      server = createServer({ requestTimeout: 10_000, headersTimeout: 5_000 }, (req, res) => {
        // This handler only reads the last completed pass's status.
        if (req.url === '/healthz' && (req.method === 'GET' || req.method === 'HEAD')) {
          res.writeHead(status.state === 'running' ? 200 : 503, {
            'content-type': 'application/json', 'cache-control': 'no-store',
          });
          res.end(JSON.stringify(status));
        } else {
          res.writeHead(req.url === '/healthz' ? 405 : 404, { connection: 'close' }).end();
        }
      });
      server.on('connection', socket => {
        sockets.add(socket);
        socket.once('close', () => sockets.delete(socket));
      });
      await new Promise((resolve, reject) => {
        const bindFailed = () => reject(new Error('Could not bind worker HTTP listener'));
        server.once('error', bindFailed);
        server.listen(httpPort, '0.0.0.0', () => {
          server.off('error', bindFailed);
          server.on('error', () => { listenerFailed = true; stop(); });
          resolve();
        });
      });
    }
    if (shutdown.signal.aborted) return;
    const worker = publicError({ message: env.EH_WORKER_NAME ?? 'event-horizon-worker' }, env);
    log({ event: 'worker_started', worker, cadenceMs: 1000, httpPort: server?.address()?.port });
    await pollLoop({
      signal: shutdown.signal,
      run: async () => {
        const summary = await run({ env, log });
        status.lastSuccessfulPass = new Date().toISOString();
        if (summary) {
          status.nextBlock = summary.toBlock;
          status.lagBlocks = summary.lagBlocks;
        }
        if (!shutdown.signal.aborted) status.state = 'running';
      },
      onError(error) {
        if (!(error instanceof WorkerPassError)) throw error;
        if (!shutdown.signal.aborted) status.state = 'retrying';
      },
    });
    if (listenerFailed) throw new Error('Worker HTTP listener failed');
    log({ event: 'worker_stopped', worker });
  } finally {
    stop();
    try { await closed; } finally {
      process.off('SIGTERM', stop);
      process.off('SIGINT', stop);
    }
  }
}
