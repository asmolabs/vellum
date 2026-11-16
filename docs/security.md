# Architecture Sécurité — Vellum

Ce document recense la surface d'attaque de **Vellum**, les contrôles de sécurité implémentés dans le code et les recommandations opérationnelles.

---

## 1. Surface d'attaque & Modèle de menaces

| Vecteur | Actif exposé | Risque principal |
| :--- | :--- | :--- |
| Fichiers EPUB / CBZ / RSS non fiables | WebView Dioxus, Web Companion, EPUB générés | XSS stocké (`<script>`, `onerror`, `javascript:`, `<svg onload=...>`) |
| Serveur HTTP `OpdsServer` (LAN `0.0.0.0`) | Bibliothèque locale, fichiers livres, API progression | Accès non autorisé sur LAN, CORS trop permissif, sniffing MIME, clickjacking |
| Noms de fichiers / archives ZIP | Système de fichiers hôte | Path traversal (`../`, `.`, `..`, caractères de contrôle) |
| Base SQLite locale & bases tierces | Données bibliothèque, `metadata.db`, `KoboReader.sqlite` | Injection SQL, corruption de base tierce |
| Mémoire & FFI Rust | Processus desktop | Corruption mémoire (`unsafe`) |

---

## 2. Contrôles de sécurité implémentés

### A. Sûreté mémoire (`unsafe_code = "forbid"`)
- `Cargo.toml` (à la racine du workspace) interdit strictement tout bloc `unsafe` dans l'ensemble des crates du workspace via `[workspace.lints.rust] unsafe_code = "forbid"`.

### B. Protection anti-XSS par liste blanche (`ammonia`)
- **Lecteur interne (`vellum-core/src/reader_provider/html.rs`)** :
  - `sanitize_reader_html` s'appuie sur `ammonia::Builder` (analyseur HTML5 `html5ever`, liste blanche stricte de balises/attributs).
  - Bloque `<script>`, `<iframe>`, `<svg>`, `onerror`/`onload`, `javascript:` tout en autorisant les schémas `data:` pour les images embarquées et les attributs `class`, `id`, `alt`, `title`, `src`, `width`, `height`, `loading`.
- **Générateur de presse RSS/Atom (`vellum-core/src/news_service/feed_parser.rs`)** :
  - `sanitize_article_html` supprime les pixels de suivi `1×1` puis passe le contenu dans `ammonia::Builder` avant l'écriture dans l'EPUB généré.
- **Web Companion (`vellum-core/src/opds_server/web_companion_assets.rs` et `assets/web_companion/{library,reader}.html`)** :
  - Les données dynamiques et messages d'erreur injectés dans le DOM passent par `escapeHtml(...)`.

### C. Durcissement du serveur OPDS & Web Companion (`vellum-core/src/opds_server/mod.rs`)
- **Authentification Bearer optionnelle** :
  - `OpdsServer::run_server` lit la variable d'environnement `VELLUM_OPDS_TOKEN`.
  - Lorsque défini (ou via `OpdsServer::router_with_token`), le middleware `auth_and_security_headers` exige l'en-tête `Authorization: Bearer <token>` et retourne `401 Unauthorized` sinon.
- **En-têtes de sécurité HTTP** appliqués à toutes les réponses :
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: SAMEORIGIN`
- **CORS explicite** :
  - Remplacement de `CorsLayer::permissive()` par des méthodes explicites (`GET`, `POST`, `OPTIONS`) et des en-têtes autorisés explicites (`CONTENT_TYPE`, `AUTHORIZATION`).
- **Téléchargements sûrs** :
  - L'en-tête `Content-Disposition` sanitise le nom de fichier via `BookService::sanitize_filename`.

### D. Protection contre le Path Traversal
- `BookService::sanitize_filename` (`vellum-core/src/book_service/mod.rs`) :
  - Remplace les séparateurs et caractères interdits (`/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`) ainsi que les caractères de contrôle (`c.is_control()`).
  - Supprime les points en début/fin (`.trim_matches('.')`) et rejette explicitement `.` et `..` en retournant un nom de repli sûr (`book`).

### E. Persistance SQLite
- Toutes les requêtes dans `vellum-storage` utilisent des **paramètres liés** (`rusqlite::params!`).
- Les importateurs de bases externes (`calibre_importer.rs`, `ereader/kobo.rs`) ouvrent les fichiers tiers avec `OpenFlags::SQLITE_OPEN_READ_ONLY`.

---

## 3. Recommandations d'exploitation

1. **Sur réseau partagé / Wi-Fi public** : toujours définir `VELLUM_OPDS_TOKEN` avant de démarrer l'application afin d'activer l'authentification Bearer sur le serveur OPDS/Web Companion.
2. **Credentials SMTP** : `SmtpConfig` est actuellement persisté dans la configuration JSON locale de l'application ; l'utilisation d'un mot de passe d'application dédié (et non du mot de passe principal) est recommandée en attendant une intégration éventuelle avec le trousseau système (macOS Keychain).

