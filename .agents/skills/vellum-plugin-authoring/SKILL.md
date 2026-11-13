---
name: vellum-plugin-authoring
description: Procédure pour créer, enregistrer et tester un nouveau plugin Vellum (format, convertisseur, enrichisseur ou synchronisation liseuse) dans vellum-plugins.
---

# Création d'un plugin dans Vellum

Utilise ce skill pour ajouter un nouveau plugin intégré dans `rust-desktop/crates/vellum-plugins/`.

## 1. Choisir le contrat dans `vellum-domain`
Les traits sont définis dans `rust-desktop/crates/vellum-domain/src/plugin.rs` :
- **Format de livre** : `BookFormatPlugin` (détection, extraction de métadonnées et couverture)
- **Convertisseur** : `BookConverterPlugin` (conversion directe ou vers/depuis le format pivot `epub`)
- **Enrichisseur** : `MetadataEnricherPlugin` (recherche de métadonnées distantes)
- **Synchronisation liseuse** : `DeviceSyncPlugin` (montage USB, progression, annotations)
- **Transformation** : `BookTransformationPlugin`

> **Règle d'architecture** : ne jamais importer de dépendances techniques dans `vellum-domain`. Si le plugin lit une base SQLite tierce, utiliser impérativement `OpenFlags::SQLITE_OPEN_READ_ONLY`.

## 2. Implémenter le plugin dans `vellum-plugins`
1. Créer un module dans le sous-dossier adapté :
   - `src/format/`
   - `src/converter/`
   - `src/ereader/`
   - ou un module dédié (ex. `src/enricher.rs`)
2. Exposer et enregistrer le plugin dans `rust-desktop/crates/vellum-plugins/src/lib.rs` via la fonction correspondante (`built_in_format_plugins`, `built_in_converter_plugins`, `built_in_device_plugins`, etc.).
3. Mapper toute erreur ZIP ou I/O technique vers `VellumError` (`vellum_domain::VellumError`).

## 3. Tester et valider
1. Ajouter des tests unitaires ou d'intégration dans `rust-desktop/crates/vellum-plugins/tests/`.
2. Vérifier :
```bash
cargo test --manifest-path rust-desktop/Cargo.toml -p vellum-plugins
cargo test --manifest-path rust-desktop/Cargo.toml -p vellum-core
cargo clippy --manifest-path rust-desktop/Cargo.toml --workspace --all-targets
```
