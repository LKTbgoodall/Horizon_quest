---
tags: [adr]
status: accepted
date: 2026-11-10
---
# ADR-010: Item detection by polling plus triggers

## Context
Item tasks ("have 8 logs") must notice items however they reach the player. Bastien suggested watching every pickup. At Horizon `fdc87ae`:
- `native_item_picked_up(player, item, count, canceled)` (API 40) only sees items taken from the ground. It misses chests, crafting results, furnace outputs, trades, `/give`, rewards, and items held before the quest opened. It also fires **before** the item enters the inventory.
- `native_player_inventory_changed` (the ideal hook, "what quest mods need instead of polling") is still ⏳ pending.
- `native_inventory(uuid, Player)` (API 132) reads any online player's 41 slots from the server tick, in solo too (the window joins the integrated server as a player).

## Options
1. Pickup events only: fast, but misses most ways of getting items, and counts pickups instead of what the player holds.
2. Polling only: complete, but up to `detection_delay` ticks late.
3. **Polling plus triggers.**

## Decision
Option 3, in `hq_session` + `hq_server_glue`:
- every player's inventory is read every `detection_delay` ticks (20 by default), only when they have an open item task;
- a pickup (`native_item_picked_up`), a craft (`native_item_crafted`, 161) or a furnace take (`native_furnace_extract`, 228) **pokes** the player, so the next tick reads their inventory at once;
- tags (`#minecraft:logs`) are matched with `native_server_item_in_tag` (63).
When `native_player_inventory_changed` gets a number, it replaces the polling and the pokes.

## Consequences
- Nothing is missed; the common cases react within one tick.
- Cost: one 41-slot read per player per second, only with an open item task.
- Item tasks count what the player **holds** (like FTB), not what they picked up.
