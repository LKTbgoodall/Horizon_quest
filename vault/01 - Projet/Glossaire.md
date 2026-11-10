---
tags: [projet, glossaire]
---
# Glossaire

| Terme | Sens |
|---|---|
| **Horizon** | Minecraft Java 26.3 réécrit en Rust (client + `horizon-server`). Ce n'est pas un modloader. → [[Horizon - Vue d'ensemble]] |
| **Mod natif** | Code Rust **compilé dans le jeu** par le launcher, livré en `.hnmod`, non sandboxé. → [[Mods natifs - structure et manifeste]] |
| **Mod de contenu** | Mod data-only (blocs, items, recettes, textures) livré en `.hmod`. |
| **Hook point** | Point d'extension stable exposé par Horizon (`native_tick`, `native_hud`…). Il survit aux Nightlies. |
| **Injection** | Code inséré dans une fonction du jeu (`HEAD`, `RETURN`, `BEFORE_CALL`, `MODIFY_ARG`, `OVERWRITE`), comme les Mixins. Il oblige à épingler `horizon_revision`. |
| **Typed API** | `horizon_client::mod_api` : trait `Hooks`, `Host`, `Flow`. Plus propre que les hooks bruts. |
| **`horizon-mixin.toml`** | Le manifeste d'un mod natif. |
| **`compatibility.json`** | Déclare la version de jeu, la version d'API des hooks et les commits testés. |
| **`native.py`** | L'outil du dépôt `horizon-mods` pour `check`, `build` et `package` un mod natif. |
| **Nightly** | Build quotidien d'Horizon. L'API peut bouger entre deux Nightlies. |
| **Quête** | Nœud du livre. Elle contient des tâches et des récompenses. |
| **Tâche** | La condition à remplir (obtenir un item, tuer un mob…). |
| **Récompense** | Ce qu'on reçoit en réclamant (items, XP, commande…). |
| **Chapitre** | Une page du livre, avec son canvas de quêtes. |
| **Dépendance** | Une quête A doit être terminée (ou commencée) avant B. |
| **Claim** | Réclamer une récompense. |
| **Team / équipe** | Groupe de joueurs qui partage la même progression. |
| **ADR** | *Architecture Decision Record* : la fiche d'une décision technique. → [[Index des décisions]] |
