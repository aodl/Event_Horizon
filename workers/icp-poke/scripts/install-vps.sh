#!/usr/bin/env bash
# Stage or update an inactive systemd service on Debian/Ubuntu.
set -euo pipefail
umask 077

fail() { printf '%s\n' "$*" >&2; exit 2; }

worker_check_only=false
if [[ "${1:-}" == '--check' ]]; then
  worker_check_only=true
  shift
fi
[[ $# -eq 1 ]] || fail 'Usage: bash scripts/install-vps.sh [--check] /path/to/worker.settings.json'
worker_settings="$1"
worker_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
worker_binary="$worker_root/eh-poke-linux-x86_64"
worker_unit="$worker_root/systemd/event-horizon-poke.service"
worker_destination='/opt/event-horizon-worker'
worker_config_dir='/etc/event-horizon'
worker_installed_settings="$worker_config_dir/worker.settings.json"
worker_service='event-horizon-poke.service'
worker_unit_target="/etc/systemd/system/$worker_service"
worker_state='/var/lib/event-horizon-worker'

[[ -f "$worker_settings" && ! -L "$worker_settings" ]] || fail 'Expected a regular, nonsymlinked settings file'
[[ -f "$worker_binary" && -x "$worker_binary" && ! -L "$worker_binary" ]] || fail 'Extract the VPS installation archive first'
# Shared parser, identity/start checks, and filesystem permission checks.
validate() { env -i "$worker_binary" --validate --settings "$worker_settings" "$@" || exit 2; }
if "$worker_check_only"; then
  validate
  exit
fi

[[ "$(id -u)" == 0 ]] || fail 'Run as root on the VPS, or use --check locally'
[[ -x /usr/bin/flock && -d /run/systemd/system && -x /usr/bin/systemctl ]] || fail 'The VPS requires systemd and util-linux'
[[ -f "$worker_unit" && ! -L "$worker_unit" && ! -L "$worker_root/systemd" ]] || fail 'Expected a regular unit file'

check_inactive() {
  local active
  active="$(/usr/bin/systemctl show --property=ActiveState --value "$worker_service")"
  case "$active" in
    inactive|failed) ;;
    *) fail 'Stop event-horizon-poke before installing an update; its checkpoint will be retained' ;;
  esac
}
check_inactive

# Installation writes only below trusted root-owned directories. The state
# directory is service-owned and is never changed by this installer.
for directory in /opt /etc /etc/systemd /etc/systemd/system /var/lib "$worker_destination" "$worker_config_dir"; do
  [[ ! -L "$directory" ]] || fail "Unsafe symlink: $directory"
  if [[ -e "$directory" ]]; then
    [[ -d "$directory" && -O "$directory" ]] || fail "Expected a root-owned directory: $directory"
    mode="$(stat -c '%a' "$directory")"
    (( (8#$mode & 0022) == 0 )) || fail "Directory is writable by other users: $directory"
  fi
done
[[ ! -L "$worker_state" && ( ! -e "$worker_state" || -d "$worker_state" ) ]] || fail 'Unsafe state directory'
for target in "$worker_destination/eh-poke-linux-x86_64" "$worker_installed_settings" "$worker_unit_target" "$worker_state/checkpoint.json" "$worker_state/worker.lock"; do
  [[ ! -L "$target" && ( ! -e "$target" || -f "$target" ) ]] || fail "Unsafe installation target: $target"
done

# Snapshot the input before validating it or touching installed files.
worker_stage="$(mktemp -d /tmp/eh-worker-install.XXXXXX)"
worker_temporary=''
trap 'if [[ -n "$worker_temporary" ]]; then rm -f -- "$worker_temporary"; fi; rm -rf -- "$worker_stage"' EXIT
validate
cp --no-dereference --preserve=mode -- "$worker_settings" "$worker_stage/settings.json"
worker_settings="$worker_stage/settings.json"
if [[ -e "$worker_installed_settings" ]]; then
  validate --compare-settings "$worker_installed_settings" --checkpoint "$worker_state/checkpoint.json"
else
  [[ ! -e "$worker_state/checkpoint.json" ]] || fail 'Restore the original installed settings before updating this checkpoint'
  validate
fi

if id -u ehworker >/dev/null 2>&1; then
  [[ "$(id -u ehworker)" != 0 && "$(getent passwd ehworker | cut -d: -f7)" == */nologin ]] || fail 'Existing ehworker account must be a non-login system account'
  [[ "$(id -gn ehworker)" == ehworker ]] || fail 'Existing ehworker account must have its own ehworker primary group'
else
  ! getent group ehworker >/dev/null || fail 'Unexpected existing ehworker group'
fi
check_inactive
if ! id -u ehworker >/dev/null 2>&1; then
  useradd --system --user-group --no-create-home --home-dir /nonexistent --shell /usr/sbin/nologin ehworker
fi

install -d -m 0755 -o root -g root "$worker_destination"
install -d -m 0750 -o root -g ehworker "$worker_config_dir"
replace_file() {
  local source="$1" target="$2" mode="$3" group="$4"
  worker_temporary="$(mktemp -- "${target}.XXXXXX")"
  install -T -m "$mode" -o root -g "$group" -- "$source" "$worker_temporary"
  mv -fT -- "$worker_temporary" "$target"
  worker_temporary=''
}
replace_file "$worker_binary" "$worker_destination/eh-poke-linux-x86_64" 0755 root
replace_file "$worker_settings" "$worker_installed_settings" 0640 ehworker
replace_file "$worker_unit" "$worker_unit_target" 0644 root
/usr/bin/systemctl daemon-reload
printf '\nInstalled; service activation was not changed.\n'
printf 'After authorizing this principal: systemctl enable --now event-horizon-poke\n'
printf 'Logs: journalctl -u event-horizon-poke -f\n'
