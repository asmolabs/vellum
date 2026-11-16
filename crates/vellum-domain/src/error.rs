use thiserror::Error;

#[derive(Debug, Error)]
pub enum VellumError {
    #[error("Entité introuvable : {0}")]
    NotFound(String),

    #[error("Argument invalide : {0}")]
    InvalidArgument(String),

    #[error("État invalide : {0}")]
    InvalidState(String),

    #[error("Format ou opération non supporté : {0}")]
    Unsupported(String),

    #[error("Erreur d'entrée/sortie : {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur ZIP : {0}")]
    Zip(String),

    #[error("Erreur de sérialisation JSON : {0}")]
    Json(#[from] serde_json::Error),

    #[error("Erreur base de données : {0}")]
    Database(String),

    #[error("Erreur d'exécution de plugin : {0}")]
    Plugin(String),

    #[error("Commande externe : {0}")]
    ExternalCommand(String),

    #[error("Erreur système : {0}")]
    System(String),
}

pub type VellumResult<T> = Result<T, VellumError>;
