---
tags: [adr]
status: accepted
date: 2026-11-10
---
# ADR-013: Version numbers

## Context
Before 0.1.0 every step took the next 0.0.x. After the first private release, 0.1.1 and 0.1.2 bumped the last number, then parties, settings and data pack quests became 0.2.0, 0.3.0 and 0.4.0 without the rule being written down. Bastien asked why.

## Options considered
1. **Semver for 0.x**: + the usual reading, a minor bump says "new feature" / − untested versions look like releases.
2. **One number per step tested in game**: + every number was played / − several steps waiting for a test share one number.

## Decision
Option 1, accepted by Bastien (2026-11-10):
- **x.y.Z** (last number): a small addition or a fix (0.1.1 the book item, 0.1.2 its texture).
- **x.Y.0** (middle number): a new feature, one per phase 3 step (0.2.0 parties, 0.3.0 settings, 0.4.0 data packs).
- **X.0.0**: 1.0.0 is the first public release on playhorizon.dev (end of phase 3).
- A version is bumped when it is coded, whether or not it was tested in game yet; the journal says what is still to test.

## Consequences
- Every number goes in `Cargo.toml`, `horizon-mixin.toml`, `content/horizon-mod.toml` and `hq_meta::VERSION` together.
- A git tag marks only versions tested in game (`v0.1.0` so far).

## Links
- [[Decision index]]
