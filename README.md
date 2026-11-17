<div align="center">
  <img src="docs/images/vellum-icon.png" alt="Vellum" width="128" height="128">
  <h1>Vellum</h1>
</div>

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
├── Cargo.toml               # Workspace Rust (unsafe_code = "forbid")
├── crates/
│   ├── vellum-domain/       # Entités métier, ports et erreurs (0 dépendance infra)
│   ├── vellum-storage/      # Persistance SQLite WAL (requêtes paramétrées)
│   ├── vellum-plugins/      # Formats, convertisseurs, enrichissement, liseuses, traduction
│   ├── vellum-core/         # Services applicatifs, lecteur, i18n FR/EN, OPDS/Web Companion
│   ├── vellum-cli/          # Interface ligne de commande (import, audit, indexation, export)
│   └── vellum-ui/           # Application desktop Dioxus
├── docs/
│   ├── architecture-c4.md   # Modèle C4 (Contexte, Conteneurs, Composants, Dépendances)
│   └── security.md          # Modèle de menaces, contrôles et durcissement
└── TODO.md
```

## Démarrage rapide

### Prérequis
- Rust (toolchain stable avec `clippy` et `rustfmt`)

### Tests et qualité

```bash
# Vérifier le formatage
cargo fmt --all -- --check

# Lancer l'ensemble des tests du workspace (unitaires, intégration et doc-tests)
cargo test

# Vérifier Clippy sur toutes les cibles (zéro warning)
cargo clippy --workspace --all-targets -- -D warnings

# Générer la documentation Rustdoc locale (zéro warning)
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

### Lancer l'application desktop

```bash
# Optionnel : protéger le serveur OPDS / Web Companion par un token Bearer
export VELLUM_OPDS_TOKEN="votre-secret-long"

cargo run -p vellum-ui
```

## Documentation détaillée

- [Architecture & Modèle C4](docs/architecture-c4.md)
- [Architecture Sécurité](docs/security.md)

