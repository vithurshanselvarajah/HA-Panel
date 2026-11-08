# CI rules

The workflows in this repository follow four rules. They are enforced by
`tools/check_workflows.py`, which runs in CI and fails the build on a
violation, so a rule that is not checked is a rule that is not a rule.

## Actions name a version

An action is referred to by a version tag, never by a branch and never by
a commit SHA.

```yaml
# Right.
- uses: actions/checkout@v7
- uses: actions/setup-node@v7
- uses: github/codeql-action/analyze@v4

# Wrong: a branch moves, and the build changes with no commit.
- uses: actions/checkout@master

# Wrong: a SHA is not a version, and the action it names does not know
# what it is supposed to install.
- uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
```

A version tag can be a major line or a full version: `v7`, `v4.39.3`,
`v2.0.0-rc1`.

### The one exception

`dtolnay/rust-toolchain` is named by a branch, because the branch is how
it takes a toolchain:

```yaml
- uses: dtolnay/rust-toolchain@stable
```

The `stable` branch installs the current stable Rust. The action's `v1`
tag, by contrast, requires the toolchain as an explicit input, so
`@v1` on its own does not work.

Every other action must name a version.

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
