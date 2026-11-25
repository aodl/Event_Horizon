#!/usr/bin/env node
import { createFileStore } from '../src/file-store.mjs';
import { loadCheckpoint, publicError } from '../src/runtime.mjs';
import { runService } from '../src/service.mjs';
import { serviceOptions } from '../src/service-config.mjs';

let options;
try {
  if (!process.versions.bun && Number(process.versions.node.split('.')[0]) !== 24) {
    throw new Error('Use Node.js 24 for source execution');
  }
  options = await serviceOptions(process.argv.slice(2));
  if (options.validate) {
    if (options.checkpointToValidate !== undefined) {
      await loadCheckpoint(createFileStore(options.checkpointToValidate), options.config);
    }
    console.log('Offline validation passed. Host provisioning and poke permission were not checked.');
  } else {
    await runService(options);
  }
} catch (error) {
  console.error(`Worker stopped: ${publicError(error, [options?.env, process.env])}`);
  process.exitCode = 1;
}
