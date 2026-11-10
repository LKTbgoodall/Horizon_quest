---
tags: [adr]
status: accepted
date: 2026-11-10
---
# ADR-007: Obsidian vault as project memory

## Context
The project will span many sessions with Claude. Rereading an entire conversation each time is impossible.

## Decision
All project knowledge lives in `vault/` (Obsidian, Markdown + `[[links]]`): Horizon and FTB reference, design, ADRs, roadmap and journal. The `CLAUDE.md` at the root forces Claude to read `00 - Home` at the start of every session and to update the journal, roadmap and home at the end.

## Consequences
- + Quick context pickup for Bastien and Claude alike.
- − Discipline is needed to maintain the notes (that is the role of the end-of-session routine).

[[Decision index]]
