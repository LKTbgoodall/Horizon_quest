---
tags: [conception, tasks, hooks]
statut: brouillon
date: 2026-11-10
---
# Tâches : correspondance avec les hooks Horizon

Sources : [[Types de tâches FTB]] × [[Hooks et API Horizon]]

| Tâche | Stratégie de détection | Hook / helper | Statut API | Phase |
|---|---|---|---|---|
| **Checkmark** | clic dans le livre (menu ou écran) ou `/quests check <id>` | `native_menu_clicked`, `native_perform_command`, net message | ✅ | MVP |
| **Item** (avoir n items) | **sondage** de l'inventaire toutes les `detection_delay` ticks, puis événement quand le hook arrive | `native_inventory(uuid, Player)` (132) ; plus tard `native_player_inventory_changed` (⏳) ; déclenchement rapide par `native_item_picked_up` (40) | ✅ / ⏳ | MVP |
| Item **consume** | prise au clic « Soumettre » dans le livre | `native_run_command("clear …")` (depuis un tick ou un menu), en mesurant avant et après (pattern smp `MenuBag`) | ✅ | MVP |
| **Kill** | événement | `native_mob_killed` (40), sinon `native_living_death(..., attacker, ...)` (392) | ✅ | MVP |
| **Craft** | événement | `native_item_crafted` (161) ; four : `native_furnace_extract` (228) | ✅ | V1 |
| **XP Levels** | événement + lecture | `native_player_level_change` (347) | ✅ | V1 |
| **Dimension** | événement + sondage de secours | `native_player_changed_dimension` (154) ⚠ à tester en solo ; `native_server_players().dimension` | ✅ | V1 |
| **Location** | sondage de la position | `native_server_players()` ou `native_player_tick` (31) | ✅ | V1 |
| **Advancement** | événement | `native_player_advancement_done` (249) | ✅ | V1 |
| Casser / poser un bloc | événement | `native_destroy_block` / `native_place_block` (14). ⚠ Il faut récupérer l'id du bloc (pas lu par smp) : **à vérifier** | ✅ ? | V2 |
| **Stat** | événement | `native_player_stat_award` (321) | ✅ | V2 |
| **Biome** | sondage | `native_chunk_snapshot().noise_biome` + `native_biome_name` | ⏳ | V2 |
| **Structure** | — | ❌ non documenté | ❌ | plus tard |
| **Observation** | sondage du regard (yaw/pitch + raycast ?) | pas de raycast documenté | ❌ ? | plus tard |
| Utiliser, manger un item | événement | `native_living_entity_use_item_finish` (90), `native_use_item` (13) | ✅ | V2 |
| Pêcher, apprivoiser, élever, commercer | événement | `native_player_fish` (160), `native_entity_tame` (220), `native_entity_breed` (213), `native_trade_with_villager` (310) | ✅ | V2 |
| **Custom** | API publique pour d'autres mods (`[dependencies]`) ou commande `/quests progress <player> <task> <n>` | — | ✅ | V2 |

## Règles du moteur
- Une tâche ne progresse que si la quête est **startable** (mode Linear) ou visible (mode Flexible). Voir [[Modèle de données FTB]].
- Les compteurs sont **par équipe**. Un événement d'un membre fait progresser toute l'équipe.
- Tâches `Item` sans consume : progression = `min(count, nombre max trouvé chez un membre en ligne)`. Elle **ne régresse pas** une fois la tâche complétée.
- Les tâches séquentielles (V1) ne progressent qu'une à la fois.
