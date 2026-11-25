# Webdock and netcup VPS setup

The commands below are for the owner during deployment. Start with the
[shared build](README.md#build-and-test) and reuse the [prepared settings and
boundary](START_HERE.md). The two provider labels identify prepared files only.
The deployment computer can go offline once the service is running.

## Trusted deployment computer

Choose a Linux x86-64 VPS with SSE4.2, systemd, Bash, util-linux, and outbound
HTTPS. Ubuntu 24.04 is suitable. Use SSH key access and retain provider console
access. The VPS runs the executable directly. The default service opens no
inbound HTTP port.

Verify `dist/SHA256SUMS` as described in the build guide. Transfer
`eh-poke-vps-linux-x86_64.tar.gz` to a private staging directory on each VPS.
Compare its SHA-256 on arrival. Put only the chosen worker's settings in that
directory via your private provisioning channel with mode 0600. Keep
controller credentials on the trusted computer. Use the backend's documented
[caller authorisation procedure](../../canisters/event-horizon/README.md#configure-worker-wakes)
separately before relying on signed wakes.

## Each VPS, as root

Use a private staging directory for extraction. The initial installation
creates a disabled, inactive service. Enable it only after the explicit
authorisation checks.

```bash
cd /path/to/private/staging
umask 077
sha256sum eh-poke-vps-linux-x86_64.tar.gz
tar -xzf eh-poke-vps-linux-x86_64.tar.gz
bash scripts/install-vps.sh --check "$PWD/worker.settings.json"
bash scripts/install-vps.sh "$PWD/worker.settings.json"
systemctl is-enabled event-horizon-poke
systemctl is-active event-horizon-poke
systemd-analyze verify /etc/systemd/system/event-horizon-poke.service
stat -c '%a %U:%G %n' /etc/event-horizon /etc/event-horizon/worker.settings.json
```

Expect `disabled`, `inactive`, a `root:ehworker` settings directory at 0750
and file at 0640. The root-owned executable lives at
`/opt/event-horizon-worker/eh-poke-linux-x86_64`. A non-login `ehworker`
process holds a local `flock` and uses
`/var/lib/event-horizon-worker/checkpoint.json` under a 0700 state directory
created by systemd. Keep the staging settings private and keep a trusted
backup outside the replaceable install directory. If moving an existing worker,
stop its prior process and restore its own protected checkpoint before enabling
the service, with owner `ehworker:ehworker` and mode 0600.

When authorised, start the service and observe progress.

```bash
systemctl enable --now event-horizon-poke
systemctl status event-horizon-poke --no-pager
journalctl -u event-horizon-poke -f
```

Check `heartbeat` and `wake` events for `toBlock` and `lagBlocks`.
Inspect the checkpoint's `startBlock` and `nextBlock` locally over time. It
appears after the first committed progress. `worker_error` reports failed
attempts that back off and replay. Stats such as CPU, memory and elapsed
intervals need to be checked while idle and catching up. Follow the
[hosted acceptance checklist](VALIDATION.md#hosted-acceptance).

```bash
stat -c '%a %U:%G %n' /var/lib/event-horizon-worker{,/checkpoint.json}
grep -oE '"(startBlock|nextBlock)"[[:space:]]*:[[:space:]]*"[0-9]+"' \
  /var/lib/event-horizon-worker/checkpoint.json
systemctl show -p MemoryCurrent -p CPUUsageNSec event-horizon-poke
```

## VPS updates and recovery

Stage and verify the new executable archive first. On the VPS, run from the
newly extracted archive directory as root:

```bash
systemctl stop event-horizon-poke
bash scripts/install-vps.sh /etc/event-horizon/worker.settings.json
systemctl start event-horizon-poke
```

An update rejects a running service and changes to the installed identity,
backend or original start. The installer validates an existing checkpoint
offline without moving it. Update one worker at a time. Before recovery or
state restore, stop the affected service. Restore that worker's own trusted
checkpoint with protected ownership and mode. If state is lost or corrupt,
preserve any corrupt file for diagnosis and replay from the original
`EH_START_BLOCK`; replays may duplicate wakes. A checkpoint from another
installation can encode different learned interests and progress.

To retire an installation, run `systemctl disable --now event-horizon-poke`.
Handle hosting cancellation and removal of the principal's backend permission
through their separate owner workflows.
