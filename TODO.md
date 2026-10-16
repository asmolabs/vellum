# 📋 Feuille de Route & Tâches Vellum (TODO)

Dernière mise à jour : 8 octobre 2026

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
- [ ] **Connexion des véritables moteurs pour les plugins internes**
  - [ ] `GoogleBooksEnricherPlugin` : implémenter la requête HTTP réelle vers l'API REST Google Books pour enrichir description, éditeur et tags.
  - [x] `EpubTranslatorPlugin` : architecture multi-agents connectée aux LLMs réels (Ollama local, LM Studio, OpenAI GPT-4o, DeepL) avec parser XHTML JSoup, agent planificateur de charte éditoriale, agent glossaire dynamique, cache de segments MD5 et validation QA.

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
