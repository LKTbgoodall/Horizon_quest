---
tags: [adr]
statut: proposé
date: 2026-11-10
---
# ADR-003 : Hook points plutôt qu'injections brutes

## Contexte
- Les **hook points** du catalogue (`native-hooks.json`) ne sont qu'ajoutés et leurs contrats sont gelés. Un mod qui ne les utilise que compile sur toute version d'Horizon ≥ `hooks`.
- Une injection sur une `target` brute peut casser à chaque commit, et oblige à épingler `horizon_revision`.

## Décision proposée
N'utiliser **que des hook points** (avec `hook` + `hook_version`, ou les cibles `Game::native_*` comme smp). On n'injecte une fonction brute qu'en dernier recours, avec une ADR dédiée.
Les hooks **⏳ pending** (ex. `player_inventory_changed`) ne sont pas utilisés tant qu'ils n'ont pas de numéro. On passe par des solutions de repli (sondage).

## Conséquences
- + Les mises à jour d'Horizon sont indolores.
- − Certaines fonctionnalités attendent des hooks (biome, structure, toasts) : voir [[Questions ouvertes]].

[[Index des décisions]]
