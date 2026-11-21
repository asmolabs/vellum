//! Abstraction multiplateforme pour les dialogues fichiers et l'ouverture d'URL (Desktop vs Mobile).

use std::path::PathBuf;

#[cfg(feature = "desktop")]
pub fn pick_files(title: &str, filters: &[(&str, &[&str])]) -> Option<Vec<PathBuf>> {
    let mut dlg = rfd::FileDialog::new().set_title(title);
    for (name, exts) in filters {
        dlg = dlg.add_filter(*name, exts);
    }
    dlg.pick_files()
}

#[cfg(not(feature = "desktop"))]
pub fn pick_files(_title: &str, _filters: &[(&str, &[&str])]) -> Option<Vec<PathBuf>> {
    None
}

#[cfg(feature = "desktop")]
pub fn pick_file(title: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf> {
    let mut dlg = rfd::FileDialog::new().set_title(title);
    for (name, exts) in filters {
        dlg = dlg.add_filter(*name, exts);
    }
    dlg.pick_file()
}

#[cfg(not(feature = "desktop"))]
pub fn pick_file(_title: &str, _filters: &[(&str, &[&str])]) -> Option<PathBuf> {
    None
}

#[cfg(feature = "desktop")]
pub fn pick_folder(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_folder()
}

#[cfg(not(feature = "desktop"))]
pub fn pick_folder(_title: &str) -> Option<PathBuf> {
    None
}

#[cfg(feature = "desktop")]
pub fn save_file(title: &str, default_name: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf> {
    let mut dlg = rfd::FileDialog::new()
        .set_title(title)
        .set_file_name(default_name);
    for (name, exts) in filters {
        dlg = dlg.add_filter(*name, exts);
    }
    dlg.save_file()
}

#[cfg(not(feature = "desktop"))]
pub fn save_file(
    _title: &str,
    _default_name: &str,
    _filters: &[(&str, &[&str])],
) -> Option<PathBuf> {
    None
}

#[cfg(feature = "desktop")]
#[allow(dead_code)]
pub async fn pick_folder_async(title: &str) -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title(title)
        .pick_folder()
        .await
        .map(|h| h.path().to_path_buf())
}

#[cfg(not(feature = "desktop"))]
#[allow(dead_code)]
pub async fn pick_folder_async(_title: &str) -> Option<PathBuf> {
    None
}

#[cfg(feature = "desktop")]
#[allow(dead_code)]
pub async fn pick_file_async(title: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf> {
    let mut dlg = rfd::AsyncFileDialog::new().set_title(title);
    for (name, exts) in filters {
        dlg = dlg.add_filter(*name, exts);
    }
    dlg.pick_file().await.map(|h| h.path().to_path_buf())
}

#[cfg(not(feature = "desktop"))]
#[allow(dead_code)]
pub async fn pick_file_async(_title: &str, _filters: &[(&str, &[&str])]) -> Option<PathBuf> {
    None
}

#[cfg(all(feature = "desktop", target_os = "macos"))]
pub async fn pick_calibre_path_async(title: &str) -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title(title)
        .pick_file_or_folder()
        .await
        .map(|h| h.path().to_path_buf())
}

#[cfg(all(feature = "desktop", not(target_os = "macos")))]
pub async fn pick_calibre_path_async(title: &str) -> Option<PathBuf> {
    if let Some(folder) = pick_folder_async(title).await {
        return Some(folder);
    }
    pick_file_async(title, &[("Base de données Calibre", &["db"])]).await
}

#[cfg(not(feature = "desktop"))]
pub async fn pick_calibre_path_async(_title: &str) -> Option<PathBuf> {
    None
}

#[cfg(feature = "desktop")]
pub fn open_url(url: &str) {
    let _ = open::that(url);
}

#[cfg(not(feature = "desktop"))]
pub fn open_url(_url: &str) {}
