# Vellum — Règles projet pour les agents

## Architecture en couches (Clean Architecture)
Le workspace Rust (`crates/`) est découpé en 5 crates avec sens de dépendance strict :
1. `vellum-domain` : entités, traits/ports, erreurs (`VellumError`), configurations. **Interdiction** d'y ajouter des dépendances d'infrastructure (`rusqlite`, `axum`, `dioxus`, `zip`, `reqwest`).
2. `vellum-storage` : dépôts SQLite WAL (`rusqlite` uniquement avec requêtes paramétrées `params!`).
3. `vellum-plugins` : formats, convertisseurs, enrichisseurs, liseuses (`KoboReader.sqlite` en `SQLITE_OPEN_READ_ONLY` uniquement), traduction, plugins externes.
4. `vellum-core` : cas d'usage métier, lecteur, serveur OPDS/Web Companion (`axum`), presse RSS/Atom.
5. `vellum-ui` : interface Dioxus. Toute nouvelle fonctionnalité métier doit vivre dans `vellum-core`, pas dans les composants UI.

## Règles de sécurité obligatoires
- **`unsafe_code = "forbid"`** : ne jamais introduire de bloc `unsafe`.
- **HTML non fiable** (EPUB, CBZ, flux RSS/Atom) : toujours sanitiser via `ammonia::Builder` (voir `reader_provider.rs` et `news_service.rs`), jamais via regex seule.
- **Web Companion** (`web_companion_assets.rs`) : toujours échapper les données dynamiques injectées dans le DOM via `escapeHtml(...)`.
- **Noms de fichiers** : toujours passer par `BookService::sanitize_filename` (anti path traversal).
- **Serveur OPDS / Web Companion** (`opds_server.rs`) : préserver les en-têtes `X-Content-Type-Options: nosniff`, `X-Frame-Options: SAMEORIGIN`, le CORS explicite et le support `VELLUM_OPDS_TOKEN` (`router_with_token`).

## Qualité & Validation avant commit
- Toujours exécuter :
  - `cargo test`
  - `cargo clippy --workspace --all-targets`
- Surveiller la taille de `target/` (éviter l'accumulation inutile d'artefacts debug).
- Documentation technique : [README.md](README.md), [docs/architecture-c4.md](docs/architecture-c4.md), [docs/security.md](docs/security.md).

