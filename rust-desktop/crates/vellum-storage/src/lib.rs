//! # `vellum-storage`
//!
//! Couche de persistance SQLite (mode WAL) pour Vellum.
//!
//! Implémente les dépôts relationnels avec des requêtes exclusivement paramétrées :
//! - [`Database`] : initialisation du schéma, migrations et connexion SQLite WAL
//! - [`BookRepository`] : CRUD livres, formats, recherche et statistiques de lecture
//! - [`TagRepository`] : gestion des tags et associations livres-tags
//! - [`ParamRepository`] : stockage clé/valeur des paramètres applicatifs
//! - [`SyncRepository`] : journalisation des événements de synchronisation multi-clients

pub mod book_repo;
pub mod db;
pub mod param_repo;
pub mod sync_repo;
pub mod tag_repo;

pub use book_repo::BookRepository;
pub use db::Database;
pub use param_repo::ParamRepository;
pub use sync_repo::SyncRepository;
pub use tag_repo::TagRepository;
