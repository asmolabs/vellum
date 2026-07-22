# Bibliothèque Ebook Desktop - Vellum (Reflex + pywebview)

Cette version autonome regroupe le frontend réalisé en **Reflex** (Python UI) et la logique backend sous forme d'une application de bureau fluide utilisant `pywebview` pour l'affichage desktop, `FastAPI` pour les API REST et `SQLAlchemy` avec `SQLite` pour la gestion des métadonnées.

## Prérequis

- [uv](https://github.com/astral-sh/uv) (gestionnaire de paquets Python)
- Python >= 3.10

## Lancement de l'application

Pour installer les dépendances et exécuter l'application de bureau en une seule commande, lancez depuis le dossier `backend` :

```bash
uv run vellum
```

ou

```bash
uv run python -m vellum.main
```
