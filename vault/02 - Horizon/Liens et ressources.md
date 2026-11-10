---
tags: [horizon, liens, reference]
verifie_le: 2026-11-10
---
# Liens et ressources

## Projet
- Dépôt : https://github.com/LKTbgoodall/Horizon_quest

## Horizon : site
- Accueil : https://playhorizon.dev
- Catalogue des mods : https://playhorizon.dev/mods
- Soumettre un mod : https://playhorizon.dev/mods/submit
- Politique des mods : https://playhorizon.dev/mods/policy
- Docs : https://playhorizon.dev/docs
  - Créer un mod : https://playhorizon.dev/docs/make-a-mod
  - Manifeste : https://playhorizon.dev/docs/mod-manifest
  - Code de mod : https://playhorizon.dev/docs/mod-code
  - Tester : https://playhorizon.dev/docs/test-a-mod
  - Publier : https://playhorizon.dev/docs/publish-a-mod
  - APIs : https://playhorizon.dev/docs/apis
  - API serveur : https://playhorizon.dev/docs/api/server
  - Mods de contenu (non lu) : https://playhorizon.dev/docs/content-mods

## Horizon : GitHub
- Moteur : https://github.com/horizon-rust/horizon (MIT)
  - **Catalogue des hooks** : `docs/native-mods.md`, et `native-hooks.json` à la racine
  - Code des hooks : `crates/horizon-client/src/game/native_hooks.rs`
  - Issues : https://github.com/horizon-rust/horizon/issues
- Registre des mods et outils : https://github.com/horizon-rust/horizon-mods
  - Exemple serveur : `native/smp` → [[Exemple - mod smp (patterns à reprendre)]]
  - Petits exemples : `native/fps-overlay`, `native/coords-hud`, `native/zoom`, `native/keystrokes`, `native/minimap`, `native/recipes`
  - Outil : `native/tools/native.py`
- Mixins : https://github.com/horizon-rust/horizon-mixins
- Template : https://github.com/horizon-rust/horizon-native-mod-template
- Discord : https://discord.gg/HcTeMZxe5E

## Copies locales (scratchpad, temporaires)
Les clones `--depth 1` de `horizon` (docs seulement) et de `horizon-mods` ont été faits le 2026-11-10 dans le scratchpad de la session Claude. **Ils ne sont pas persistants** : il faudra les recloner dans un dossier `ext/` du projet si besoin (voir [[Roadmap]], phase 0).

## FTB Quests (modèle)
- CurseForge : https://www.curseforge.com/minecraft/mc-mods/ftb-quests-forge
- Docs : https://docs.feed-the-beast.com/mod-docs/mods/suite/Quests/
- Source : https://github.com/FTBTeam/FTB-Quests (All Rights Reserved : **lecture pour comprendre, jamais de copie**)
- FTB Teams : https://docs.feed-the-beast.com/mod-docs/mods/suite/Teams/
