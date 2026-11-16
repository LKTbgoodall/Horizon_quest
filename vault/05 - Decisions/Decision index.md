---
tags: [adr, index]
---
# Decision index (ADR)

Every important technical decision has its own note (template: `99 - Templates/Template - ADR`). An ADR is never deleted: if it changes, we mark it **superseded** and create the new one.

| # | Decision | Status | Date |
|---|---|---|---|
| 001 | [[ADR-001 Native Rust mod for Horizon]] | ✅ accepted (imposed) | 2026-11-10 |
| 002 | [[ADR-002 Authoritative server and dual UI]] | ✅ accepted, UI part superseded by 008 | 2026-11-10 |
| 003 | [[ADR-003 Hook points over raw injections]] | ✅ accepted, superseded by 018 at the port | 2026-11-10 |
| 004 | [[ADR-004 Mixin bundle style like smp]] | ✅ accepted, superseded by 018 at the port | 2026-11-10 |
| 005 | [[ADR-005 Progress as JSON in the world folder]] | ✅ accepted | 2026-11-10 |
| 006 | [[ADR-006 Quest definitions as JSON per chapter]] | ✅ accepted | 2026-11-10 |
| 007 | [[ADR-007 Obsidian vault as project memory]] | ✅ accepted | 2026-11-10 |
| 008 | [[ADR-008 Graphical client book first]] | ✅ accepted | 2026-11-10 |
| 009 | [[ADR-009 MIT license and mod id horizon_quests]] | ✅ accepted | 2026-11-10 |
| 010 | [[ADR-010 Item detection by polling plus triggers]] | ✅ accepted | 2026-11-10 |
| 011 | [[ADR-011 Repeatable quests start over]] | ✅ accepted (coded in v0.0.6) | 2026-11-10 |
| 012 | [[ADR-012 Parties share one progress]] | ✅ accepted (coded in v0.2.0) | 2026-11-10 |
| 013 | [[ADR-013 Version numbers]] | ✅ accepted | 2026-11-10 |
| 014 | [[ADR-014 Publish one hmod with content and code]] | ✅ accepted | 2026-11-11 |
| 015 | [[ADR-015 Embed the content pack in the native mod]] | ❌ dropped: the mod API format ships content and code in one `.hmod` (018) | 2026-11-11 |
| 016 | [[ADR-016 Edit mode in the book]] | ✅ accepted (coded in v0.11.0) | 2026-11-12 |
| 017 | [[ADR-017 Edit in place]] | ✅ accepted (coded in v0.15.0) | 2026-11-12 |
| 018 | [[ADR-018 Port to the mod API]] | ✅ accepted, ported on branch `mod-api-port` (compiles against `main`); merged when the Nightly is out, 1.0.0 on it | 2026-11-16 |
| 019 | [[ADR-019 Big main chapters, few side paths]] | ✅ accepted (done in v0.21.1) | 2026-11-16 |

> 🟡 = proposed, waiting for Bastien. Set `status: accepted` in the frontmatter and update this table.
