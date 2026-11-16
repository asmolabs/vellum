---
name: vellum-dev
description: Workflow de développement, build, tests, clippy, Rustdoc et gestion du cache target/ pour le workspace Rust Vellum.
---

# Vellum — Workflow de développement

Utilise ce skill pour compiler, tester, auditer ou lancer l'application desktop **Vellum**.

## Commandes de référence

### 1. Tests complets du workspace
```bash
cargo test
```
Tests ciblés par crate :
```bash
cargo test -p vellum-core
cargo test -p vellum-plugins
cargo test -p vellum-storage
cargo test -p vellum-domain
```

### 2. Qualité et lints Clippy
```bash
cargo clippy --workspace --all-targets
```

### 3. Documentation Rustdoc
```bash
cargo doc --no-deps
```

### 4. Lancement de l'application desktop
```bash
# Optionnel : activer l'authentification Bearer sur le serveur OPDS / Web Companion
export VELLUM_OPDS_TOKEN="votre-secret"
cargo run -p vellum-ui
```

### 5. Contrôle de l'espace disque (`target/`)
Vérifier la taille du dossier de build :
```bash
du -sh target
```
Si le dossier dépasse `4 Go`, nettoyer les anciens artefacts incrémentaux ou exécuter `cargo clean` avant un rebuild propre.

