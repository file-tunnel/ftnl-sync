import json
import os
import re
import tomllib
import unittest
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[2]
INSTALL_ROOT = "zed_modules/opto-sync/opto-sync-clients"
EXPECTED_DEPENDENCY = {
    "package": "opto-sync/opto-sync-clients",
    "range": "^0.2.0",
    "installRoot": INSTALL_ROOT,
}
KNOWN_ADAPTERS = {
    "rust": ("opto-sync-client", "clients/rust"),
    "typescript": ("@opto-sync/client", "clients/ts"),
    "dart": ("opto_sync_client", "clients/dart"),
    "gleam": ("opto_sync_client", "clients/gleam"),
}


def load_contract():
    manifest = tomllib.loads((ROOT / ".zpkg.toml").read_text(encoding="utf-8"))
    lock = tomllib.loads((ROOT / ".zpkg.lock").read_text(encoding="utf-8"))
    profile = json.loads((ROOT / "opto-sync-adapter.json").read_text(encoding="utf-8"))
    return manifest, lock, profile


class OptoSyncWrapperE2E(unittest.TestCase):
    def test_dependency_lock_and_legacy_source_provenance_fail_closed(self) -> None:
        manifest, lock, profile = load_contract()
        self.assertEqual(manifest["dependencies"]["opto-sync/opto-sync-clients"], "^0.2.0")
        self.assertEqual(manifest["install"]["dir"], "zed_modules")
        self.assertEqual(profile["dependency"], EXPECTED_DEPENDENCY)
        self.assertEqual(profile["legacySourcePins"]["opto-sync-clients"], "opto-sync-clients")
        removal_policy = profile["legacySourcePins"]["removalPolicy"].lower()
        for gate in ("source-pin", "formal-replay", "persistence", "immutable zed artifact parity"):
            self.assertIn(gate, removal_policy)
        packages = lock.get("package", [])
        if profile["releaseState"] == "blocked-until-certified-package-published":
            self.assertEqual(lock.get("version"), 1)
            self.assertEqual(packages, [])
        else:
            package = next(item for item in packages if item.get("org") == "opto-sync" and item.get("name") == "opto-sync-clients")
            for field in ("version", "sha256", "size", "format", "vcs_tag", "vcs_commit", "source"):
                self.assertTrue(package.get(field), f"missing lock field: {field}")
            self.assertRegex(package["sha256"], re.compile(r"^[0-9a-f]{64}$"))
        serialized = json.dumps(profile).lower()
        for mutable_reference in ("refs/heads/main", 'branch = "main"', "latest"):
            self.assertNotIn(mutable_reference, serialized)

    def test_native_adapter_boundary_stays_inside_the_installed_sdk(self) -> None:
        _, _, profile = load_contract()
        self.assertEqual(profile["repository"], os.environ.get("GITHUB_REPOSITORY", "file-tunnel/ftnl-sync"))
        self.assertEqual(profile["e2eRepository"], "file-tunnel/ftnl-e2e")
        for language, adapter in profile["nativeAdapters"].items():
            package, suffix = KNOWN_ADAPTERS[language]
            self.assertEqual(adapter["package"], package)
            self.assertTrue(adapter["path"].startswith(INSTALL_ROOT))
            self.assertTrue(adapter["path"].endswith(suffix))
            self.assertNotIn("..", PurePosixPath(adapter["path"]).parts)

    def test_file_tunnel_keeps_transfer_integrity_and_blob_bytes_outside_sync(self) -> None:
        _, _, profile = load_contract()
        retained = " ".join(profile["wrapperRetains"]).lower()
        for policy in (
            "session authorization",
            "range validation",
            "retry policy",
            "byte-offset monotonicity",
            "resumable-transfer idempotency",
            "integrity metadata",
            "external blob bytes",
            "formal replay adapters",
        ):
            self.assertIn(policy, retained)
        self.assertEqual(profile["persistence"]["blobPlane"], ["external-object-storage"])
        self.assertEqual(set(profile["productCollections"]), {"upload_jobs", "byte_checkpoints", "transfer_manifests", "retry_state"})
        self.assertTrue(profile["delegatesToOptoSync"])
        for invariant in ("renderLocalView", "realtimeIsWakeHint", "serverCursorIsAuthoritative", "mutableGitRefsForbidden", "removeBespokeCoreOnlyAfterParity"):
            self.assertIs(profile["invariants"].get(invariant), True)


if __name__ == "__main__":
    unittest.main()
