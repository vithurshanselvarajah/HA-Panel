# CI rules

The workflows in this repository follow four rules. They are enforced by
`tools/check_workflows.py`, which runs in CI and fails the build on a
violation, so a rule that is not checked is a rule that is not a rule.

## Actions are pinned by commit SHA

An action is never referred to by a tag or a branch.

```yaml
# Right.
- uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1

# Wrong: the tag can be moved.
- uses: actions/checkout@v7

# Wrong: the branch can be moved, and the comment does not pin anything.
- uses: actions/checkout@v7 # 3d3c42e5aac5ba805825da76410c181273ba90b1
```

The tag goes in a comment after the SHA. The comment is what tells a
reader which release the SHA is, and what a dependabot bump will move to.

A container image is pinned by digest, the same way.

## Runners name a release

`runs-on: ubuntu-latest` is not allowed. `ubuntu-latest` moves, and a
workflow that behaves differently next month without a commit is a
workflow that cannot be debugged.

```yaml
# Right.
runs-on: ubuntu-24.04

# Wrong.
runs-on: ubuntu-latest
```

## `pull_request_target` is not used

`pull_request_target` runs with the base repository's secrets and the
pull request's code. That combination is the one that turns a fork into a
way into the repository. Every workflow here uses `pull_request` and
grants permissions per job.

## The default permission is read-only

Every workflow declares this before its first job:

```yaml
permissions:
  contents: read
```

A job that needs more says so:

```yaml
jobs:
  publish:
    permissions:
      contents: write
```

The default is read-only so that a job which forgets to ask for a
permission does not get one.

## The guard itself is tested

`tests/test_check_workflows.py` checks that the guard catches each thing
it claims to. A guard that does not catch anything passes every build
and is worse than no guard, because it looks like a check.

The same applies to the ignored Trivy findings in `ci/.trivyignore`:
every entry must carry an expiry date, so that an ignore is a decision
that gets revisited rather than one that is forgotten.
