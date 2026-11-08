#!/usr/bin/env python3
"""Enforce the workflow pinning rules.

The rules are the ones the CI rules in CONTRIBUTING.md state:

- every action is pinned by commit SHA, never by a tag or a branch;
- every runner names a release, never "latest";
- `pull_request_target` is not used, because it runs with the base
  repository's secrets and a pull request's code;
- the workflow declares a read-only default permission before its first
  job, so that a job which forgets to ask for more does not get more.

Stdlib only, so it runs before anything is installed. Exit status 1 lists
every violation.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

SHA = re.compile(r"@[0-9a-f]{40}$")

def check(path: Path) -> list[str]:
    text = path.read_text()
    errors = []

    for n, line in enumerate(text.splitlines(), 1):

        code = line.split(" #", 1)[0]

        m = re.search(r"\buses:\s*(\S+)", code)
        if m:
            ref = m.group(1).strip("'\"")
            if not ref.startswith("./") and not SHA.search(ref):
                if ref.startswith("docker://"):
                    if "@sha256:" not in ref:
                        errors.append(f"{path.name}:{n}: image not pinned by digest: {ref}")
                else:
                    errors.append(f"{path.name}:{n}: action not pinned by commit SHA: {ref}")

        m = re.search(r"\bruns-on:\s*(\S+)", code)
        if m and m.group(1).endswith("-latest"):
            errors.append(f"{path.name}:{n}: runner must name a release, not {m.group(1)}")

        if re.search(r"^\s*pull_request_target\s*:", code) or re.search(r"\[.*\bpull_request_target\b.*\]", code):
            errors.append(f"{path.name}:{n}: pull_request_target is not allowed")

    head = text.split("\njobs:", 1)[0]
    if not re.search(r"^permissions:\s*\n\s+contents:\s*read\s*$", head, re.M):
        errors.append(f"{path.name}: top-level 'permissions: contents: read' is missing")

    return errors

def check_trivyignore(path: Path) -> list[str]:
    """Every ignored finding has to expire.

    A permanent ignore is a decision that is never revisited, and the
    expiry date is what forces the decision to be revisited.
    """
    if not path.exists():
        return []

    errors = []
    for n, line in enumerate(path.read_text().splitlines(), 1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        if not re.match(r"^CVE-\d{4}-\d+\s+\d{4}-\d{2}-\d{2}\s+.+", stripped):
            errors.append(
                f"{path.name}:{n}: ignored finding must be 'CVE-YYYY-NNNN YYYY-MM-DD reason': {stripped}"
            )
    return errors

def main() -> int:
    workflows = sorted((ROOT / ".github" / "workflows").glob("*.y*ml"))
    if not workflows:
        print("no workflows found; nothing would be checked", file=sys.stderr)
        return 1

    errors = [e for wf in workflows for e in check(wf)]
    errors += check_trivyignore(ROOT / "ci" / ".trivyignore")

    for e in errors:
        print(e, file=sys.stderr)
    if not errors:
        print(f"{len(workflows)} workflow(s) pass")
    return 1 if errors else 0

if __name__ == "__main__":
    sys.exit(main())
