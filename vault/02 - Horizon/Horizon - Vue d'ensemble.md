---
tags: [horizon, reference]
source: playhorizon.dev (/, /docs, /mods, /docs/make-a-mod, /docs/mod-code, /docs/apis, /docs/publish-a-mod, /docs/api/server), github.com/horizon-rust/horizon/docs
verifie_le: 2026-11-10
---
# Horizon : vue d'ensemble

> « Horizon is Minecraft: Java Edition 26.3 rewritten in Rust. » Il s'installe via un petit launcher et tourne sur les fichiers de jeu de **votre propre** copie de Minecraft.

## Ce que c'est (et ce que ce n'est pas)
- C'est une **réimplémentation clean-room** en Rust du client, de la logique de jeu et d'un serveur dédié (`horizon-server`). La parité avec le vanilla est vérifiée par 281 « harnesses ».
- Ce **n'est PAS un modloader** : les jars Java (Forge, Fabric, NeoForge) ne se chargent pas et les Mixins Java ne s'appliquent pas. **FTB Quests ne peut donc pas tourner** dessus, il faut une réécriture.
- Version de jeu : **Java Edition 26.3 uniquement**. Les mondes de 1.8 à 26.2 sont mis à niveau.
- Plateformes : Linux x86-64 (Vulkan), macOS 12+ (Apple Silicon ou Intel), **Windows x64 ou ARM64** (Vulkan ou DX12).
- Licence d'Horizon : **MIT**. Il ne contient ni code ni assets Minecraft. Ce n'est pas un produit officiel.
- Version du launcher vue le 2026-11-10 : 0.2.58.

## Les types de mods
| Type | Fichier | Contenu | Pour nous |
|---|---|---|---|
| **Mod de contenu** | `.hmod` | data-only : blocs, items, recettes, loot tables, textures, lang (`horizon-mod.toml`) | utile pour les **assets** (icônes, textures du livre, item « livre de quêtes ») |
| **Mod natif** | `.hnmod` | du **code Rust compilé dans le jeu** (`horizon-mixin.toml`), non sandboxé | ✅ **le cœur de Horizon Quest** |
| Combiné | `.hmod` avec `horizon-native.json` + `mixins/<id>/` + `assets/` | du code et des assets | 🎯 la cible finale (code + textures + lang) |
| Shader, resource pack | — | — | — |

Détails : [[Mods natifs - structure et manifeste]]

## Côtés client et serveur
- Un même binaire contient plusieurs `Game` : la fenêtre, le serveur intégré du solo et le serveur dédié. Chaque hook déclare son côté (`server`, `client` ou `both`).
- **Solo / hôte LAN** : les mods tournent dans le jeu de l'hôte **et** dans son serveur intégré.
- **Rejoindre un serveur** : seuls les hooks client agissent.
- **Serveur dédié** : il faut un *engine export* compilé avec `--server-output` (voir [[Build, test et outils]]). ⚠️ La doc publique le classe encore en « Not there yet ». Il n'y a **pas encore de handshake de compatibilité** (suivi #6304).
- Les mods natifs **ne sont pas** dans la vérification de mods à la connexion : notre mod devra faire son **propre handshake** via les mod messages.

Détails : [[Hooks et API Horizon]] · [[Persistance et synchronisation]]

## Stabilité
| API | Niveau |
|---|---|
| API des mods natifs | **experimental** |
| API de contrôle serveur `/api/v1` | stable |
| Shaders | planned |

Les **hook points** ne sont qu'ajoutés et leurs contrats sont gelés (un changement donne un `_v2`). Les **injections sur des fonctions brutes** peuvent casser à chaque commit. → On utilise **uniquement des hook points** dès que possible ([[ADR-003 Hook points plutôt qu'injections brutes]]).

## Écosystème
- Le catalogue https://playhorizon.dev/mods compte 158 projets (133 mods natifs) au 2026-11-10. **Aucun mod de quêtes.**
- Mods portés notables : JEI, Create, AE2, Jade… Il y a un `native_show_recipes` pour ouvrir le visualiseur de recettes installé depuis **notre** livre.
- Mod serveur de référence : `smp` → [[Exemple - mod smp (patterns à reprendre)]].

Liens : [[Liens et ressources]]
