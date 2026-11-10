---
tags: [accueil, moc]
mise_a_jour: 2026-11-10
---
# 🧭 Horizon Quest : accueil

> **Un mod de quêtes façon FTB Quests pour Horizon** (Minecraft Java 26.3 réécrit en Rust).
> Ce vault est le « deuxième cerveau » du projet. Tout ce qui compte est ici, pour ne pas avoir à relire les conversations.

## 📍 Où on en est
| | |
|---|---|
| **Phase** | 0 : mise en place → [[Roadmap]] |
| **Dernière session** | [[2026-11-10]] : recherche + création du vault |
| **Code** | rien encore (dépôt GitHub vide) |
| **Bloquant** | ADR à valider, outils à installer |

## ➡️ Prochaine étape
1. Relire et valider les ADR 🟡 → [[Index des décisions]]
2. Répondre aux décisions à prendre → [[Questions ouvertes]]
3. Installer les outils → [[Build, test et outils]] § Prérequis
4. Hello world (mod client), puis `/quests` (mod serveur)

## 🗺️ Carte du vault
### 01 · Projet
- [[Vision et objectifs]] : pourquoi, quoi, non-objectifs
- [[Roadmap]] : les phases 0 à 5 avec leurs cases à cocher
- [[Questions ouvertes]] : à décider ou à vérifier
- [[Glossaire]]

### 02 · Horizon (la plateforme)
- [[Horizon - Vue d'ensemble]] : ce qu'est Horizon, les types de mods, client et serveur
- [[Mods natifs - structure et manifeste]] : `horizon-mixin.toml`, `compatibility.json`, crates autorisées
- [[Hooks et API Horizon]] : ⭐ **la référence API** (réseau, événements, récompenses, persistance, GUI, commandes)
- [[Exemple - mod smp (patterns à reprendre)]] : ⭐ **le modèle à copier**
- [[Build, test et outils]] : launcher, `native.py`, tests, pièges Windows
- [[Publication et politique des mods]]
- [[Liens et ressources]]

### 03 · Référence FTB Quests (le modèle fonctionnel)
- [[FTB Quests - Vue d'ensemble]]
- [[Modèle de données FTB]] · [[Types de tâches FTB]] · [[Types de récompenses FTB]]
- [[Équipes et progression FTB]] · [[Éditeur et interface FTB]] · [[Format de fichiers FTB]] · [[Commandes FTB]]

### 04 · Conception (notre mod)
- [[Architecture]] : ⭐ événements → règles pures → effets
- [[Modèle de données Horizon Quest]]
- [[Tâches - correspondance hooks]] · [[Récompenses - implémentation]]
- [[Interface - Livre de quêtes]] : livre coffre (MVP) et livre graphique (V2)
- [[Persistance et synchronisation]] · [[Format des fichiers de quêtes]] · [[Commandes et permissions]]

### 05 · Décisions
- [[Index des décisions]]

### 06 · Journal
- Une note par session, au format `AAAA-MM-JJ` (modèle : `99 - Modèles/Modèle - Journal`)

## 🔁 Routine de session
**Début** : lire cette page → la dernière note du journal → la phase en cours dans la [[Roadmap]].
**Fin** :
1. Créer ou compléter la note du jour dans `06 - Journal`.
2. Cocher la [[Roadmap]].
3. Mettre à jour « Où on en est » et « Prochaine étape » ci-dessus.
4. Reporter les questions dans [[Questions ouvertes]] et les décisions dans une ADR.
5. Si une info sur Horizon a changé, mettre à jour la note de référence et son `verifie_le`.

## 📏 Conventions
- Notes en français, termes techniques en anglais quand c'est l'usage (hook, claim, tick…).
- Frontmatter : `tags`, `statut`, `date` / `verifie_le`, et `source` pour les notes de référence.
- Statut API : ✅ disponible · ⏳ pending (pas encore numéroté) · ❌ inexistant · ❓ inconnu.
- Priorités : 🔴 MVP · 🟠 V1 · 🟡 V2 · ⚪ plus tard.
- Les notes rapides vont dans `00 - Inbox`, à trier ensuite.
