#!/usr/bin/env python3
"""Fail-closed applicability guard for the temporary block-buffer OSV exception."""

from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import textwrap
import tomllib
import unittest


EXPECTED = {
    "name": "block-buffer",
    "version": "0.10.4",
    "source": "registry+https://github.com/rust-lang/crates.io-index",
    "checksum": "3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71",
}
ROOT = Path(__file__).resolve().parents[2]


class GuardError(ValueError):
    """The lockfile does not satisfy the reviewed exception scope."""


def normalized_package_name(value: object) -> str:
    if not isinstance(value, str):
        return ""
    return value.replace("_", "-")


def validate_lock_bytes(contents: bytes, label: str) -> None:
    try:
        document = tomllib.loads(contents.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        raise GuardError(f"cannot parse {label} as UTF-8 TOML: {error}") from error

    packages = document.get("package")
    if not isinstance(packages, list):
        raise GuardError(f"{label} has no structural [[package]] array")

    matches = [
        package
        for package in packages
        if isinstance(package, dict)
        and normalized_package_name(package.get("name")) == "block-buffer"
    ]
    if len(matches) != 1:
        identities = [
            f"{package.get('name')!r} {package.get('version')!r}"
            for package in matches
        ]
        raise GuardError(
            f"{label} must contain exactly one block-buffer package after hyphen/underscore "
            f"normalisation; found {len(matches)} ({', '.join(identities) or 'none'})"
        )

    package = matches[0]
    differences = [
        f"{field}: expected {expected!r}, found {package.get(field)!r}"
        for field, expected in EXPECTED.items()
        if package.get(field) != expected
    ]
    if differences:
        raise GuardError(
            f"{label} block-buffer identity is outside the reviewed exception scope: "
            + "; ".join(differences)
        )


def validate_repository_lock() -> None:
    lockfile = ROOT / "Cargo.lock"
    try:
        contents = lockfile.read_bytes()
    except OSError as error:
        raise GuardError(f"cannot read required root lockfile {lockfile}: {error}") from error
    validate_lock_bytes(contents, str(lockfile))


def package_entry(**changes: str) -> str:
    fields = dict(EXPECTED)
    fields.update(changes)
    return textwrap.dedent(
        f'''\
        [[package]]
        name = "{fields['name']}"
        version = "{fields['version']}"
        source = "{fields['source']}"
        checksum = "{fields['checksum']}"
        '''
    )


def lockfile(*entries: str) -> bytes:
    return ("version = 4\n\n" + "\n".join(entries)).encode()


class LockfileApplicabilityTests(unittest.TestCase):
    def assert_rejected(self, contents: bytes) -> None:
        with self.assertRaises(GuardError):
            validate_lock_bytes(contents, "synthetic Cargo.lock")

    def test_exact_reviewed_package_passes(self) -> None:
        validate_lock_bytes(lockfile(package_entry()), "synthetic Cargo.lock")

    def test_current_repository_lock_passes(self) -> None:
        validate_repository_lock()

    def test_version_0_11_0_fails(self) -> None:
        self.assert_rejected(lockfile(package_entry(version="0.11.0")))

    def test_version_0_12_0_fails(self) -> None:
        self.assert_rejected(lockfile(package_entry(version="0.12.0")))

    def test_version_0_12_1_fails_until_exception_is_removed(self) -> None:
        self.assert_rejected(lockfile(package_entry(version="0.12.1")))

    def test_registry_source_change_fails(self) -> None:
        self.assert_rejected(
            lockfile(package_entry(source="registry+https://example.invalid/index"))
        )

    def test_checksum_change_fails(self) -> None:
        self.assert_rejected(lockfile(package_entry(checksum="0" * 64)))

    def test_additional_block_buffer_package_fails(self) -> None:
        self.assert_rejected(
            lockfile(package_entry(), package_entry(version="0.11.0"))
        )

    def test_hyphen_and_underscore_duplicate_fails(self) -> None:
        self.assert_rejected(
            lockfile(package_entry(), package_entry(name="block_buffer"))
        )

    def test_underscore_spelling_is_identified_but_not_approved(self) -> None:
        self.assert_rejected(lockfile(package_entry(name="block_buffer")))

    def test_removed_package_fails(self) -> None:
        self.assert_rejected(lockfile(package_entry(name="digest")))

    def test_malformed_lockfile_fails(self) -> None:
        self.assert_rejected(b"version = 4\n[[package]\n")


class SecurityPolicyIntegrationTests(unittest.TestCase):
    def test_policy_has_only_the_expected_five_exceptions(self) -> None:
        with (ROOT / "osv-scanner.toml").open("rb") as source:
            policy = tomllib.load(source)
        ignored = policy.get("IgnoredVulns")
        self.assertIsInstance(ignored, list)
        ids = [entry.get("id") for entry in ignored]
        self.assertEqual(
            ids,
            [
                "RUSTSEC-2025-0012",
                "RUSTSEC-2024-0384",
                "RUSTSEC-2024-0436",
                "RUSTSEC-2021-0127",
                "GHSA-qwgh-2vcv-g2f7",
            ],
        )
        self.assertEqual(ids.count("GHSA-qwgh-2vcv-g2f7"), 1)

    def test_canonical_scan_keeps_all_scanners_and_orders_guard_before_osv(self) -> None:
        scan = (ROOT / "tools/scripts/security-scan").read_text()
        required = [
            "cargo audit",
            "cargo deny check advisories licenses bans sources",
            "npm audit --omit=dev",
            '"$ROOT/tools/scripts/block-buffer-osv-guard" --test',
            '"$ROOT/tools/scripts/block-buffer-osv-guard"',
            "osv-scanner scan -L Cargo.lock -L package-lock.json",
        ]
        positions = [scan.index(command) for command in required]
        self.assertEqual(positions, sorted(positions))
        self.assertIn("set -euo pipefail", scan)
        self.assertNotIn("osv-scanner scan -L Cargo.lock -L package-lock.json ||", scan)

    def fake_scan(self, guard_exit: int, osv_exit: int) -> tuple[subprocess.CompletedProcess[str], bool]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scripts = root / "tools/scripts"
            scripts.mkdir(parents=True)
            shutil.copy2(ROOT / "tools/scripts/security-scan", scripts / "security-scan")
            guard = scripts / "block-buffer-osv-guard"
            guard.write_text(
                "#!/usr/bin/env bash\n"
                "if [[ \"${1:-}\" == \"--test\" ]]; then exit 0; fi\n"
                f"exit {guard_exit}\n"
            )
            guard.chmod(0o755)

            fake_bin = root / "fake-bin"
            fake_bin.mkdir()
            for tool in ("cargo", "cargo-audit", "cargo-deny", "npm"):
                executable = fake_bin / tool
                executable.write_text("#!/usr/bin/env bash\nexit 0\n")
                executable.chmod(0o755)
            marker = root / "osv-called"
            osv = fake_bin / "osv-scanner"
            osv.write_text(
                "#!/usr/bin/env bash\n: > \"$OSV_MARKER\"\n"
                f"exit {osv_exit}\n"
            )
            osv.chmod(0o755)
            environment = dict(os.environ)
            environment["PATH"] = f"{fake_bin}:/usr/bin:/bin"
            environment["OSV_MARKER"] = str(marker)
            result = subprocess.run(
                [str(scripts / "security-scan")],
                cwd=root,
                env=environment,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                check=False,
            )
            return result, marker.exists()

    def test_guard_failure_prevents_osv_success(self) -> None:
        result, osv_called = self.fake_scan(guard_exit=42, osv_exit=0)
        self.assertEqual(result.returncode, 42, result.stdout)
        self.assertFalse(osv_called, result.stdout)

    def test_unrelated_osv_finding_remains_blocking(self) -> None:
        result, osv_called = self.fake_scan(guard_exit=0, osv_exit=23)
        self.assertEqual(result.returncode, 23, result.stdout)
        self.assertTrue(osv_called, result.stdout)


def main() -> int:
    if sys.argv[1:] == ["--test"]:
        suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        return 0 if result.wasSuccessful() else 1
    if sys.argv[1:]:
        print("usage: block-buffer-osv-guard [--test]", file=sys.stderr)
        return 2
    try:
        validate_repository_lock()
    except GuardError as error:
        print(f"block-buffer OSV exception guard failed: {error}", file=sys.stderr)
        return 1
    print(
        "block-buffer OSV exception guard passed: exactly one locked "
        "block-buffer 0.10.4 matches the reviewed registry source and checksum"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
