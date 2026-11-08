"""The workflow guard must catch each thing it claims to, or it passes vacuously."""

import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "check_workflows", Path(__file__).resolve().parents[1] / "tools" / "check_workflows.py")
cw = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cw)

GOOD = """name: x
on: [push]
permissions:
  contents: read
jobs:
  a:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
      - uses: ./local-action
"""

def errors_for(text: str) -> list[str]:
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "wf.yml"
        p.write_text(text)
        return cw.check(p)

class CheckWorkflows(unittest.TestCase):
    def test_good_passes(self):
        self.assertEqual(errors_for(GOOD), [])

    def test_a_branch_fails(self):
        self.assertTrue(errors_for(GOOD.replace("@v7", "@master")))

    def test_a_sha_fails(self):
        # A SHA is not a version, and the action it names would not know
        # which toolchain to install.
        self.assertTrue(errors_for(GOOD.replace("@v7", "@3d3c42e5aac5ba805825da76410c181273ba90b1")))

    def test_a_version_in_a_comment_does_not_count(self):
        self.assertTrue(errors_for(GOOD.replace(
            "@v7", "@master # v7")))

    def test_a_ref_with_no_version_fails(self):
        self.assertTrue(errors_for(GOOD.replace("@v7", "")))

    def test_a_dotted_version_passes(self):
        self.assertEqual(errors_for(GOOD.replace("@v7", "@v4.39.3")), [])

    def test_a_prerelease_version_passes(self):
        self.assertEqual(errors_for(GOOD.replace("@v7", "@v2.0.0-rc1")), [])

    def test_the_stable_branch_passes_for_a_toolchain(self):
        # The rust-toolchain action's stable branch is how a toolchain is
        # named without an explicit input.
        self.assertEqual(errors_for(GOOD), [])
        self.assertEqual(
            errors_for(GOOD.replace("actions/checkout@v7", "dtolnay/rust-toolchain@stable")),
            [],
        )

    def test_the_stable_branch_fails_for_another_action(self):
        self.assertTrue(errors_for(GOOD.replace("@v7", "@stable")))

    def test_a_local_action_passes(self):
        self.assertEqual(errors_for(GOOD), [])

    def test_docker_tag_fails_digest_passes(self):
        self.assertTrue(errors_for(GOOD + "      - uses: docker://alpine:3.20\n"))
        self.assertEqual(
            errors_for(GOOD + "      - uses: docker://alpine@sha256:" + "a" * 64 + "\n"), [])

    def test_latest_runner_fails(self):
        self.assertTrue(errors_for(GOOD.replace("ubuntu-24.04", "ubuntu-latest")))

    def test_missing_permissions_fails(self):
        self.assertTrue(errors_for(GOOD.replace("permissions:\n  contents: read\n", "")))

    def test_job_level_permissions_do_not_count_as_default(self):
        text = GOOD.replace("permissions:\n  contents: read\n", "").replace(
            "    runs-on", "    permissions:\n      contents: read\n    runs-on")
        self.assertTrue(errors_for(text))

    def test_pull_request_target_fails(self):
        self.assertTrue(errors_for(GOOD.replace("on: [push]", "on: [push, pull_request_target]")))
        self.assertTrue(errors_for(GOOD.replace("on: [push]", "on:\n  pull_request_target:")))


class Trivyignore(unittest.TestCase):
    """Every ignored finding has to expire."""

    def errors_for(self, text: str) -> list[str]:
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / ".trivyignore"
            p.write_text(text)
            return cw.check_trivyignore(p)

    def test_a_dated_entry_passes(self):
        self.assertEqual(self.errors_for("CVE-2026-12345 2026-12-31 because\n"), [])

    def test_a_permanent_entry_fails(self):
        self.assertTrue(self.errors_for("CVE-2026-12345\n"))

    def test_an_entry_without_a_reason_fails(self):
        self.assertTrue(self.errors_for("CVE-2026-12345 2026-12-31\n"))

    def test_comments_are_ignored(self):
        self.assertEqual(self.errors_for("# just a comment\n"), [])


if __name__ == "__main__":
    unittest.main()
