---
tags: [conception, ui]
statut: brouillon
date: 2026-11-10
---
# Interface : le livre de quêtes

Référence : [[Éditeur et interface FTB]] · API : [[Hooks et API Horizon]] §5 · Décision : [[ADR-002 Serveur autoritaire et double interface]]

## Deux interfaces, une seule logique
| | **A. Livre « coffre »** (serveur) | **B. Livre graphique** (client Horizon) |
|---|---|---|
| Client requis | n'importe lequel (vanilla inclus) | Horizon avec le mod installé |
| Techno | `native_open_menu` (6 lignes = 54 slots) | `native_mod_screen_open` + `native_screen_render_post` |
| Rendu | des items avec un nom et un lore | un canvas libre : formes, lignes, icônes, texte |
| Phase | **MVP** | V2 |
| Édition | non (commandes et fichiers) | oui (V3) |

## A. Livre coffre (MVP)
Ce sont des pages pures dans `hq_book_menu` (`layout()` / `click()`, testables comme `smp_market_menu`).

```
Page Chapitres               Page Chapitre (liste)          Page Quête
┌─────────────────────────┐  ┌─────────────────────────┐   ┌─────────────────────────┐
│ [📘ch1][📗ch2][📕ch3]…  │  │ [✔q1][▶q2][🔒q3][?q4]…  │   │ [icône] Titre           │
│                         │  │  (vert = fini, jaune =  │   │ Tâches : [⛏ 3/5][✓]     │
│                         │  │   en cours, gris = lock)│   │ Récompenses : [💎][XP]  │
│ ─────── barre nav ───── │  │ ─────── barre nav ───── │   │ [Soumettre][Réclamer]   │
│ [◀]   [?]   [▶]    [✖] │  │ [↩][◀]  [Tout récl.][▶] │   │ [↩] [Suivre 📌]   [✖]  │
└─────────────────────────┘  └─────────────────────────┘   └─────────────────────────┘
```
- La ligne du bas (slots 45..53) sert de navigation : `BACK=45`, `PREV=47`, `NEXT=51`, `CLOSE=53` (convention smp). On affiche 45 éléments par page.
- Le statut se voit par l'icône (verre coloré ou item de la quête avec un *glint* ?) et par le lore (« ✔ Terminée », « 🔒 Requiert : … »).
- Ouverture : commande `/quests` (ou `/q`), et éventuellement un item « livre » via `native_use_item`.
- **Quête suivie** : affichée dans la sidebar (`native_sidebar`) avec la progression des tâches, mise à jour une fois par seconde et seulement quand des lignes changent (pattern `smp_board`).
- Notifications : message de chat cliquable (« [Voir la quête] » → `RunCommand /quests open <id>`), action bar, son `ui.toast.challenge_complete`, titre via la commande `title`.

## B. Livre graphique (V2)
- **Canvas** : transformation `screen = (world - pan) * zoom + origin`. Pan avec `screen_mouse_dragged_pre`, zoom avec `screen_mouse_scrolled_pre`, clic avec `screen_mouse_button_pressed_pre`.
- Rendu :
  - formes : `native_sprite` (textures de formes dans `assets/horizon_quest/textures/gui/shapes/`) ou `native_quad` ;
  - icônes : `native_item_stack` ;
  - lignes de dépendance : `native_quad` (épaisseur via un quad orienté) ;
  - texte : `native_text`.
- ⚠ **Pas de clip/scissor 2D documenté** : il faudra rogner nous-mêmes (ne pas dessiner hors de la zone) ou limiter le canvas au plein écran.
- Sidebar des chapitres, panneau de détail de quête et boutons : **widgets maison** (pas de bibliothèque disponible).
- Clic sur une tâche item : `native_show_recipes(stack, lookup)` ouvre JEI/REI si présent.
- Données : envoyées par le serveur via `hq_net` (définitions à la connexion et au reload, puis deltas de progression). Il faut **découper** les messages (limite de 30 000 octets).
- Toast client : `native_add_toast` (⏳).
- Ouverture : une touche (`native_mod_key`), la commande client `native_command`, ou le livre en main (`native_claim_items`).

## C. Éditeur in-game (V3)
- Mode édition réservé aux ops (niveau 2+, plus tard avec un nœud de permission).
- Actions : créer, déplacer (drag), lier les dépendances (shift-clic), éditer les champs (saisie via `screen_char_typed_pre`).
- Chaque modification devient un net message → le serveur valide, sauvegarde le fichier du chapitre et rediffuse.
- **Alternative moins chère** : éditer les JSON à la main, plus `/quests reload`, plus un éditeur web externe (?).
