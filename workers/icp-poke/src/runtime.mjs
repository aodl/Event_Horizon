import { readConfig } from './config.mjs';
import { initialState, validateState, runPass } from './core.mjs';
import { createIcpClient, validateInstance } from './icp.mjs';
import { createFileStore } from './file-store.mjs';

let running = false;
let lastHeartbeatHour = -1;
let verifiedBackend;

export async function loadCheckpoint(store, config) {
  const loaded = await store.load();
  // JSON null in an existing file is corrupt state, not a missing checkpoint.
  const state = loaded.etag === null ? initialState(config) : loaded.state;
  validateState(state, config);
  return { state, etag: loaded.etag };
}

export function publicError(error, env = {}) {
  let message = String(error?.message ?? 'Worker failed');
  const environments = Array.isArray(env) ? env : [env];
  const secrets = environments.flatMap((settings) => [
    settings?.EH_IDENTITY_SEED,
  ]).filter(value => typeof value === 'string' && value.length > 0).sort((a, b) => b.length - a.length);
  // Redact every identity before truncation: truncating first could leave a
  // partial seed that no subsequent whole-value replacement can recognize.
  for (const secret of secrets) message = message.split(secret).join('[redacted]');
  return message.replace(/[\r\n]+/g, ' ').slice(0, 600);
}

// Pass errors keep their cursor eligible for retry. Failures in service
// orchestration (including the single-flight guard and logging) are fatal.
export class WorkerPassError extends Error {}

export async function runWorker({ env = process.env, log = (event) => console.log(JSON.stringify(event)) } = {}) {
  // The service loop awaits each pass. Guard accidental overlap in this process;
  // the systemd unit uses flock to enforce one writer per VPS checkpoint.
  if (running) throw new Error('A worker invocation is already running');
  const config = readConfig(env);
  const worker = publicError({ message: config.workerName }, env);
  running = true;
  const controller = new AbortController();
  let timeout;
  let summary;
  try {
    timeout = setTimeout(() => controller.abort(new Error('Worker invocation deadline exceeded')), config.runTimeoutMs);
    const client = await createIcpClient(config, { signal: controller.signal });
    const verificationKey = `${config.host}/${config.backend}`;
    if (verifiedBackend !== verificationKey) {
      validateInstance(await client.getInstance(), config);
      verifiedBackend = verificationKey;
    }
    const store = createFileStore(env.EH_CHECKPOINT_PATH, { signal: controller.signal });
    const { state, etag } = await loadCheckpoint(store, config);
    summary = await runPass({
      state, config, client,
      persist: (next) => store.save(next, etag),
    });
  } catch (error) {
    const message = publicError(error, env);
    log({ event: 'worker_error', worker, message });
    // Do not pass the SDK's full request/configuration object to platform logging.
    throw new WorkerPassError(message);
  } finally {
    clearTimeout(timeout);
    controller.abort();
    running = false;
  }
  const hour = Math.floor(Date.now() / 3_600_000);
  if (summary.poked || lastHeartbeatHour !== hour) {
    log({ event: summary.poked ? 'wake' : 'heartbeat', worker, ...summary });
    lastHeartbeatHour = hour;
  }
  return summary;
}
