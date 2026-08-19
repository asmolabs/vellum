# Vellum Desktop — Java 25 & JavaFX Edition

Application de gestion de bibliothèque d'ebooks (EPUB, MOBI, AZW3) ultra-moderne développée en **Java 25**, **Spring Boot 3.4 / 4**, **Spring Data JPA**, **SQLite (mode WAL)**, **Litestream** (sauvegarde & réplication continue) et **JavaFX**.

---

## Architecture Logicielle

L'application adopte une stricte architecture en couches :

1. **Couche Présentation (`com.asmolabs.vellum.ui`)** :
   - Interface JavaFX avec thème sombre Slate & Violet moderne (`#0f111a`, accent `#8b5cf6`).
   - `SidebarView` : Barre latérale rétractable avec importateur, sélecteur de dossier et liste des tags avec badges de comptage.
   - `HeaderView` : Barre de recherche instantanée, indicateur d'état Litestream en direct, sélecteur de mode grille/tableau.
   - `BooksGridView` : Grille de cartes dynamiques avec couvertures, badges de formats et interactions fluides.
   - `DetailDrawerView` : Tiroir latéral d'inspection complète et d'actions (lecture, édition, transformation).
   - `ReaderDialog` : Lecteur EPUB intégré avec JavaFX `WebView`, pagination par chapitres et thème sombre injecté.
   - `EditBookDialog` : Éditeur de métadonnées, gestionnaire de tags (pills) et sélecteur de couverture.
   - `SettingsDialog` : Configuration de la bibliothèque, de Litestream et des plugins.

2. **Couche Métier / Service (`com.asmolabs.vellum.service`)** :
   - `BookService` : Importation, organisation sur disque sous `Auteur/Titre (UUID)/`, renommage dynamique, mise à jour de couverture, suppression et gestion des formats.
   - `MetadataService` : Parser EPUB autonome en pur Java (archives ZIP, XML DOM/Dublin Core, extraction automatique des couvertures).
   - `PluginService` : Système d'enrichissement (Google Books API), nettoyeur EPUB OceanOFPDF et traducteur.
   - `LitestreamService` : Pilotage du binaire Litestream, génération de `litestream.yml`, exécution du processus de réplication en tâche de fond et restauration de snapshots.

3. **Couche Persistance (`com.asmolabs.vellum.repository` & `domain`)** :
   - Entités JPA : `Book`, `BookFormat`, `Tag`, `Param`.
   - Repositories Spring Data JPA avec requêtes JPQL/SQL dérivées.
   - SQLite configuré en mode **WAL** (`PRAGMA journal_mode=WAL;`), clé étrangère active et timeout busy configuré pour une compatibilité parfaite avec Litestream.

---

## Prérequis

- **Java Development Kit (JDK) 25**
- **Gradle 9.x** (ou utilisation du wrapper inclus `./gradlew`)
- *(Optionnel)* Binaire `litestream` installé dans votre PATH (`brew install litestream`) pour la réplication continue de la base de données.

---

## Lancement & Commandes

### Exécution de l'application
Depuis le dossier `java-desktop` :

```bash
./gradlew run
```

### Lancement des tests automatisés
```bash
./gradlew test
```

### Construction du package autonome
```bash
./gradlew build
```
