---
tags: [adr]
statut: proposé
date: 2026-11-10
---
# ADR-004 : Style bundle mixin comme smp (et non trait mod pur)

## Contexte
Il y a deux styles :
1. **trait mod** (`[api]`, `Hooks`, `Host`), plus propre ;
2. **bundle mixin** (`[[module]]` + `[[injection]]` + helpers), le style de `smp`.

Le trait n'a **pas encore tous les hooks** : il lui manque `player_command`, les entrées clavier et molette des écrans, `player_respawned`, `server_data_loaded`… Il lui manque aussi des helpers (`sidebar`, `player`).

## Décision proposée
Bundle mixin à la smp pour le MVP, parce que c'est l'exemple serveur le plus complet et qu'il est maintenu par l'équipe Horizon. Les helpers peuvent appeler la typed API via `&mut Game as &mut dyn Host` quand c'est plus simple. On réévaluera le trait pur quand il sera complet.

## Conséquences
- + On peut copier les patterns smp tels quels.
- − Plus de fichiers `.toml` et de helpers à maintenir.

[[Index des décisions]] · [[Mods natifs - structure et manifeste]]
