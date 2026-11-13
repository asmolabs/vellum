# Vellum

Gestionnaire de bibliothèque numérique desktop écrit en **Rust** (Dioxus + SQLite WAL + Axum), conçu selon une architecture en couches propres (*Clean Architecture*).

## Fonctionnalités principales

- **Gestion de bibliothèque** : import multi-formats (EPUB, PDF, CBZ/CBR, MOBI/AZW3, Audiobooks), organisation configurable par templates (`NamingTemplate`), recherche avancée style Calibre (`SearchQuery`), audit de santé et détection/fusion de doublons (`library_audit`).
- **Lecteur intégré** : lecture EPUB, PDF, CBZ/CBR (mode Webtoon/Manga inclus) et TXT avec marque-pages, surlignages et suivi de progression (`reader_provider`, `reader_store`).
- **Éditeur & Convertisseur EPUB** : inspection, réparation et édition de table des matières (`EpubEditor`), conversions directes ou via format pivot EPUB (`ConversionService`).
- **Serveur OPDS 1.2 & Web Companion** : catalogue OPDS et lecteur web mobile servis via Axum (`OpdsServer`), avec authentification Bearer optionnelle.
- **Presse & Flux RSS/Atom** : génération d'EPUB quotidiens à partir de recettes RSS/Atom (`NewsService`).
- **Liseuses & Cloud** : synchronisation Kobo/Kindle (`DeviceService`), synchronisation multi-clients par dossier (`SyncService`), réplication SQLite via Litestream (`LitestreamService`), envoi par e-mail SMTP (`MailService`) et traduction IA bilingue (`vellum-plugins::translator`).

## Structure du dépôt

```text
library/
├── docs/
│   ├── architecture-c4.md   # Modèle C4 (Contexte, Conteneurs, Composants, Dépendances)
│   └── security.md          # Modèle de menaces, contrôles et durcissement
├── rust-desktop/
│   ├── Cargo.toml           # Workspace Rust (unsafe_code = "forbid")
│   └── crates/
│       ├── vellum-domain/   # Entités métier, ports et erreurs (0 dépendance infra)
│       ├── vellum-storage/  # Persistance SQLite WAL (requêtes paramétrées)
│       ├── vellum-plugins/  # Formats, convertisseurs, enrichissement, liseuses, traduction
│       ├── vellum-core/     # Services applicatifs, lecteur, OPDS/Web Companion, presse
│       └── vellum-ui/       # Application desktop Dioxus
└── TODO.md
```

## Démarrage rapide

### Prérequis
- Rust (toolchain stable récente, testé avec `rustc 1.99`)

### Tests et qualité

```bash
# Lancer l'ensemble des tests du workspace (58 tests)
cargo test --manifest-path rust-desktop/Cargo.toml

# Vérifier Clippy sur toutes les cibles
cargo clippy --manifest-path rust-desktop/Cargo.toml --workspace --all-targets

# Générer la documentation Rustdoc locale
cargo doc --manifest-path rust-desktop/Cargo.toml --no-deps
```

### Lancer l'application desktop

```bash
# Optionnel : protéger le serveur OPDS / Web Companion par un token Bearer
export VELLUM_OPDS_TOKEN="votre-secret-long"

cargo run --manifest-path rust-desktop/Cargo.toml -p vellum-ui
```

## Documentation détaillée

- [Architecture & Modèle C4](docs/architecture-c4.md)
- [Architecture Sécurité](docs/security.md)

