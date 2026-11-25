import { createHash, randomUUID } from 'node:crypto';
import { constants } from 'node:fs';
import { open, mkdir, rename, unlink } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';

const MAX_CHECKPOINT_BYTES = 8 * 1024 * 1024;

async function readCheckpoint(path, signal) {
  signal?.throwIfAborted();
  let file;
  try {
    file = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
  } catch (error) {
    if (error.code === 'ENOENT') return null;
    throw error;
  }
  try {
    const metadata = await file.stat();
    if (!metadata.isFile() || metadata.size > MAX_CHECKPOINT_BYTES) {
      throw new Error('Checkpoint must be a regular file no larger than 8 MiB');
    }
    const raw = await file.readFile();
    signal?.throwIfAborted();
    if (!raw.length || raw.length > MAX_CHECKPOINT_BYTES) throw new Error('Checkpoint is empty or too large');
    let state;
    try { state = JSON.parse(raw.toString('utf8')); } catch {
      throw new Error('Checkpoint JSON is malformed');
    }
    return { state, etag: createHash('sha256').update(raw).digest('hex') };
  } finally {
    await file.close();
  }
}

/** Atomic local checkpoint for one flock-protected process. The digest detects
 * unexpected manual edits; it is not a cross-process compare-and-swap lock. */
export function createFileStore(filename, { signal } = {}) {
  if (typeof filename !== 'string' || !filename.startsWith('/') || filename.includes('\0')) {
    throw new Error('EH_CHECKPOINT_PATH must be an absolute file path');
  }
  const path = resolve(filename);
  return {
    async load() {
      return (await readCheckpoint(path, signal)) ?? { state: null, etag: null };
    },
    async save(state, previousEtag) {
      signal?.throwIfAborted();
      if (typeof previousEtag !== 'string' && previousEtag !== null) {
        throw new Error('Checkpoint expected digest is malformed');
      }
      const folder = dirname(path);
      await mkdir(folder, { recursive: true, mode: 0o700 });
      const present = await readCheckpoint(path, signal);
      if ((present?.etag ?? null) !== previousEtag) {
        throw new Error('Checkpoint changed unexpectedly; refusing stale write');
      }
      const payload = Buffer.from(`${JSON.stringify(state)}\n`);
      if (payload.length > MAX_CHECKPOINT_BYTES) throw new Error('Checkpoint exceeds 8 MiB limit');
      const temporary = `${path}.${randomUUID()}.tmp`;
      let file;
      try {
        file = await open(temporary, 'wx', 0o600);
        await file.writeFile(payload);
        await file.sync();
        await file.close();
        file = undefined;
        signal?.throwIfAborted();
        const latest = await readCheckpoint(path, signal);
        if ((latest?.etag ?? null) !== previousEtag) {
          throw new Error('Checkpoint changed before commit; refusing stale write');
        }
        await rename(temporary, path);
        const directory = await open(folder, 'r');
        try { await directory.sync(); } finally { await directory.close(); }
      } finally {
        if (file) await file.close();
        await unlink(temporary).catch(error => { if (error.code !== 'ENOENT') throw error; });
      }
    },
  };
}
