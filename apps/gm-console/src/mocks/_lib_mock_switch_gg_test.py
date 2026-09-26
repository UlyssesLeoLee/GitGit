#!/usr/bin/env python3
"""GitGit MSW mock_switch reader tests (per ULYS-190 §4.4 stage6 cross-project pattern)."""

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]  # src/mocks/ → gm-console/
ACI = ROOT / "src" / "mocks" / ".aci.json"
CLUSTER = ROOT / "src" / "mocks" / ".mock-cluster.json"
HELPER = ROOT / "src" / "mocks" / "_lib_mock_switch_gg.py"


class TestGitGitModuleSwitch(unittest.TestCase):
    def setUp(self):
        self.aci = json.loads(ACI.read_text(encoding="utf-8"))
        self.cluster = json.loads(CLUSTER.read_text(encoding="utf-8"))

    def test_aci_file_exists(self):
        self.assertTrue(ACI.exists(), f".aci.json missing at {ACI}")

    def test_cluster_file_exists(self):
        self.assertTrue(CLUSTER.exists(), f".mock-cluster.json missing at {CLUSTER}")

    def test_helper_exists(self):
        self.assertTrue(HELPER.exists())

    def test_module_switch_total_count_is_7(self):
        plugins = self.aci["plugins"]
        total = sum(len(p.get("modules", {})) for p in plugins.values())
        self.assertEqual(total, 7, f"expected 7 modules across 3 plugins, got {total}")

    def test_module_switch_all_enabled_by_default(self):
        for pid, pconf in self.aci["plugins"].items():
            for mid, mconf in pconf.get("modules", {}).items():
                self.assertTrue(mconf.get("enabled", False), f"{pid}.{mid}")

    def test_health_plugin_has_1_module(self):
        mods = self.aci["plugins"]["health"]["modules"]
        self.assertEqual(set(mods.keys()), {"heartbeat"})

    def test_repo_plugin_has_3_modules(self):
        mods = self.aci["plugins"]["repo"]["modules"]
        self.assertEqual(set(mods.keys()), {"list_repos", "get_repo", "get_refs"})

    def test_vault_plugin_has_3_modules(self):
        mods = self.aci["plugins"]["vault"]["modules"]
        self.assertEqual(set(mods.keys()), {"list_keys", "key_detail", "versions"})

    def test_cluster_enabled_true_and_mode_offline(self):
        self.assertEqual(self.cluster["enabled"], True)
        self.assertEqual(self.cluster["mode"], "offline")

    def test_aci_compat_version_consistent_across_cluster_and_aci(self):
        cv = self.cluster.get("aci_compat_version")
        av = self.aci.get("aci_compat_version")
        self.assertEqual(cv, av)

    def test_plugins_count_matches_summary_plugins_total(self):
        self.assertEqual(len(self.aci["plugins"]), 3)

    def test_run_helper_read_plugins(self):
        proc = subprocess.run(
            [sys.executable, str(HELPER), "--aci-config", str(ACI), "read-plugins"],
            capture_output=True, text=True, cwd=str(ROOT),
        )
        self.assertEqual(proc.returncode, 0, f"helper failed: {proc.stderr}")
        out = json.loads(proc.stdout)
        self.assertEqual(out["plugins_total"], 3)
        self.assertEqual(out["modules_total"], 7)
        self.assertEqual(out["modules_enabled"], 7)

    def test_run_helper_trace(self):
        proc = subprocess.run(
            [sys.executable, str(HELPER), "--aci-config", str(ACI), "trace"],
            capture_output=True, text=True, cwd=str(ROOT),
        )
        self.assertEqual(proc.returncode, 0)
        self.assertIn("=7/7 modules", proc.stdout)

    def test_run_helper_validate_compat(self):
        proc = subprocess.run(
            [sys.executable, str(HELPER), "--aci-config", str(ACI), "validate-compat"],
            capture_output=True, text=True, cwd=str(ROOT),
        )
        self.assertEqual(proc.returncode, 0)
        self.assertIn("ACI_COMPAT=OK", proc.stdout)

    def test_cluster_module_count_total_7(self):
        self.assertEqual(self.cluster.get("module_count_total"), 7)

    def test_cluster_module_count_enabled_7(self):
        self.assertEqual(self.cluster.get("module_count_enabled"), 7)


if __name__ == "__main__":
    unittest.main(verbosity=2)
