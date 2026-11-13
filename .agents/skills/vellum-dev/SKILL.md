---
name: vellum-dev
description: Workflow de développement, build, tests, clippy, Rustdoc et gestion du cache target/ pour le workspace Rust Vellum.
---

# Vellum — Workflow de développement

Utilise ce skill pour compiler, tester, auditer ou lancer l'application desktop **Vellum** (`rust-desktop/`).

## Commandes de référence

### 1. Tests complets du workspace
```bash
cargo test --manifest-path rust-desktop/Cargo.toml
```
Tests ciblés par crate :
```bash
cargo test --manifest-path rust-desktop/Cargo.toml -p vellum-core
cargo test --manifest-path rust-desktop/Cargo.toml -p vellum-plugins
cargo test --manifest-path rust-desktop/Cargo.toml -p vellum-storage
cargo test --manifest-path rust-desktop/Cargo.toml -p vellum-domain
```

### 2. Qualité et lints Clippy
```bash
cargo clippy --manifest-path rust-desktop/Cargo.toml --workspace --all-targets
```

### 3. Documentation Rustdoc
```bash
cargo doc --manifest-path rust-desktop/Cargo.toml --no-deps
```

### 4. Lancement de l'application desktop
```bash
# Optionnel : activer l'authentification Bearer sur le serveur OPDS / Web Companion
export VELLUM_OPDS_TOKEN="votre-secret"
cargo run --manifest-path rust-desktop/Cargo.toml -p vellum-ui
```

### 5. Contrôle de l'espace disque (`target/`)
Vérifier la taille du dossier de build :
```bash
du -sh rust-desktop/target
```
Si le dossier dépasse `4 Go`, nettoyer les anciens artefacts incrémentaux ou exécuter `cargo clean --manifest-path rust-desktop/Cargo.toml` avant un rebuild propre.
