# Horizon Quest : instructions pour Claude

Mod de quêtes façon FTB Quests pour **Horizon** (Minecraft Java 26.3 réécrit en Rust), écrit comme **mod natif Rust**.
Réponds en **français**.

## Mémoire du projet = `vault/` (Obsidian)
**Au début de chaque session, AVANT toute autre chose :**
1. Lire `vault/00 - Accueil.md` (état, prochaine étape, carte du vault).
2. Lire la note la plus récente de `vault/06 - Journal/`.
3. Selon la tâche, lire les notes concernées. Les plus importantes :
   - `vault/02 - Horizon/Hooks et API Horizon.md` (la référence API)
   - `vault/02 - Horizon/Exemple - mod smp (patterns à reprendre).md` (le modèle de code)
   - `vault/04 - Conception/Architecture.md`

**À la fin de chaque session (ou après une étape importante) :**
1. Créer ou compléter `vault/06 - Journal/AAAA-MM-JJ.md` (modèle : `vault/99 - Modèles/Modèle - Journal.md`).
2. Cocher `vault/01 - Projet/Roadmap.md`.
3. Mettre à jour « Où on en est » et « Prochaine étape » dans `vault/00 - Accueil.md`, ainsi que `mise_a_jour`.
4. Questions → `vault/01 - Projet/Questions ouvertes.md`. Décision importante → nouvelle ADR dans `vault/05 - Décisions/` + `Index des décisions.md`.
5. Une info Horizon apprise ou corrigée → mettre à jour la note de référence et son `verifie_le`.

Conventions du vault : liens wiki `[[Nom de note]]`, frontmatter YAML, statuts API ✅ / ⏳ pending / ❌ / ❓.

## Règles du projet
- **Ne jamais copier de code, d'assets ou de textes de FTB Quests** (All Rights Reserved). On réimplémente les concepts.
- Aucun code ni asset Mojang dans le dépôt : les données du jeu se lisent au runtime.
- N'utiliser que des **hook points** du catalogue, sans injection brute sur des fonctions (ADR-003), et aucun hook ⏳ pending non numéroté.
- Seules les crates présentes dans le `Cargo.lock` d'Horizon sont autorisées.
- `RUSTFLAGS=-D warnings` : un warning casse le build.
- La logique va dans des modules **purs testables** (`hq_*.rs`, `cargo test`). Le code qui touche `Game` va dans les `*_glue.rs` (pattern smp).
- **Ne jamais exécuter soi-même une commande avec `--trust-native-code`** (règle Horizon : c'est la décision du joueur). On prépare la commande, l'utilisateur la lance.

## Git et GitHub
- Le seul auteur est **LKTbgoodall** : `user.name = LKTbgoodall`, `user.email = bgoodall79@gmail.com` (config locale du dépôt).
- **Aucun `Co-Authored-By`**, aucune ligne d'attribution Claude ou « Generated with », que ce soit dans les commits ou dans les PR.
- Remote : https://github.com/LKTbgoodall/Horizon_quest (branche `main`).

## Références externes
- Docs Horizon : https://playhorizon.dev/docs
- Moteur : https://github.com/horizon-rust/horizon (`docs/native-mods.md`, `native-hooks.json`)
- Registre et exemples : https://github.com/horizon-rust/horizon-mods (`native/smp`)
- Clones locaux de référence prévus dans `ext/` (gitignoré).
