//! Backend de synchronisation cloud utilisant directement l'API REST Google Drive v3.

use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::RwLock;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, LOCATION};
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;
use vellum_domain::{SyncDelta, VellumError, VellumResult};

use super::folder_backend::CloudSyncBackend;
use super::gdrive_oauth::{GoogleDriveTokens, TokenManager};

pub const DEFAULT_DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";
pub const DEFAULT_DRIVE_UPLOAD_BASE: &str = "https://www.googleapis.com/upload/drive/v3";
const FOLDER_MIME_TYPE: &str = "application/vnd.google-apps.folder";

#[derive(Debug, Deserialize)]
struct DriveFileList {
    #[serde(default)]
    files: Vec<DriveFileRef>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DriveFileRef {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct CreateFolderBody<'a> {
    name: &'a str,
    #[serde(rename = "mimeType")]
    mime_type: &'a str,
    parents: [&'a str; 1],
}

#[derive(Debug, Serialize)]
struct UploadMetadataBody<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    parents: Option<[&'a str; 1]>,
}

/// Backend de synchronisation directe via Google Drive REST API v3 (OAuth 2.0 PKCE).
pub struct GoogleDriveApiBackend {
    token_manager: TokenManager,
    root_folder_name: String,
    api_base_url: String,
    upload_base_url: String,
    folder_cache: RwLock<HashMap<String, String>>,
}

impl GoogleDriveApiBackend {
    pub fn new(
        client_id: String,
        client_secret: Option<String>,
        root_folder_name: String,
        tokens_path: PathBuf,
    ) -> VellumResult<Self> {
        let folder_name = if root_folder_name.trim().is_empty() {
            "Vellum".to_string()
        } else {
            root_folder_name.trim().to_string()
        };

        Ok(Self {
            token_manager: TokenManager::new(client_id, client_secret, tokens_path),
            root_folder_name: folder_name,
            api_base_url: DEFAULT_DRIVE_API_BASE.to_string(),
            upload_base_url: DEFAULT_DRIVE_UPLOAD_BASE.to_string(),
            folder_cache: RwLock::new(HashMap::new()),
        })
    }

    fn http_client(&self) -> VellumResult<Client> {
        Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| VellumError::System(format!("Erreur client HTTP Google Drive: {e}")))
    }

    pub fn with_endpoints(
        mut self,
        api_base: String,
        upload_base: String,
        token_endpoint: String,
    ) -> Self {
        self.api_base_url = api_base.trim_end_matches('/').to_string();
        self.upload_base_url = upload_base.trim_end_matches('/').to_string();
        self.token_manager = self.token_manager.with_token_endpoint(token_endpoint);
        self
    }

    pub fn with_initial_tokens(mut self, tokens: GoogleDriveTokens) -> Self {
        self.token_manager = self.token_manager.with_initial_tokens(tokens);
        self
    }

    fn access_token(&self, http: &Client) -> VellumResult<String> {
        self.token_manager.valid_access_token(http)
    }

    fn escape_query(value: &str) -> String {
        value.replace('\\', "\\\\").replace('\'', "\\'")
    }

    fn parse_rel_path(rel_path: &str) -> VellumResult<(Vec<String>, String)> {
        let path = Path::new(rel_path);
        let mut segments = Vec::new();
        for comp in path.components() {
            match comp {
                Component::Normal(os) => {
                    let s = os.to_str().ok_or_else(|| {
                        VellumError::InvalidArgument("Chemin relatif UTF-8 invalide.".to_string())
                    })?;
                    if !s.is_empty() {
                        segments.push(s.to_string());
                    }
                }
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(VellumError::InvalidArgument(format!(
                        "Chemin relatif de synchronisation interdit: {rel_path}"
                    )));
                }
            }
        }

        let filename = segments.pop().ok_or_else(|| {
            VellumError::InvalidArgument("Nom de fichier manquant dans rel_path.".to_string())
        })?;
        Ok((segments, filename))
    }

    fn find_child_by_name(
        &self,
        parent_id: &str,
        name: &str,
        folders_only: Option<bool>,
    ) -> VellumResult<Option<String>> {
        let http = self.http_client()?;
        let token = self.access_token(&http)?;
        let escaped_name = Self::escape_query(name);
        let escaped_parent = Self::escape_query(parent_id);

        let mut query = format!(
            "name = '{escaped_name}' and '{escaped_parent}' in parents and trashed = false"
        );
        match folders_only {
            Some(true) => query.push_str(&format!(" and mimeType = '{FOLDER_MIME_TYPE}'")),
            Some(false) => query.push_str(&format!(" and mimeType != '{FOLDER_MIME_TYPE}'")),
            None => {}
        }

        let url = format!("{}/files", self.api_base_url);
        let resp = http
            .get(&url)
            .bearer_auth(&token)
            .query(&[
                ("q", query.as_str()),
                ("fields", "files(id,name)"),
                ("pageSize", "1"),
                ("spaces", "drive"),
            ])
            .send()
            .map_err(|e| VellumError::System(format!("Erreur requête Google Drive: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(VellumError::System(format!(
                "Google Drive files.list a échoué ({status}): {text}"
            )));
        }

        let list: DriveFileList = resp
            .json()
            .map_err(|e| VellumError::System(format!("Réponse JSON Google Drive invalide: {e}")))?;
        Ok(list.files.into_iter().next().map(|f| f.id))
    }

    fn create_folder(&self, parent_id: &str, name: &str) -> VellumResult<String> {
        let http = self.http_client()?;
        let token = self.access_token(&http)?;
        let url = format!("{}/files?fields=id", self.api_base_url);
        let body = CreateFolderBody {
            name,
            mime_type: FOLDER_MIME_TYPE,
            parents: [parent_id],
        };

        let resp = http
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .map_err(|e| {
                VellumError::System(format!("Erreur création dossier Google Drive: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(VellumError::System(format!(
                "Impossible de créer le dossier '{name}' sur Google Drive ({status}): {text}"
            )));
        }

        #[derive(Deserialize)]
        struct CreatedId {
            id: String,
        }
        let created: CreatedId = resp.json().map_err(|e| {
            VellumError::System(format!("Réponse création dossier Drive invalide: {e}"))
        })?;
        Ok(created.id)
    }

    fn resolve_folder_chain(
        &self,
        segments: &[&str],
        create_missing: bool,
    ) -> VellumResult<Option<String>> {
        let mut full_chain = Vec::with_capacity(segments.len() + 1);
        full_chain.push(self.root_folder_name.as_str());
        full_chain.extend_from_slice(segments);

        let mut current_parent = "root".to_string();
        let mut cache_key = String::new();

        for seg in full_chain {
            if !cache_key.is_empty() {
                cache_key.push('/');
            }
            cache_key.push_str(seg);

            if let Ok(cache) = self.folder_cache.read() {
                if let Some(cached_id) = cache.get(&cache_key) {
                    current_parent = cached_id.clone();
                    continue;
                }
            }

            let found = self.find_child_by_name(&current_parent, seg, Some(true))?;
            let folder_id = match found {
                Some(id) => id,
                None if create_missing => {
                    debug!("Création du dossier Google Drive: {cache_key}");
                    self.create_folder(&current_parent, seg)?
                }
                None => return Ok(None),
            };

            if let Ok(mut cache) = self.folder_cache.write() {
                cache.insert(cache_key.clone(), folder_id.clone());
            }
            current_parent = folder_id;
        }

        Ok(Some(current_parent))
    }

    fn download_file_bytes(&self, file_id: &str) -> VellumResult<Vec<u8>> {
        let http = self.http_client()?;
        let token = self.access_token(&http)?;
        let url = format!("{}/files/{file_id}?alt=media", self.api_base_url);
        let resp =
            http.get(&url).bearer_auth(&token).send().map_err(|e| {
                VellumError::System(format!("Erreur téléchargement Google Drive: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(VellumError::System(format!(
                "Téléchargement Google Drive échoué ({status}): {text}"
            )));
        }

        let bytes = resp
            .bytes()
            .map_err(|e| VellumError::System(format!("Erreur lecture flux Google Drive: {e}")))?;
        Ok(bytes.to_vec())
    }

    fn upload_multipart_json(
        &self,
        parent_id: &str,
        filename: &str,
        json_bytes: &[u8],
    ) -> VellumResult<()> {
        let http = self.http_client()?;
        let token = self.access_token(&http)?;
        let boundary = format!("vellum_{}", Uuid::new_v4().simple());
        let meta = serde_json::to_string(&UploadMetadataBody {
            name: filename,
            parents: Some([parent_id]),
        })?;

        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
        body.extend_from_slice(meta.as_bytes());
        body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Type: application/json\r\n\r\n");
        body.extend_from_slice(json_bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());

        let url = format!("{}/files?uploadType=multipart", self.upload_base_url);
        let resp = http
            .post(&url)
            .bearer_auth(&token)
            .header(
                CONTENT_TYPE,
                format!("multipart/related; boundary={boundary}"),
            )
            .body(body)
            .send()
            .map_err(|e| VellumError::System(format!("Erreur upload delta Google Drive: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(VellumError::System(format!(
                "Upload delta Google Drive échoué ({status}): {text}"
            )));
        }

        Ok(())
    }

    fn upload_resumable_file(
        &self,
        parent_id: &str,
        existing_id: Option<&str>,
        filename: &str,
        local_path: &Path,
    ) -> VellumResult<()> {
        let http = self.http_client()?;
        let token = self.access_token(&http)?;
        let file_bytes = fs::read(local_path)?;
        let total_len = file_bytes.len();

        let init_resp = match existing_id {
            Some(id) => {
                let url = format!("{}/files/{id}?uploadType=resumable", self.upload_base_url);
                let meta = UploadMetadataBody {
                    name: filename,
                    parents: None,
                };
                http.patch(&url)
                    .bearer_auth(&token)
                    .header("X-Upload-Content-Length", total_len.to_string())
                    .json(&meta)
                    .send()
            }
            None => {
                let url = format!("{}/files?uploadType=resumable", self.upload_base_url);
                let meta = UploadMetadataBody {
                    name: filename,
                    parents: Some([parent_id]),
                };
                http.post(&url)
                    .bearer_auth(&token)
                    .header("X-Upload-Content-Length", total_len.to_string())
                    .json(&meta)
                    .send()
            }
        }
        .map_err(|e| VellumError::System(format!("Erreur init upload Google Drive: {e}")))?;

        if !init_resp.status().is_success() {
            let status = init_resp.status();
            let text = init_resp.text().unwrap_or_default();
            return Err(VellumError::System(format!(
                "Init upload Google Drive échoué ({status}): {text}"
            )));
        }

        let upload_uri = init_resp
            .headers()
            .get(LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                VellumError::System(
                    "En-tête Location manquant dans la session resumable Google Drive.".to_string(),
                )
            })?
            .to_string();

        let put_resp = http
            .put(&upload_uri)
            .header(CONTENT_LENGTH, total_len.to_string())
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(file_bytes)
            .send()
            .map_err(|e| {
                VellumError::System(format!("Erreur transfert fichier Google Drive: {e}"))
            })?;

        if !put_resp.status().is_success() {
            let status = put_resp.status();
            let text = put_resp.text().unwrap_or_default();
            return Err(VellumError::System(format!(
                "Transfert fichier Google Drive échoué ({status}): {text}"
            )));
        }

        Ok(())
    }
}

impl CloudSyncBackend for GoogleDriveApiBackend {
    fn name(&self) -> &str {
        "Google Drive API"
    }

    fn is_available(&self) -> bool {
        self.token_manager.has_credentials()
    }

    fn list_deltas(&self) -> VellumResult<Vec<String>> {
        let Some(deltas_folder_id) = self.resolve_folder_chain(&["sync", "deltas"], false)? else {
            return Ok(Vec::new());
        };

        let http = self.http_client()?;
        let token = self.access_token(&http)?;
        let escaped_parent = Self::escape_query(&deltas_folder_id);
        let query = format!(
            "'{escaped_parent}' in parents and trashed = false and mimeType != '{FOLDER_MIME_TYPE}'"
        );

        let mut list = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let url = format!("{}/files", self.api_base_url);
            let mut req = http.get(&url).bearer_auth(&token).query(&[
                ("q", query.as_str()),
                ("fields", "nextPageToken,files(id,name)"),
                ("pageSize", "1000"),
                ("spaces", "drive"),
            ]);
            if let Some(ref pt) = page_token {
                req = req.query(&[("pageToken", pt.as_str())]);
            }

            let resp = req.send().map_err(|e| {
                VellumError::System(format!("Erreur listage deltas Google Drive: {e}"))
            })?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().unwrap_or_default();
                return Err(VellumError::System(format!(
                    "Listage deltas Google Drive échoué ({status}): {text}"
                )));
            }

            let parsed: DriveFileList = resp.json().map_err(|e| {
                VellumError::System(format!("Réponse listage deltas invalide: {e}"))
            })?;

            for f in parsed.files {
                if f.name.to_ascii_lowercase().ends_with(".json") {
                    list.push(f.name);
                }
            }

            match parsed.next_page_token {
                Some(next) if !next.is_empty() => page_token = Some(next),
                _ => break,
            }
        }

        list.sort();
        Ok(list)
    }

    fn read_delta(&self, filename: &str) -> VellumResult<SyncDelta> {
        let Some(deltas_folder_id) = self.resolve_folder_chain(&["sync", "deltas"], false)? else {
            return Err(VellumError::NotFound(format!(
                "Dossier sync/deltas introuvable sur Google Drive pour '{filename}'"
            )));
        };

        let file_id = self
            .find_child_by_name(&deltas_folder_id, filename, Some(false))?
            .ok_or_else(|| {
                VellumError::NotFound(format!("Delta '{filename}' introuvable sur Google Drive."))
            })?;

        let bytes = self.download_file_bytes(&file_id)?;
        let delta: SyncDelta = serde_json::from_slice(&bytes)?;
        Ok(delta)
    }

    fn write_delta(&self, delta: &SyncDelta) -> VellumResult<String> {
        let deltas_folder_id = self
            .resolve_folder_chain(&["sync", "deltas"], true)?
            .ok_or_else(|| {
                VellumError::System(
                    "Impossible de résoudre sync/deltas sur Google Drive.".to_string(),
                )
            })?;

        let filename = format!(
            "{}_{}.json",
            delta.client_id,
            delta.created_at.replace([':', '-'], "").replace('.', "_")
        );
        let content = serde_json::to_vec_pretty(delta)?;
        self.upload_multipart_json(&deltas_folder_id, &filename, &content)?;
        Ok(filename)
    }

    fn upload_file(&self, local_path: &Path, rel_path: &str) -> VellumResult<()> {
        if !local_path.exists() {
            return Err(VellumError::NotFound(format!(
                "Fichier source introuvable: {}",
                local_path.display()
            )));
        }

        let (dir_segments, filename) = Self::parse_rel_path(rel_path)?;
        let mut chain: Vec<&str> = Vec::with_capacity(dir_segments.len() + 1);
        chain.push("files");
        for s in &dir_segments {
            chain.push(s.as_str());
        }

        let parent_id = self.resolve_folder_chain(&chain, true)?.ok_or_else(|| {
            VellumError::System(
                "Impossible de créer le dossier cible sur Google Drive.".to_string(),
            )
        })?;

        let existing_id = self.find_child_by_name(&parent_id, &filename, Some(false))?;
        self.upload_resumable_file(&parent_id, existing_id.as_deref(), &filename, local_path)
    }

    fn download_file(&self, rel_path: &str, local_path: &Path) -> VellumResult<bool> {
        let (dir_segments, filename) = Self::parse_rel_path(rel_path)?;
        let mut chain: Vec<&str> = Vec::with_capacity(dir_segments.len() + 1);
        chain.push("files");
        for s in &dir_segments {
            chain.push(s.as_str());
        }

        let Some(parent_id) = self.resolve_folder_chain(&chain, false)? else {
            return Ok(false);
        };

        let Some(file_id) = self.find_child_by_name(&parent_id, &filename, Some(false))? else {
            return Ok(false);
        };

        let bytes = self.download_file_bytes(&file_id)?;

        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = local_path.with_extension("tmp_download");
        fs::write(&temp_path, bytes)?;
        fs::rename(temp_path, local_path)?;

        Ok(true)
    }

    fn file_exists(&self, rel_path: &str) -> VellumResult<bool> {
        let (dir_segments, filename) = Self::parse_rel_path(rel_path)?;
        let mut chain: Vec<&str> = Vec::with_capacity(dir_segments.len() + 1);
        chain.push("files");
        for s in &dir_segments {
            chain.push(s.as_str());
        }

        let Some(parent_id) = self.resolve_folder_chain(&chain, false)? else {
            return Ok(false);
        };

        Ok(self
            .find_child_by_name(&parent_id, &filename, Some(false))?
            .is_some())
    }
}
