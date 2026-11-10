---
tags: [adr]
statut: proposé
date: 2026-11-10
---
# ADR-005 : Progression en JSON dans le dossier du monde

## Contexte
- `AttachmentType` sur `Holder::Player` ne couvre que les joueurs **en ligne**, alors qu'une équipe a des membres hors ligne. Au respawn, il faut `copy_on_death` et un codec NBT.
- smp stocke tout en JSON dans `native_world_dir()/horizon-smp/`, ce qui est simple, lisible, sauvegardé avec le monde et testable.

## Décision proposée
La progression et les équipes vont en **JSON** dans `<monde>/horizon-quest/`, avec un fichier par équipe et une écriture atomique en arrière-plan (copie du pattern `smp_store`). Les attachments éventuels ne serviraient qu'à **synchroniser** vers le client.

## Conséquences
- + Débogage facile, édition manuelle possible, migrations simples.
- − Il faut gérer nous-mêmes la cohérence (flush à la fermeture, écriture atomique).

[[Index des décisions]] · [[Persistance et synchronisation]]
