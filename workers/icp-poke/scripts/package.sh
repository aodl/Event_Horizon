#!/usr/bin/env bash
set -euo pipefail

worker_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
worker_bun="$(command -v "${BUN:-bun}")" || { echo 'Install Bun 1.4.5 or set BUN to its executable' >&2; exit 1; }
worker_bun="$(realpath "$worker_bun")"
python3 - "$worker_root" "$worker_bun" <<'PY'
from pathlib import Path
import hashlib
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile

root = Path(sys.argv[1])
bun = sys.argv[2]
target = 'bun-linux-x64'
binary = 'eh-poke-linux-x86_64'
archive = 'eh-poke-vps-linux-x86_64.tar.gz'
record = 'BUILD.txt'
runtime = [
    'package.json', 'package-lock.json', 'scripts/run-service.mjs',
    'src/accounts.mjs', 'src/config.mjs', 'src/core.mjs', 'src/file-store.mjs',
    'src/icp.mjs', 'src/idl.mjs', 'src/poll-loop.mjs', 'src/runtime.mjs',
    'src/service-config.mjs', 'src/service.mjs',
]
support = ['scripts/install-vps.sh', 'systemd/event-horizon-poke.service']
inputs = runtime + support + ['scripts/package.sh']
for name in inputs:
    path = root / name
    if not path.is_file() or path.is_symlink() or path.resolve() != path:
        raise SystemExit(f'Invalid package input: {name}')


def digest(path):
    checksum = hashlib.sha256()
    with path.open('rb') as stream:
        for data in iter(lambda: stream.read(1024 * 1024), b''):
            checksum.update(data)
    return checksum.hexdigest()


repo = root.parent.parent
revision = subprocess.check_output(['git', '-C', repo, 'rev-parse', 'HEAD'], text=True).strip()
changes = subprocess.run(['git', '-C', repo, 'diff', '--quiet', 'HEAD', '--']).returncode
if changes not in (0, 1):
    raise SystemExit('Could not determine local Git changes')
dist = root / 'dist'
if dist.is_symlink():
    raise SystemExit('Invalid package output directory')
dist.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix='.build-', dir=dist) as directory:
    stage = Path(directory)
    home = stage / 'home'
    home.mkdir()
    env = {'PATH': os.environ['PATH'], 'HOME': str(home), 'TMPDIR': str(home), 'LC_ALL': 'C'}
    for name in inputs:
        output = stage / name
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(root / name, output)
    provenance = [f'{digest(stage / name)}  {name}' for name in sorted(inputs)]

    def command(*args):
        return subprocess.check_output(args, cwd=stage, env=env, text=True).strip()

    if command(bun, '--version') != '1.4.5':
        raise SystemExit('Bun 1.4.5 is required')
    subprocess.run(['npm', 'ci', '--ignore-scripts', '--no-audit', '--no-fund', '--omit=dev'],
                   cwd=stage, env=env, check=True)
    subprocess.run([bun, 'build', '--compile', f'--target={target}',
                    '--no-compile-autoload-dotenv', '--no-compile-autoload-bunfig',
                    'scripts/run-service.mjs', '--outfile', binary], cwd=stage, env=env, check=True)
    (stage / record).write_text('\n'.join([
        f'Git base {revision}',
        f'Local tracked changes {"present" if changes else "absent"}',
        f'Bun {command(bun, "--revision")}',
        f'Node {command("node", "--version")}',
        f'npm {command("npm", "--version")}',
        f'Target {target} (Linux x86-64, glibc, SSE4.2)',
        'Dependency tree pinned in package-lock.json',
        f'Executable SHA-256 {digest(stage / binary)}',
        '', 'Build and installer input SHA-256', *provenance, '',
    ]))
    with tarfile.open(stage / archive, 'w:gz') as package:
        for name in [binary, *support, record]:
            package.add(stage / name, arcname=name, recursive=False)
    sums = 'SHA256SUMS'
    (stage / sums).write_text(''.join(f'{digest(stage / name)}  {name}\n' for name in [binary, archive, record]))
    for name in [binary, archive, record, sums]:
        os.replace(stage / name, dist / name)
    print((dist / sums).read_text(), end='')
PY
