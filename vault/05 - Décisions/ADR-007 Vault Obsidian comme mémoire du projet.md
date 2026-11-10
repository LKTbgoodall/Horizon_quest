---
tags: [adr]
statut: accepté
date: 2026-11-10
---
# ADR-007 : Vault Obsidian comme mémoire du projet

## Contexte
Le projet va s'étaler sur beaucoup de sessions avec Claude. Relire toute une conversation à chaque fois est impossible.

## Décision
Toute la connaissance du projet vit dans `vault/` (Obsidian, Markdown + `[[liens]]`) : référence Horizon et FTB, conception, ADR, roadmap et journal. Le `CLAUDE.md` à la racine oblige Claude à lire `00 - Accueil` au début de chaque session et à mettre à jour le journal, la roadmap et l'accueil à la fin.

## Conséquences
- + Reprise de contexte rapide pour Bastien comme pour Claude.
- − Il faut de la discipline pour maintenir les notes (c'est le rôle de la routine de fin de session).

[[Index des décisions]]
