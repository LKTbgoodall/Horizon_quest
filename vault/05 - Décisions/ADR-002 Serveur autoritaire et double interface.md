---
tags: [adr]
statut: proposé
date: 2026-11-10
---
# ADR-002 : Serveur autoritaire et double interface

## Contexte
- La progression doit être fiable (anti-triche) et partagée en équipe, donc gérée côté serveur.
- Les écrans de mod (`native_mod_screen_*`) et les mod messages n'existent **que pour les clients Horizon** équipés du mod.
- Les **menus coffre** (`native_open_menu`) fonctionnent avec **n'importe quel client** et sont déjà éprouvés dans `smp`.

## Options
1. Client seulement (progression locale) → triche facile, pas de multi.
2. Serveur et écran client graphique uniquement → gros chantier GUI avant d'avoir quoi que ce soit de jouable.
3. **Serveur autoritaire + livre coffre (MVP), puis écran graphique client (V2)**.

## Décision proposée
Option 3. Toute la logique et les données sont côté serveur. Le MVP se joue avec des menus coffre, la sidebar et le chat. Le livre graphique arrive ensuite et lit les mêmes données via `hq_net`.

## Conséquences
- + Un MVP jouable vite, compatible avec tous les clients.
- + La logique est testable sans GUI.
- − Le livre coffre n'est pas aussi joli que FTB (pas de canvas).
- − Deux interfaces à maintenir (mais une seule logique, `hq_logic`).

Voir [[Architecture]] · [[Interface - Livre de quêtes]] · [[Index des décisions]]
