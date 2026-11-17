# 📋 Feuille de Route & Tâches Vellum (TODO)

Dernière mise à jour : 10 novembre 2026 (Transition complète vers l'application native Rust ; version Java archivée dans la branche `archive/java-desktop` et tag `v1.0-java-final`)

---

## 🟢 1. Fonctionnalités Complétées (Terminé)

- [x] **Migration d'architecture**
  - [x] Remplacement de la stack Python/Reflex par une application Desktop Java 21 / JavaFX + Spring Boot 3.4.
  - [x] Nettoyage et purge des anciens fichiers du dépôt Git.
  - [x] Configuration de SQLite avec Hibernate Community Dialect et persistance des paramètres (`ParamRepository`).
- [x] **Catalogue & Formats de Livres**
  - [x] Extraction de métadonnées et couvertures EPUB (`MetadataService`).
  - [x] Support des bandes dessinées CBZ / CBR (lecture de `ComicInfo.xml` et extraction de la page 1).
  - [x] Support PDF avec extraction de métadonnées et rendu de couverture via *Apache PDFBox 3.0.4*.
  - [x] Import direct par glisser-déposer (Drag & Drop) de fichiers sur la grille de livres.
- [x] **Gestion de Bibliothèque & UI**
  - [x] Double mode d'affichage : Grille de cartes (`BooksGridView`) et Tableau tabulaire (`TableView`).
  - [x] Actions groupées (sélection multiple, ajout de tags en lot, suppression en lot).
  - [x] Tiroir latéral d'informations et métadonnées du livre sélectionné (`DetailDrawerView`).
  - [x] Dialogue d'édition manuelle des métadonnées du livre (`EditBookDialog`).
  - [x] Suivi de lecture : progression en pourcentage, index de chapitre, statut terminé, note.
  - [x] Dialogue de statistiques de lecture avec indicateurs KPIs et graphique camembert de répartition des genres (`StatsDialog`).
- [x] **Connectivité Mobile (OPDS)**
  - [x] Serveur de catalogue OPDS 1.2 (`/opds`, `/opds/books`) pour applications mobiles (*Moon+ Reader, Librera, KOReader*).
  - [x] Téléchargement et flux d'acquisition d'ebooks via HTTP.
  - [x] Onglet de guide de connexion et copie d'URL dans les paramètres.
- [x] **Système de Plugins & Extensions**
  - [x] Contrat d'interface unifié (`VellumPlugin`, `MetadataEnricherPlugin`, `BookTransformationPlugin`).
  - [x] Architecture modulaire de formats (`BookFormatPlugin`) :
    - [x] Découplage du parsing EPUB, PDF et CBZ/CBR en plugins de formats dédiés (`EpubFormatPlugin`, `PdfFormatPlugin`, `ComicFormatPlugin`).
    - [x] Support natif interne Amazon Kindle (`.mobi`, `.azw`, `.azw3`) via `MobiFormatPlugin` (parsing pur Java Palm PDB, en-têtes MOBI, tags EXTH et enregistrement d'image de couverture).
    - [x] Support natif interne Livres Audio (`.m4b`, `.m4a`, `.mp3`) via `AudiobookFormatPlugin` (parsing pur Java des tags ID3v2 pour MP3 et atomes MP4/QuickTime `moov.udta.meta.ilst` avec jaquette `covr`).
    - [x] Support des formats externes en Java :
      - [x] Archives JAR (`.jar`) avec découverte via `ServiceLoader<BookFormatPlugin>`, `vellum-plugin.properties` ou scan automatique de classes.
      - [x] Compilation et rechargement dynamique à chaud de fichiers source `.java` sans packaging préalable (via le `JavaCompiler` JDK intégré).
    - [x] Support dynamique des formats externes via scripts (`plugin.json` avec `type: "format"` et `supportedExtensions`).
    - [x] Détection automatique des formats supportés à l'importation sans liste statique en dur.
  - [x] **Moteurs de conversion de formats unifiés en tant que vrais plugins Vellum (`BookConverterPlugin`)** :
    - [x] Contrat d'interface `BookConverterPlugin extends VellumPlugin` avec type `PluginType.CONVERSION`.
    - [x] 6 plugins de conversion natifs 100% Java enregistrés comme composants Spring indépendants :
      - [x] `CbzToPdfConverterPlugin` (`converter-cbz-to-pdf`) : conversion BD vers PDF via PDFBox.
      - [x] `PdfToCbzConverterPlugin` (`converter-pdf-to-cbz`) : extraction haute résolution avec slider DPI configurable (72-300).
      - [x] `EpubToTextConverterPlugin` (`converter-epub-to-txt`) : extraction textuelle et markdown épurée.
      - [x] `TextToEpubConverterPlugin` (`converter-txt-to-epub`) : packaging OCF/XHTML avec sélection de typographie.
      - [x] `MobiToEpubConverterPlugin` (`converter-mobi-to-epub`) : décompresseur PalmDOC LZ77 pur Java et conversion vers EPUB.
      - [x] `EpubToPdfConverterPlugin` (`converter-epub-to-pdf`) : pagination et rendu vectoriel A4 via PDFBox avec taille de police configurable.
    - [x] Chaque convertisseur est individuellement activable/désactivable dans l'onglet Plugins des Paramètres.
    - [x] Badge visuel dédié `[Conversion]` dans la liste des plugins.
    - [x] Bouton de diagnostic « 🧪 Tester la configuration » pour chaque moteur de conversion.
    - [x] Support des convertisseurs externes via archives JAR (`ServiceLoader<BookConverterPlugin>`) et scripts (`type: "conversion"`).
    - [x] `ConversionService` dynamique et réactif aux activations/désactivations de plugins.
    - [x] Attachement direct des formats convertis à la même fiche de livre (multi-formats par entité).
  - [x] Plugins internes Spring (`OceanOfPdfCleanerPlugin`, `GoogleBooksEnricherPlugin`, `EpubTranslatorPlugin`).
  - [x] Chargeur de plugins externes (`ExternalPluginLoader`) pour scripts (Python, Bash avec manifeste `plugin.json`) et archives JAR.
  - [x] Rechargement à chaud des plugins et raccourci pour ouvrir le dossier `~/.vellum/plugins`.
  - [x] Form Builder dynamique supportant : `text`, `password` (avec œil 👁️), `file` 📂, `directory`, `slider` 🎚️, `textarea` 📝, `select`, `boolean`.
  - [x] Suite de tests automatisés (100% au vert sur `./gradlew test`).

---

## 🟡 2. Priorités Courantes (Plugins & Connexions Réelles)

- [x] **Bouton « 🧪 Tester la configuration » des plugins**
  - [x] Tester les clés API (requête HTTP en direct pour Google Books, vérification format jeton DeepL/GPT).
  - [x] Tester la disponibilité des scripts externes (vérification interpréteur `which`, exécution script `--test` ou détection fichier).
  - [x] Afficher un retour visuel direct dans `SettingsDialog` (badge vert ✓ ou message d'erreur rouge réactif sans rechargement de page).
- [x] **Modale d'options à l'exécution (Run Dialog)**
  - [x] Lors du clic sur un plugin de transformation (ex: traduction), afficher une fenêtre modale `PluginRunDialog` pour ajuster les paramètres de ce livre spécifique.
  - [x] Formulaire dynamique réactif (select, slider, boolean, textarea, text, password).
  - [x] Option « Attacher le format résultant à cette fiche de livre (multi-formats) ».
  - [x] Option « Enregistrer ces paramètres comme configuration par défaut ».
  - [x] Exécution en thread virtuel en arrière-plan avec indicateur d'avancement.
- [x] **Connexion des véritables moteurs pour les plugins internes**
  - [x] `GoogleBooksEnricherPlugin` : implémentation complète des requêtes réelles HTTP vers Google Books API (recherche par ISBN / Titre + Auteur, nettoyage HTML des résumés, extraction des catégories en tags, téléchargement de jaquette HD) avec système de secours automatique Open Library et boutons d'auto-complétion & d'enrichissement dans l'interface (`EditBookDialog` & `DetailDrawerView`).
  - [x] `EpubTranslatorPlugin` : architecture multi-agents connectée aux LLMs réels (Ollama local, LM Studio, OpenAI GPT-4o, DeepL) avec parser XHTML JSoup, agent planificateur de charte éditoriale, agent glossaire dynamique, cache de segments MD5 et validation QA.
  - [x] **Synchronisation physique des liseuses (Kobo & Kindle)** :
    - [x] Détection automatique des volumes USB (`/Volumes/KOBOeReader`, `/Volumes/Kindle`, Linux & Windows).
    - [x] Architecture de plugins dédiée `DeviceSyncPlugin` (`PluginType.DEVICE`).
    - [x] Moteur pur Java KePub (`KepubifyService`) injectant les balises `<span class="koboSpan">` pour le moteur de rendu ACCESS de Kobo (pagination par chapitre, instant-bookmark).
    - [x] Export de livres vers la liseuse avec conversion automatique au format optimal (`.kepub.epub` pour Kobo, `.azw3`/`.mobi`/`.pdf`/`.epub` pour Kindle).
    - [x] Import direct des livres présents sur la liseuse vers Vellum.
    - [x] Synchronisation bidirectionnelle de l'avancement de lecture (lecture de `KoboReader.sqlite`).
    - [x] Extraction et gestion des citations/surlignages (`My Clippings.txt` pour Kindle, table `Bookmark` SQLite pour Kobo).
    - [x] Interface utilisateur complète : dialogue modal `DeviceSyncDialog`, bouton d'accès rapide dans l'en-tête `HeaderView`, action d'export dans `DetailDrawerView` et visualisation des notes par livre.

---

## ⚪ 3. Lecteur Intégré Universel Multi-Formats (`ReaderDialog`)

- [x] **Améliorations du lecteur & typographie**
  - [x] Sélecteur de thème de lecture complet (Sombre `#181824`, Clair `#f8fafc`, Sépia `#fbf0d9`).
  - [x] Contrôle de la taille de police dynamique (`A-` / `A+`, de 12px à 32px) et bascule de police Serif / Sans-serif (`Georgia` / `Inter, system-ui`).
  - [x] Raccourcis clavier internes au lecteur (`Gauche`/`PageUp` pour page précédente, `Droite`/`PageDown`/`Espace` pour page suivante, `Échap` pour fermer).
  - [x] Barre de progression interactive et sélecteur de format à chaud pour les fiches multi-formats (EPUB, PDF, CBZ, MOBI, TXT).
- [x] **Support universel de lecture pour tous les formats**
  - [x] Lecteur EPUB avec extraction XHTML et nettoyage CSS responsive.
  - [x] Visionneuse intégrée pour fichiers PDF (rendu vectoriel dynamique page par page via Apache PDFBox 3.0.4 avec cache mémoire).
  - [x] Visionneuse haute performance pour bandes dessinées / mangas CBZ et CBR (pagination des images, zoom ajusté à la fenêtre).
  - [x] Lecteur Kindle MOBI / AZW / AZW3 avec décompression directe PalmDOC LZ77 en mémoire.
  - [x] Lecteur de fichiers texte et Markdown (`.txt`, `.md`).

---

## ⚪ 4. Ergonomie & UI / UX

- [x] **Raccourcis clavier globaux macOS & desktop**
  - [x] `Cmd + F` (ou `Ctrl + F`) : donner le focus à la barre de recherche et sélectionner le texte.
  - [x] `Cmd + ,` (ou `Ctrl + ,`) : ouvrir instantanément la modale des paramètres.
  - [x] `Espace` ou `Entrée` : ouvrir le livre sélectionné dans le lecteur universel (avec protection si le focus est sur un champ de saisie).
  - [x] `Suppr` / `Backspace` : ouvrir la boîte de dialogue de confirmation de suppression du livre sélectionné.
  - [x] `Échap` : fermer le tiroir latéral de détails s'il est ouvert.
- [x] **Filtres de statut de lecture**
  - [x] Boutons chips à bascule dans l'en-tête : « Tous », « 📖 En cours », « ✓ Terminés », « ⏳ À lire ».
  - [x] Filtrage instantané combiné avec la recherche textuelle et les tags de genres.

---

## ⚪ 5. Sauvegarde & Distribution

- [ ] **Synchronisation Litestream**
  - [ ] Vérifier et fiabiliser le démarrage/arrêt du processus démon Litestream sous macOS depuis `LitestreamService`.
- [ ] **Packaging macOS autonome**
  - [ ] Configurer la tâche Gradle `jpackage` pour générer un fichier `.dmg` ou une application `.app` autonome intégrant le runtime Java.

---

## 🦀 6. Version Native Rust

- [x] **Architecture Multi-Crates & Fondations**
  - [x] Workspace Cargo (`vellum-domain`, `vellum-storage`, `vellum-plugins`, `vellum-core`, `vellum-ui`).
  - [x] SQLite WAL via `rusqlite` avec migration automatique du schéma (`books`, `book_formats`, `tags`, `book_tags`, `params`).
  - [x] Serveur OPDS 1.2 natif ultra-rapide basé sur Tokio & Axum.
  - [x] Persistance et schéma complet avec colonne `isbn`, dates de publication et recherche plein champ.
- [x] **Enrichissement & Formats en Rust**
  - [x] Extraction de l'ISBN depuis les archives EPUB (`dc:identifier`), MOBI/AZW3 (tag EXTH 104) et fichiers externes.
  - [x] Moteur complet `GoogleBooksEnricherPlugin` natif Rust : interrogation Google Books API (par ISBN ou Titre+Auteur), nettoyage des balises HTML, fallback automatique Open Library et téléchargement de couverture haute résolution en arrière-plan.
  - [x] Intégration de l'enrichissement automatique à l'importation (`BookService::import_book`) et bouton d'enrichissement à la demande dans le tiroir de détails (`DetailDrawer`).
  - [x] **Synchronisation Cloud & Multi-Postes Google Drive (Architecture Décentralisée Anti-Corruption)**
    - [x] Architecture à flux de deltas événementiels décentralisés (`SyncAction`, `SyncEvent`, `SyncDelta`, `CloudSyncConfig`) isolant les fichiers binaires immuables (`files/`) des métadonnées incrémentales (`sync/deltas/`).
    - [x] Moteur de synchronisation bidirectionnel `SyncService` avec backend `FolderSyncBackend` supportant l'auto-détection de Google Drive sous macOS et Windows.
    - [x] Résolution de conflits multi-clients : Last-Write-Wins (LWW) sur la progression de lecture avec horodatage UTC, fusion additive des tags, tombstones de suppression, stockage immuable basé sur UUID.
    - [x] Téléchargement automatique ou à la demande des fichiers de livres distants (EPUB, PDF, CBZ...) entre tous les ordinateurs connectés.
    - [x] Tables de persistance `sync_events` et `applied_deltas` avec reprise automatique après déconnexion ou utilisation hors-ligne.
    - [x] Interface utilisateur intégrée dans `Header` (bouton synchro avec indicateur d'état actif/inactif) et `SettingsModal` (configuration du dossier Drive, détection automatique, options de téléchargement miroir complet, déclenchement manuel).
    - [x] Tâche de synchronisation périodique en arrière-plan toutes les 60s.
    - [x] Suite de tests d'intégration multi-clients (`test_two_clients_sync_via_google_drive_folder`) validée à 100%.
  - [x] **Séries, Sagas & Numérotation de Tomes (Standard Calibre & ComicInfo)**
    - [x] Modèle de données étendu (`series: Option<String>`, `series_index: Option<f64>`) sur `Book` et `ExtractedMetadata`.
    - [x] Migration de schéma SQLite idempotente (`ALTER TABLE books ADD COLUMN series TEXT; ALTER TABLE books ADD COLUMN series_index REAL;`).
    - [x] Extracteurs de formats enrichis : lecture de `<meta name="calibre:series">` et `series_index` sur EPUB + standard EPUB3 `belongs-to-collection` ; extraction `<Series>` et `<Number>` sur ComicInfo.xml pour CBZ/CBR.
    - [x] Badges visuels de séries sur `BookCard` (`📚 Dune • T.1`), édition dans `DetailDrawer`, filtre dédié dans `Sidebar` avec décompte par saga et récapitulatif actif avec fermeture en 1 clic.
  - [x] **Palette de Commandes Globale Keyboard-First (`Cmd + K` / `Ctrl + K`)**
    - [x] Composant modal `CommandPalette` avec flou glassmorphic et autofocus réactif.
    - [x] Raccourci global `Cmd + K` / `Ctrl + K` et bouton d'accès rapide dans la barre de recherche du `Header`.
    - [x] Recherche unifiée et instantanée (livres, sagas, auteurs, tags) et exécution d'actions rapides en une touche (lecture directe, import, synchro Google Drive, filtres de lecture, paramètres).
  - [x] Suite de tests 100% au vert sur `cargo test --workspace`.

---

## Feuille de route — Standalone, Web/Docker & Mobile (Android / iOS)

### Réalisé récemment
- [x] **Synchronisation Google Drive API v3 directe (sans logiciel de sync local)**
  - [x] Authentification OAuth 2.0 PKCE (`S256`) + serveur loopback local `127.0.0.1` (`gdrive_oauth.rs`), stockage atomique des jetons dans `~/.vellum/gdrive_tokens.json` (`0600` sous Unix) et rafraîchissement automatique du `refresh_token`.
  - [x] Backend REST `GoogleDriveApiBackend` (`gdrive_api_backend.rs`) implémentant `CloudSyncBackend` : résolution/création de `Vellum/sync/deltas` et `Vellum/files/...`, upload multipart (deltas) et resumable (livres), téléchargement atomique, validation anti-traversal des chemins relatifs.
  - [x] Configuration `CloudSyncMode::GoogleDriveApi` dans `SettingsModal` (Client ID, Client Secret optionnel, nom du dossier Drive, boutons Connecter / Déconnecter) et traductions FR/EN.
  - [x] Test d'intégration avec serveur Google Drive + OAuth mocké (`test_google_drive_api_backend_and_token_refresh`).

### Phase 1 — Mode Standalone pur & Durcissement Sécurité
- [x] **Contrôle du serveur réseau local en mode Desktop**
  - [x] Option `opds_enabled` dans `AppConfig`, `param_repo` et `SettingsModal` pour activer/désactiver le démarrage automatique du serveur OPDS / Web Companion (mode desktop 100 % hors-ligne sans aucun port ouvert).
- [x] **Gestion sécurisée des secrets locaux**
  - [x] `SecretStore` (`secret_store.rs`) : stockage de `SmtpConfig.password` et `CloudSyncConfig.google_client_secret` via le trousseau système (Linux Secret Service `secret-tool` / macOS Keychain `security`) avec repli fichier `secrets.json` (`0600`) en mode headless/Docker et migration automatique depuis SQLite au démarrage.

### Phase 2 — Mode Serveur Headless (`vellum serve`) & Docker
- [x] **Serveur headless sans dépendance graphique (`vellum serve`)**
  - [x] Sous-commande `vellum serve --host 0.0.0.0 --port 8080 --data-dir /data --library-dir /books` dans `vellum-cli` démarrant `AppServices` (sans GTK/WebKit2GTK), `OpdsServer` et la synchro périodique Google Drive (plus variables `VELLUM_DATA_DIR`, `VELLUM_LIBRARY_DIR`, `VELLUM_OPDS_PORT`, `VELLUM_OPDS_ENABLED`).
  - [x] Endpoint `/api/v1/health` (sans auth même avec `VELLUM_OPDS_TOKEN`) pour les sondes de santé Docker / reverse proxy.
- [x] **Empaquetage Docker mono-conteneur léger (inspiré des bonnes pratiques BookOrbit)**
  - [x] `Dockerfile` multi-stage Rust produisant une image runtime minimale non-root (`tini`, `HEALTHCHECK`, binaires/librairies strictement nécessaires).
  - [x] `docker-compose.yml` mono-conteneur avec SQLite WAL (`/data`), bibliothèque (`/books`), `read_only: true`, `cap_drop: ALL`, `no-new-privileges: true` et variables `VELLUM_OPDS_TOKEN`, `PUID`/`PGID`.

### Phase 3 — Web App complète & Écosystème Liseuses
- [ ] **Élever le Web Companion au niveau Web App complète (`web_api.rs` + assets)**
  - [x] Upload de livres par glisser-déposer depuis le navigateur (`POST /api/books/upload`, EPUB, PDF, CBZ/CBR, TXT...).
  - [x] Édition des métadonnées, tags et séries depuis le navigateur (`POST /api/books/:id/metadata`).
  - [x] Déclenchement et suivi de la synchronisation Google Drive depuis l'interface Web (`GET /api/sync/status`, `POST /api/sync`).
  - [x] Lecteur audio intégré (MP3 / M4B / OGG / FLAC) avec streaming HTTP Range et sauvegarde de la progression (`GET /api/books/:id/audio`).
  - [x] Tableau de bord statistiques de lecture (`GET /api/stats` + bandeau stats Web).
- [x] **Synchronisation KOReader**
  - [x] Routes compatibles KOReader Progress Sync sur `OpdsServer` (`/users/auth`, `/users/create`, `PUT /syncs/progress`, `GET /syncs/progress/:document`).
- [x] **Fournisseurs de métadonnées supplémentaires**
  - [x] Enrichisseurs `HardcoverEnricherPlugin` (GraphQL Hardcover) et `ComicVineEnricherPlugin` (API ComicVine pour CBZ/CBR) ajoutés et enregistrés dans `vellum-plugins` et `PluginService`.
- [ ] **Multi-utilisateurs léger (optionnel en mode serveur/Docker)**
  - [ ] Isolation optionnelle des progressions/statuts par utilisateur lorsque Vellum tourne en mode `vellum serve`.
- [ ] **Revue d'ergonomie & UX (inspirée de BookOrbit)**
  - [x] Web Companion : section *« Reprendre la lecture »* en tête de page, onglets de filtrage rapide (`Tous`, `En cours`, `Terminés`, `Audio`) et barres de progression.
  - [x] Simplifier `SettingsModal` en 4 onglets (`Général`, `Cloud & Sync`, `Serveur & Kindle`, `Outils & Plugins`) avec largeur responsive (`min(680px, 94vw)`).
  - [ ] Auditer les écrans Desktop restants (onboarding premier lancement, états vides guidés) et améliorer l'ergonomie du lecteur (typographie/thèmes/marges, accès rapide aux notes).

### Phase 4 — Applications Mobiles Android & iOS (Dioxus Mobile)
- [ ] **Préparation du cœur Rust pour cibles mobiles (`aarch64-linux-android`, `aarch64-apple-ios`)**
  - [ ] Isoler les modules desktop/OS-spécifiques (`LitestreamService`, `DeviceService`, `WatchFolderService`, `OpdsServer`, `script_plugin.rs`) derrière des *feature flags* Cargo (`desktop` vs `mobile`).
  - [ ] Remplacer `open::that` + callback `TcpListener` de `gdrive_oauth.rs` par un flux OAuth mobile (URI de redirection / `ASWebAuthenticationSession` iOS / Custom Tabs Android).
  - [ ] Remplacer `rfd::FileDialog` dans `vellum-ui` par une abstraction compatible sélecteur de fichiers mobile.
- [ ] **Adaptation UI tactile & responsive (`vellum-ui`)**
  - [ ] Basculer les largeurs fixes desktop (modales, tiroirs, grille) vers des mises en page responsives adaptées au tactile.
  - [ ] Ajouter la configuration `Dioxus.toml` mobile et valider les builds `dx build --platform android` et `dx build --platform ios`.
  - [ ] Synchronisation transparente Mobile ↔ Desktop ↔ Docker via Google Drive API (et connexion optionnelle au serveur `vellum serve`).
