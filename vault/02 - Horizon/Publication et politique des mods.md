---
tags: [horizon, publication]
source: playhorizon.dev/docs/publish-a-mod, horizon-mods/MOD-POLICY.md, README.md, mods/*.toml, native/tools/native.py
verifie_le: 2026-11-10
---
# Publication et politique des mods

Retour : [[Horizon - Vue d'ensemble]]

## Processus
1. Le code source doit être dans un **dépôt public** sous une licence libre (nom SPDX : MIT, Apache-2.0, GPL…). Pour nous : https://github.com/LKTbgoodall/Horizon_quest. Licence à choisir, voir [[Questions ouvertes]].
2. `python3 native/tools/native.py package <dossier>` produit `native/target/<id>-<version>.hnmod`, puis affiche la taille et le sha256.
3. Formulaire sur https://playhorizon.dev/mods/submit (connexion GitHub) : fichier, id, version de jeu (26.3), description Markdown, auteurs, licence, URL source, icône PNG ≤ 256 KiB.
4. Une **PR** est ouverte automatiquement sur `horizon-rust/horizon-mods`, avec `mods/<id>.toml` et `icons/<id>.png`.
5. **Relecture humaine** : un mainteneur lit **chaque ligne** du `.hnmod`, et **chaque mise à jour** d'un mod natif est relue.
6. Chaque version publiée exige un nouveau `version`. « A published file never changes. »

## Contenu d'un `.hnmod`
Un zip déterministe qui contient `horizon-native.json` (`{schema:1, id, version, game, horizon, mixins, hooks}`), les `.rs`, `.toml` et `data/` sous `mixins/<id>/`, et la `LICENSE`.

## Entrée de registre `mods/<id>.toml`
```toml
id = "horizon_quest"        # a-z, 0-9, _
kind = "native-mod"
name = "Horizon Quest"
summary = "…"
description = """markdown"""
authors = ["…"]
owners = ["<github login>"]
owner_ids = [123]
license = "MIT"
source = "https://github.com/LKTbgoodall/Horizon_quest"
icon = "icons/horizon_quest.png"
categories = ["…"]
tags = ["quests", "progression"]
# key = "<minisign pubkey>"   (optionnel ; une fois mis, toutes les versions doivent être signées)

[[versions]]
version = "0.1.0"
published = "YYYY-MM-DD"
game = "26.3"
file = "horizon_quest-0.1.0.hnmod"
sha256 = "…"
changelog = "…"
```

## Politique (MOD-POLICY.md), résumé
En soumettant, on confirme :
1. avoir le droit de publier sous la licence indiquée ;
2. pour un **port**, respecter la licence d'origine ou avoir une permission écrite des auteurs. ⚠️ FTB Quests est *All Rights Reserved* : on fait une **réimplémentation inspirée**, pas un port. Aucun code, aucun asset et aucun texte de FTB ne doit être repris ;
3. ne fournir **aucun code, asset ou donnée Mojang**. Les tables de données du jeu (recettes, biomes) se lisent au runtime depuis les fichiers du joueur ;
4. qu'il n'y a rien de nuisible (malware, vol de données, accès réseau caché) ;
5. que le lien `source` correspond exactement au code compilé.

Pas de clients de triche. Les retraits passent par `revoked.toml`.

## Sécurité côté joueur
Les mods natifs ne sont **pas sandboxés** : le joueur accepte un avertissement rouge et le launcher compile le jeu avec `--trust-native-code`. Un mod natif est **compilé par instance**.

## Signature (optionnelle)
```bash
minisign -G -p horizon-mods.pub -s horizon-mods.key
minisign -S -s horizon-mods.key -m native/target/<id>-<version>.hnmod
```

Voir aussi : [[Build, test et outils]]
