# Releasing mom

This document is the runbook for cutting a numbered release. Anyone with
push access to `main` can drive a release; the release notes and changelog
entries must be reviewed by at least one other maintainer before the
tag is pushed.

## Release cadence

- **Pre-1.0:** one minor release per shipped phase.
- **1.0 and beyond:** quarterly minor releases, patch releases on demand.

## Pre-flight checklist

```sh
# 1. Working tree is clean.
git status --porcelain

# 2. Main is green.
cargo fmt --check
cargo clippy --all-targets -- -D warnings   # optional today, mandatory at 1.0
cargo test                                  # all suites green

# 3. Stage-1 selfhost is still bit-identical.
cargo test --test selfhost

# 4. Stdlib oracles match.
cargo test --test stdlib

# 5. CHANGELOG entry is written and dated (today).
grep -n "## \[$NEW_VERSION\]" CHANGELOG.md
```

## Bump version

1. Edit `Cargo.toml`:
   ```toml
   [package]
   version = "<new>"
   ```
2. Run `cargo build` so `Cargo.lock` updates.
3. Run `mom version` to confirm the binary reports the new string.
4. Move the **Unreleased** section in `CHANGELOG.md` under a dated
   `## [<new>] — <one-line-summary>` heading.

## Tag + push

```sh
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "release: $NEW_VERSION"
git tag -s "v$NEW_VERSION" -m "mom $NEW_VERSION"
git push origin main --follow-tags
```

The `release.yml` workflow (see `.github/workflows/release.yml` once
it lands) picks up the tag, builds the tier-1 binaries, signs them,
and uploads them as a GitHub Release.

## Post-release

- Announce in the discussions board and the project Slack/Discord.
- Open the next `Unreleased` section at the top of `CHANGELOG.md`.
- Bump `Cargo.toml` to `<new>-dev` to avoid accidental re-use.
- File an issue titled `release: retrospective $NEW_VERSION`
  capturing anything that surprised the release driver.

## Hot-fix releases

Patch releases follow the same flow on a `release/<minor>` branch.
Cherry-pick the fix, run the pre-flight, and ship.
