use vellum_domain::{Book, VellumError, VellumResult, VirtualLibrary};
use vellum_storage::VirtualLibraryRepository;

use crate::SearchQuery;

/// Service métier pour la gestion des bibliothèques virtuelles (vues enregistrées).
#[derive(Clone)]
pub struct VirtualLibraryService {
    repo: VirtualLibraryRepository,
}

impl VirtualLibraryService {
    pub fn new(repo: VirtualLibraryRepository) -> Self {
        Self { repo }
    }

    /// Crée et persiste une nouvelle bibliothèque virtuelle.
    pub fn create(
        &self,
        name: &str,
        query: &str,
        color: Option<String>,
    ) -> VellumResult<VirtualLibrary> {
        let name = name.trim();
        let query = query.trim();

        if name.is_empty() {
            return Err(VellumError::InvalidArgument(
                "Le nom de la bibliothèque virtuelle ne peut pas être vide".to_string(),
            ));
        }
        if query.is_empty() {
            return Err(VellumError::InvalidArgument(
                "La requête de la bibliothèque virtuelle ne peut pas être vide".to_string(),
            ));
        }

        let vl = VirtualLibrary::new(name, query, color);
        self.repo.save(&vl)?;
        Ok(vl)
    }

    /// Met à jour une bibliothèque virtuelle existante.
    pub fn update(
        &self,
        id: &str,
        name: &str,
        query: &str,
        color: Option<String>,
    ) -> VellumResult<VirtualLibrary> {
        let existing = self
            .repo
            .find_by_id(id)?
            .ok_or_else(|| VellumError::NotFound(format!("Bibliothèque virtuelle {id}")))?;

        let name = name.trim();
        let query = query.trim();
        if name.is_empty() || query.is_empty() {
            return Err(VellumError::InvalidArgument(
                "Le nom et la requête ne peuvent pas être vides".to_string(),
            ));
        }

        let updated = VirtualLibrary {
            id: existing.id,
            name: name.to_string(),
            query: query.to_string(),
            color,
            created_at: existing.created_at,
        };
        self.repo.save(&updated)?;
        Ok(updated)
    }

    pub fn list(&self) -> VellumResult<Vec<VirtualLibrary>> {
        self.repo.find_all()
    }

    pub fn get(&self, id: &str) -> VellumResult<Option<VirtualLibrary>> {
        self.repo.find_by_id(id)
    }

    pub fn delete(&self, id: &str) -> VellumResult<bool> {
        self.repo.delete(id)
    }

    /// Filtre une liste de livres selon la requête d'une bibliothèque virtuelle donnée.
    pub fn filter_books(&self, books: &[Book], id: &str) -> VellumResult<Vec<Book>> {
        let vl = self
            .repo
            .find_by_id(id)?
            .ok_or_else(|| VellumError::NotFound(format!("Bibliothèque virtuelle {id}")))?;
        Ok(Self::filter_with_query(books, &vl.query))
    }

    /// Filtre une liste de livres à partir d'une requête Calibre-like brute.
    pub fn filter_with_query(books: &[Book], query_str: &str) -> Vec<Book> {
        let parsed = SearchQuery::parse(query_str);
        books
            .iter()
            .filter(|book| parsed.matches(book))
            .cloned()
            .collect()
    }
}
