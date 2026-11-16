use lettre::message::{header::ContentType, Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::path::{Path, PathBuf};
use vellum_domain::{Book, SmtpConfig, VellumError, VellumResult};

#[derive(Clone)]
pub struct MailService {
    library_path: PathBuf,
}

impl MailService {
    pub fn new(library_path: PathBuf) -> Self {
        Self { library_path }
    }

    /// Test SMTP connection by establishing TLS handshake and authenticating
    pub async fn test_connection(&self, config: &SmtpConfig) -> VellumResult<()> {
        if config.host.trim().is_empty() {
            return Err(VellumError::InvalidArgument(
                "L'hôte SMTP ne peut pas être vide".into(),
            ));
        }
        if config.username.trim().is_empty() {
            return Err(VellumError::InvalidArgument(
                "Le nom d'utilisateur SMTP ne peut pas être vide".into(),
            ));
        }

        let creds = Credentials::new(config.username.clone(), config.password.clone());

        let mailer: AsyncSmtpTransport<Tokio1Executor> = if config.port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
                .map_err(|e| VellumError::System(format!("Erreur relais SMTP: {e}")))?
                .credentials(creds)
                .port(config.port)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                .map_err(|e| VellumError::System(format!("Erreur STARTTLS SMTP: {e}")))?
                .credentials(creds)
                .port(config.port)
                .build()
        };

        mailer
            .test_connection()
            .await
            .map_err(|e| VellumError::System(format!("Échec du test de connexion SMTP: {e}")))?;

        Ok(())
    }

    /// Send a book to a Kindle email address as an attachment
    pub async fn send_to_kindle(
        &self,
        book: &Book,
        config: &SmtpConfig,
        preferred_format: Option<&str>,
    ) -> VellumResult<()> {
        if !config.enabled {
            return Err(VellumError::InvalidArgument(
                "Le service d'envoi par e-mail n'est pas activé dans les Paramètres.".into(),
            ));
        }
        if config.from_address.trim().is_empty() {
            return Err(VellumError::InvalidArgument(
                "L'adresse expéditeur (From) n'est pas renseignée.".into(),
            ));
        }
        if config.kindle_email.trim().is_empty() {
            return Err(VellumError::InvalidArgument(
                "L'adresse e-mail Kindle du destinataire n'est pas renseignée.".into(),
            ));
        }

        // 1. Pick format: preferred > epub > pdf > mobi > first available
        let chosen_fmt = if let Some(pref) = preferred_format {
            book.formats
                .iter()
                .find(|f| f.format.eq_ignore_ascii_case(pref))
        } else {
            book.formats
                .iter()
                .find(|f| f.format.eq_ignore_ascii_case("epub"))
                .or_else(|| {
                    book.formats
                        .iter()
                        .find(|f| f.format.eq_ignore_ascii_case("pdf"))
                })
                .or_else(|| {
                    book.formats
                        .iter()
                        .find(|f| f.format.eq_ignore_ascii_case("mobi"))
                })
                .or_else(|| book.formats.first())
        };

        let format_entry = chosen_fmt.ok_or_else(|| {
            VellumError::NotFound(format!(
                "Aucun format de fichier disponible pour le livre '{}'",
                book.title
            ))
        })?;

        // 2. Locate file on disk
        let file_path = if Path::new(&format_entry.filepath).is_absolute() {
            PathBuf::from(&format_entry.filepath)
        } else {
            self.library_path.join(&format_entry.filepath)
        };

        if !file_path.exists() {
            return Err(VellumError::NotFound(format!(
                "Fichier introuvable sur le disque : {:?}",
                file_path
            )));
        }

        let file_bytes = std::fs::read(&file_path)?;

        // 3. Prepare MIME Content Type and clean filename
        let ext = format_entry.format.to_lowercase();
        let content_type_str = match ext.as_str() {
            "epub" => "application/epub+zip",
            "pdf" => "application/pdf",
            "mobi" => "application/x-mobipocket-ebook",
            _ => "application/octet-stream",
        };
        let content_type = ContentType::parse(content_type_str).map_err(|e| {
            VellumError::InvalidArgument(format!("Type MIME invalide ({content_type_str}): {e}"))
        })?;

        let safe_author = book
            .author
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let safe_title = book
            .title
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let attachment_filename = format!("{safe_author} - {safe_title}.{ext}");

        let attachment = Attachment::new(attachment_filename).body(file_bytes, content_type);

        // 4. Build email message
        let from_mailbox = config
            .from_address
            .parse()
            .map_err(|e| VellumError::InvalidArgument(format!("Adresse From invalide : {e}")))?;
        let to_mailbox = config
            .kindle_email
            .parse()
            .map_err(|e| VellumError::InvalidArgument(format!("Adresse Kindle invalide : {e}")))?;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(format!("Book: {}", book.title))
            .multipart(
                MultiPart::mixed()
                    .singlepart(SinglePart::plain(format!(
                        "Livre '{}' par {} envoyé depuis Vellum.",
                        book.title, book.author
                    )))
                    .singlepart(attachment),
            )
            .map_err(|e| VellumError::System(format!("Échec de construction de l'e-mail : {e}")))?;

        // 5. Connect and send via SMTP
        let creds = Credentials::new(config.username.clone(), config.password.clone());

        let mailer: AsyncSmtpTransport<Tokio1Executor> = if config.port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
                .map_err(|e| {
                    VellumError::System(format!("Erreur configuration relais SMTP : {e}"))
                })?
                .credentials(creds)
                .port(config.port)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                .map_err(|e| VellumError::System(format!("Erreur configuration STARTTLS : {e}")))?
                .credentials(creds)
                .port(config.port)
                .build()
        };

        mailer
            .send(email)
            .await
            .map_err(|e| VellumError::System(format!("Échec de l'envoi vers Kindle : {e}")))?;

        tracing::info!(
            "Livre '{}' envoyé avec succès à la Kindle ({}) via SMTP",
            book.title,
            config.kindle_email
        );

        Ok(())
    }
}
