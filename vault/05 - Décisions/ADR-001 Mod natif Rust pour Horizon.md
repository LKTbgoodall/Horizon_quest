---
tags: [adr]
statut: accepté
date: 2026-11-10
---
# ADR-001 : Mod natif Rust pour Horizon

## Contexte
On veut un FTB Quests sur Horizon. Or Horizon est une réécriture de Minecraft en Rust : les jars Java et les Mixins ne fonctionnent pas. Les mods de contenu (`.hmod`) sont data-only.

## Options
1. Porter FTB Quests → impossible : pas de JVM, et la licence est All Rights Reserved.
2. Mod de contenu seul (advancements, datapacks) → beaucoup trop limité (pas de GUI, pas de progression d'équipe).
3. **Mod natif Rust** (`.hnmod`, puis `.hmod` combiné pour les assets).

## Décision
Option 3 : un mod natif en Rust, réimplémenté **from scratch** et inspiré des concepts de FTB Quests, sans aucun code ni asset FTB.

## Conséquences
- + Tout est à écrire, mais on maîtrise le design.
- − L'API native est **experimental** : il faudra suivre les Nightlies (voir [[ADR-003 Hook points plutôt qu'injections brutes]]).
- − Le code est relu ligne à ligne par les mainteneurs à chaque version publiée.

[[Index des décisions]]
