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
    - [x] Support dynamique des formats externes via scripts (`plugin.json` avec `type: "format"` et `supportedExtensions`).
    - [x] Détection automatique des formats supportés à l'importation sans liste statique en dur.
  - [x] Plugins internes Spring (`OceanOfPdfCleanerPlugin`, `GoogleBooksEnricherPlugin`, `EpubTranslatorPlugin`).
  - [x] Chargeur de plugins externes (`ExternalPluginLoader`) pour scripts (Python, Bash avec manifeste `plugin.json`) et archives JAR.
  - [x] Rechargement à chaud des plugins et raccourci pour ouvrir le dossier `~/.vellum/plugins`.
  - [x] Form Builder dynamique supportant : `text`, `password` (avec œil 👁️), `file` 📂, `directory`, `slider` 🎚️, `textarea` 📝, `select`, `boolean`.
  - [x] Suite de tests automatisés (100% au vert sur `./gradlew test`).

---

## 🟡 2. Priorités Courantes (Plugins & Connexions Réelles)

- [ ] **Bouton « 🧪 Tester la configuration » des plugins**
  - [ ] Tester les clés API (vérifier qu'une clé Google Books ou DeepL est valide sans faire d'import complet).
  - [ ] Tester la disponibilité des scripts externes (vérifier que `python3` ou l'exécutable configuré répond).
  - [ ] Afficher un retour visuel direct dans `SettingsDialog` (badge vert ✓ ou message d'erreur rouge).
- [ ] **Modale d'options à l'exécution (Run Dialog)**
  - [ ] Lors du clic sur un plugin de transformation (ex: traduction), afficher une fenêtre modale pour ajuster les paramètres de ce livre spécifique.
  - [ ] Option « Se souvenir de ce choix pour les prochains lancements ».
- [ ] **Connexion des véritables moteurs pour les plugins internes**
  - [ ] `GoogleBooksEnricherPlugin` : implémenter la requête HTTP réelle vers l'API REST Google Books pour enrichir description, éditeur et tags.
  - [ ] `EpubTranslatorPlugin` : connecter un modèle d'IA réel (Ollama local sur `http://localhost:11434/api/generate` ou API DeepL/OpenAI).

---

## ⚪ 3. Lecteur Intégré (Reader)

- [ ] **Améliorations du lecteur EPUB**
  - [ ] Sélecteur de thème de lecture (Sombre, Clair, Sépia).
  - [ ] Contrôle de la taille de police (+ / -) et choix de la typographie (Serif / Sans-serif).
  - [ ] Table des matières interactive cliquable.
- [ ] **Support de lecture pour d'autres formats**
  - [ ] Visionneuse intégrée pour fichiers PDF (rendu page par page via PDFBox).
  - [ ] Visionneuse de pages d'images pour fichiers CBZ / CBR (mode bande dessinée/manga).

---

## ⚪ 4. Ergonomie & UI / UX

- [ ] **Raccourcis clavier globaux**
  - [ ] `Cmd + F` : donner le focus à la barre de recherche.
  - [ ] `Cmd + ,` : ouvrir les paramètres.
  - [ ] `Espace` ou `Entrée` : ouvrir le livre sélectionné dans le lecteur.
  - [ ] `Suppr` / `Backspace` : supprimer le livre sélectionné (avec confirmation).
- [ ] **Tri et filtres avancés**
  - [ ] Trier la grille/tableau par date d'ajout, note, progression de lecture ou titre.
  - [ ] Filtrer les livres par statut : "Non commencé", "En cours", "Terminé".

---

## ⚪ 5. Sauvegarde & Distribution

- [ ] **Synchronisation Litestream**
  - [ ] Vérifier et fiabiliser le démarrage/arrêt du processus démon Litestream sous macOS depuis `LitestreamService`.
- [ ] **Packaging macOS autonome**
  - [ ] Configurer la tâche Gradle `jpackage` pour générer un fichier `.dmg` ou une application `.app` autonome intégrant le runtime Java.
