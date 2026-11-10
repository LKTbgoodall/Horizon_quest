---
tags: [adr]
status: accepted
date: 2026-11-10
---
# ADR-005: Progress as JSON in the world folder

## Context
- `AttachmentType` on `Holder::Player` only covers **online** players, while a team has offline members. On respawn, `copy_on_death` and an NBT codec are needed.
- smp stores everything as JSON in `native_world_dir()/horizon-smp/`, which is simple, readable, saved with the world and testable.

## Decision
Progress and teams go in **JSON** in `<world>/horizon-quests/`, with one file per team and an atomic background write (copying the `smp_store` pattern). Any attachments would only be used to **sync** to the client.

## Consequences
- + Easy debugging, manual editing possible, simple migrations.
- − We have to handle consistency ourselves (flush on close, atomic write).

[[Decision index]] · [[Persistence and sync]]
