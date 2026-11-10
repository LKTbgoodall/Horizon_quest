---
tags: [design, storage, network]
status: draft
date: 2026-11-10
---
# Persistence and sync

API: [[Horizon hooks and API]] §1 and §4 · Decision: [[ADR-005 Progress as JSON in the world folder]]

## Where the data lives
| Data | Location | Why |
|---|---|---|
| **Quest definitions** (modpack) | `config/horizon_quests/quests/` (via `native_config_dir()`) | shared between worlds, versionable, hand-editable |
| Definitions shipped as a datapack (V1) | `data/<ns>/horizon_quests/*.json` via `native_add_server_reload_listeners` + `NativeReloadListener::json` | reloaded by `/reload`, distributable |
| Example quests | embedded in the bundle's `data/` (`native_data!`) | demo without configuration |
| **Progress and teams** | `<world>/horizon-quests/` (via `native_world_dir()`) | follows the world's backups (smp pattern) |
| Mod config | `config/horizon_quests-server.toml` (a world may override it in `<world>/serverconfig/`) via `native_register_configs`, since v0.0.5; `-client.toml` later | standard |
| Client UI preferences | `native_setting` / `native_set_setting` | per local player |

## Progress files
```
<world>/horizon-quests/
├── teams.json                 ← parties: name, owner, members (id → name), invitations (v0.2.0)
├── progress/<team-id>.json    ← one file per team: <player uuid>.json alone, party-<16 hex>.json for a party
├── audit.log                  ← JSON lines: completions, claims (debug, anti-dupe)
└── state.json                 ← schema version, timestamp
```
- **Atomic** write (temp + `sync_all` + rename), on a background writer thread. Only what changed (`dirty`) is written.
- Read: missing file → default. Corrupted file → **error, the feature disables itself, and we never overwrite** (smp rule).
- Migrations: a `version` field in each file, and `hq_store::migrate()`.

## Horizon client sync: as coded in `hq_net.rs` (2026-11-10, MVP)
- Channel `horizon_quests:book`, `PROTOCOL` 1. Messages are JSON (`serde_json`), not compressed.
- Book → server (`ToServer`): `Hello { protocol, version }`, `Check { task }`, `Submit { task }`, `Claim { reward }`, `ClaimAll`.
- Server → book (`ToClient`): `Welcome` or `Incompatible { protocol, version }`, `Book { file }` (the whole pack, on joining and after a reload), `Progress { player, progress }` (whole, after every change), `Notice { text }` (a refusal).
- A message is cut into **frames** of at most 30,000 bytes: `transfer u32 | index u16 | count u16 | bytes`, little endian. `Sender` lets **16 frames a tick** out (Horizon allows 32) and takes back those that could not leave. `Receiver` puts messages back together; a frame out of order or of a wrong size drops the message under way. The server accepts 1 part per message from a book, the book up to 512 from the server.
- Not done yet: progress deltas, compression (`flate2`), `Track`, the editor's messages.

## Horizon client sync: first design (kept for reference)
- Channel `horizon_quests:sync` (mod messages, API 40). Constraints: ≤ 30,000 bytes per message and ≤ 32 messages per tick.
- Protocol (versioned):
  1. client → `Hello { proto_version, mod_version }` on connection. The server replies `Welcome` or `Incompatible` (Horizon has **no** native handshake);
  2. server → `Definitions { chunk i/n, payload }`, compressed if needed (`flate2` is in the lock);
  3. server → `Progress { full | delta }`;
  4. client → `Claim { reward }`, `Submit { task }`, `Check { task }`, `Track { quest }`, then in V3 the `Edit…` messages.
- A client without the mod, or a vanilla one, receives nothing and uses the chest book.
- Alternative: `AttachmentType { sync: true }` on `Holder::Player` for the progress (simpler, 1 MiB max, Horizon clients only). **To evaluate.**

## Questions
- Do mod messages work in singleplayer, between the windowed game and the integrated server? → [[Open questions]]
