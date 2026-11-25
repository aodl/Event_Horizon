# Nibrun with manually enabled always-on activation

Use the same `eh-poke-linux-x86_64` from the [shared build](README.md#build-and-test).
The guest supports Linux x86-64 with glibc, assigns the HTTP port, and persists
`/app/data` across redeploys. The default guest's 256 MiB RAM and 64 MiB
temporary filesystem make the resource and restart checks below relevant.
[Nibrun guest contract](https://github.com/ilbertt/nibrun/blob/main/skills/deploy-to-nibrun/SKILL.md#the-guest-contract)

## Owner deployment

1. Create or prepare the intended app using Nibrun's deployment UI or
   [prepared app workflow](https://github.com/ilbertt/nibrun/blob/main/skills/deploy-to-nibrun/SKILL.md#3-deploy).
   Before the first boot, enter the chosen worker's `EH_IDENTITY_SEED`,
   `EH_START_BLOCK`, and any other worker settings directly in the app's
   private environment configuration. Keep the original boundary with the
   prepared identity. Keep settings out of CLI arguments and upload artifacts.
2. Ask the operator to apply the reported arrangement to enable always-on at
   no charge for this specific app. Continuous operation depends on this
   app-level setting. Confirm it with the operator directly.
   [Activation contract](https://github.com/ilbertt/nibrun/blob/main/apps/api/src/lib/deployments/desired-state.ts)
   [Operator guidance](https://github.com/ilbertt/nibrun/blob/main/skills/deploy-to-nibrun/SKILL.md#tradeoffs)
3. Upload the raw executable, checking its SHA-256 against the build. Leave
   app arguments empty for the standard setup. The worker binds
   `0.0.0.0:$NIBRUN_HTTP_PORT`. Its default checkpoint is
   `$NIBRUN_DATA_DIR/checkpoint.json`, normally `/app/data/checkpoint.json`.
   If migrating a worker with saved progress, stop the original installation
   first and import its own trusted checkpoint into the root of the persistent
   data volume through the creation-time data import workflow. A checkpoint
   override must be an absolute file path inside `NIBRUN_DATA_DIR`. The worker
   rejects symlinks within the checkpoint path on that volume at startup.
4. On the trusted deployment computer, check app activation before relying on
   continuous polling. Substitute the real app name. The status JSON must show
   `"activation": "always"`. [CLI status command](https://github.com/ilbertt/nibrun/blob/main/packages/cli/src/commands/apps/status.ts)
   [CLI status fields](https://github.com/ilbertt/nibrun/blob/main/packages/cli/src/lib/app-status.ts)

   ```bash
   nib apps status --app APP_NAME --json
   nib apps logs --app APP_NAME
   ```

   The provider checks TCP connectivity. Observe fresh successful passes and
   a moving checkpoint, using `/healthz`, logs and state. A public status
   response contains operational metadata only.

## Updates and hosted checks

Re-deploy the binary to the **same app**, keeping private configuration,
original start and data volume. With a configured CLI you can run
`nib run ./dist/eh-poke-linux-x86_64 --app APP_NAME` from
`workers/icp-poke` on the trusted computer. Recheck `activation`, confirm
the checkpoint resumes from its saved cursor, and inspect the
provider logs. A new independent app or clone requires a distinct identity,
allowlist permission, its own checkpoint and its own manual always-on
enablement. Exports and backups can contain private environment values and
checkpoint data. Keep them private and out of starter archives.
[Nibrun redeploy and export guidance](https://github.com/ilbertt/nibrun/blob/main/skills/deploy-to-nibrun/SKILL.md)

Observe progress for at least 15 minutes without sending incoming app HTTP
requests. Check logs or state before and after the window and confirm useful
progress well past the ordinary five-minute idle boundary. Check restart and
redeploy, then record pass timings, lag, CPU and memory during steady polling
and catch-up. Provider crashes and redeploys can still interrupt a pass.
Follow the [full hosted acceptance checklist](VALIDATION.md#hosted-acceptance).

A future deploy button needs a published, versioned executable asset. Its
setup instructions must require private configuration, manual always-on
activation and backend caller authorisation.
