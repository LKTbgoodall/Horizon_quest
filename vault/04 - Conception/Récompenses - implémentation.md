---
tags: [conception, rewards]
statut: brouillon
date: 2026-11-10
---
# Récompenses : implémentation

Sources : [[Types de récompenses FTB]] × [[Hooks et API Horizon]]

| Récompense | Implémentation | Statut | Phase |
|---|---|---|---|
| **Item** | `native_give_stack(uuid, stack)` → s'il reste quelque chose : `native_run_command("summon item …")` à la position du joueur, **ou** stockage dans une « boîte aux lettres » de récompenses (à décider) | ✅ | MVP |
| **XP / niveaux** | `native_give_experience(uuid, pts)` / `native_give_levels(uuid, lvls)`. S'ils renvoient `false`, on réessaie au tick suivant | ✅ | MVP |
| **Commande** | `native_run_command(line)` au tick (niveau 4, console), avec `{player}` et `{uuid}` remplacés. Plus tard `native_run_command_as` (⏳) | ✅ | V1 |
| **Toast** | client Horizon : net message → `native_add_toast` (⏳). Client vanilla : advancement injecté (`native_data_pack_files`) + `native_trigger_criterion` (69), ou un titre ou une action bar | ⏳ / ✅ | V1 |
| **Choice** | ouverture d'un menu coffre listant la table, le clic choisit | ✅ | V1 |
| **Random / Loot / AllTable** | **nos propres tables pondérées** (`hq_rewards`, RNG injecté pour les tests) → Item | ✅ | V1/V2 |
| Currency | pont optionnel vers l'économie `smp` ? (`[dependencies]` optionnelle) | ❓ | V2 |
| Advancement | `native_set_player_advancements` (⏳) ou commande `advancement grant` | ⏳ / ✅ | V2 |

## Flux de réclamation
1. La quête est complétée : `hq_progress` émet `Effect::QuestCompleted` (notification, son).
2. Pour chaque récompense :
   - `auto_claim = Enabled | NoToast | Invisible` : réclamation immédiate ;
   - sinon : l'état devient `CanClaim`, et le joueur clique dans le livre ou utilise « Tout réclamer ».
3. Réclamer :
   - `team: true` : une seule fois pour l'équipe ;
   - `team: false` : une fois **par joueur** (chaque membre réclame sa part).
4. Les effets sont appliqués **au tick, hors lock**. L'état `claimed` est enregistré **avant** de donner (pas de duplication si crash), et un journal d'audit en JSON lines est tenu comme `smp/audit.log`.

## Sécurité
- Ne jamais faire confiance au client : une demande de claim via net message est **revalidée** côté serveur (quête complétée ? récompense pas encore réclamée ? joueur dans l'équipe ?).
- Anti-spam des clics (cooldown court par joueur).
