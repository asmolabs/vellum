use std::path::PathBuf;
use std::sync::Arc;

use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use tracing::info;

mod components;
mod state;
mod styles;

use components::{
    BookCard, DetailDrawer, Header, ReaderModal, SettingsModal, Sidebar, TranslateModal,
    TranslationRequest,
};
use state::{FilterStatus, ReaderSession, SortOption};
use styles::APP_CSS;

use vellum_core::book_service::BookService;
use vellum_core::conversion_service::ConversionService;
use vellum_core::device_service::DeviceService;

use vellum_core::litestream_service::LitestreamService;
use vellum_core::metadata_service::MetadataService;
use vellum_core::opds_server::OpdsServer;
use vellum_core::plugin_service::PluginService;
use vellum_domain::{AppConfig, Book, DeviceKind};
use vellum_storage::{BookRepository, Database, ParamRepository, TagRepository};

#[derive(Clone)]
struct AppServices {
    book_service: Arc<BookService>,
    device_service: Arc<DeviceService>,
    litestream_service: Arc<LitestreamService>,
    opds_port: u16,
}


fn init_services() -> AppServices {
    let app_cfg = AppConfig::default();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let vellum_dir = home.join(".vellum");
    let _ = std::fs::create_dir_all(&vellum_dir);
    let _ = std::fs::create_dir_all(&app_cfg.library_path);
    let _ = std::fs::create_dir_all(&app_cfg.plugins_path);

    let db = Database::open(&app_cfg.database_path).expect("open vellum sqlite database");
    let book_repo = BookRepository::new(db.clone());
    let param_repo = ParamRepository::new(db.clone());
    let tag_repo = TagRepository::new(db.clone());

    let plugin_service = PluginService::new(param_repo.clone(), Some(app_cfg.plugins_path.clone()));
    let metadata_service = MetadataService::new(plugin_service.clone());
    let conversion_service = ConversionService::new(plugin_service.clone());

    let book_service = Arc::new(BookService::new(
        book_repo,
        param_repo,
        tag_repo,
        metadata_service,
        plugin_service,
        conversion_service,
        app_cfg.library_path.clone(),
    ));

    scan_and_index_library(&book_service);

    let litestream_service = Arc::new(LitestreamService::new(
        app_cfg.litestream.clone(),
        app_cfg.database_path.clone(),
    ));

    // Spawn OPDS Server in background
    let opds_bs = book_service.clone();
    let opds_port = app_cfg.opds_port;
    tokio::spawn(async move {
        info!("Starting OPDS server on port {}", opds_port);
        let _ = OpdsServer::run_server((*opds_bs).clone(), opds_port).await;
    });

    let device_service = Arc::new(DeviceService::new(
        book_service.clone(),
        book_service.plugin_service().clone(),
    ));

    AppServices {
        book_service,
        device_service,
        litestream_service,
        opds_port,
    }
}


fn scan_and_index_library(book_service: &BookService) {
    if let Ok(count) = book_service.book_repo().count() {
        if count == 0 {
            let library_path = book_service.library_path();
            if let Ok(authors) = std::fs::read_dir(library_path) {
                for author_res in authors.flatten() {
                    let author_path = author_res.path();
                    if author_path.is_dir() {
                        if let Ok(books_dir) = std::fs::read_dir(&author_path) {
                            for book_res in books_dir.flatten() {
                                let book_path = book_res.path();
                                if book_path.is_dir() {
                                    if let Ok(files) = std::fs::read_dir(&book_path) {
                                        for file_res in files.flatten() {
                                            let file_p = file_res.path();
                                            let ext = file_p
                                                .extension()
                                                .and_then(|e| e.to_str())
                                                .unwrap_or("")
                                                .to_lowercase();
                                            if matches!(
                                                ext.as_str(),
                                                "epub" | "pdf" | "cbz" | "cbr" | "mobi" | "azw3"
                                            ) {
                                                let meta = book_service
                                                    .metadata_service()
                                                    .extract_metadata(&file_p);
                                                let title = meta.title.unwrap_or_else(|| {
                                                    file_p
                                                        .file_stem()
                                                        .and_then(|s| s.to_str())
                                                        .unwrap_or("Sans titre")
                                                        .to_string()
                                                });
                                                let author = meta.author.unwrap_or_else(|| {
                                                    author_path
                                                        .file_name()
                                                        .and_then(|s| s.to_str())
                                                        .unwrap_or("Auteur inconnu")
                                                        .to_string()
                                                });

                                                let file_rel = file_p
                                                    .strip_prefix(library_path)
                                                    .unwrap_or(&file_p)
                                                    .to_string_lossy()
                                                    .to_string();

                                                let cover_path = if book_path.join("cover.jpg").exists() {
                                                    Some(
                                                        book_path
                                                            .join("cover.jpg")
                                                            .strip_prefix(library_path)
                                                            .unwrap_or(&book_path.join("cover.jpg"))
                                                            .to_string_lossy()
                                                            .to_string(),
                                                    )
                                                } else if book_path.join("cover.jpeg").exists() {
                                                    Some(
                                                        book_path
                                                            .join("cover.jpeg")
                                                            .strip_prefix(library_path)
                                                            .unwrap_or(&book_path.join("cover.jpeg"))
                                                            .to_string_lossy()
                                                            .to_string(),
                                                    )
                                                } else {
                                                    None
                                                };

                                                let file_size = std::fs::metadata(&file_p)
                                                    .map(|m| m.len())
                                                    .unwrap_or(0);
                                                let mut book = Book::new(title, author);
                                                book.description = meta.description;
                                                book.publisher = meta.publisher;
                                                book.publish_date = meta.publish_date;
                                                book.cover_path = cover_path;
                                                for t in meta.tags {
                                                    let tag = vellum_domain::Tag::new(t);
                                                    let _ = book_service.tag_repo().save(&tag);
                                                    book.add_tag(tag);
                                                }
                                                let fmt = vellum_domain::BookFormat::new(
                                                    ext,
                                                    file_p
                                                        .file_name()
                                                        .and_then(|n| n.to_str())
                                                        .unwrap_or("book")
                                                        .to_string(),
                                                    file_rel,
                                                    file_size,
                                                );
                                                book.add_format(fmt);
                                                let _ = book_service.book_repo().save(&book);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let cfg = Config::new().with_window(
        WindowBuilder::new()
            .with_title("Vellum — Bibliothèque Personnelle & Liseuse")
            .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
            .with_min_inner_size(dioxus::desktop::LogicalSize::new(960.0, 640.0)),
    );

    dioxus::LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}

#[component]
fn App() -> Element {
    let services = use_hook(init_services);
    let book_service = services.book_service.clone();
    let litestream = services.litestream_service.clone();
    let opds_port = services.opds_port;
    let device_service = services.device_service.clone();
    let mut connected_devices = use_signal(|| device_service.detect_connected_devices());


    let mut books = use_signal(|| {
        book_service
            .book_repo()
            .find_all()
            .unwrap_or_default()
    });


    let current_filter = use_signal(|| FilterStatus::All);
    let mut current_sort = use_signal(|| SortOption::RecentAdded);
    let selected_tag = use_signal(|| None::<String>);
    let search_query = use_signal(String::new);
    let mut selected_book = use_signal(|| None::<Book>);
    let mut active_reader = use_signal(|| None::<ReaderSession>);
    let mut settings_open = use_signal(|| false);
    let mut translating_book = use_signal(|| None::<Book>);
    let mut is_translating = use_signal(|| false);

    // Refresh function helper
    let refresh_library = {
        let bs = book_service.clone();
        move || {
            if let Ok(all) = bs.book_repo().find_all() {
                books.set(all);
            }
        }
    };


    // Calculate tag counts
    let tags_counts: Vec<(String, usize)> = {
        let all_books = books.read();
        let mut map = std::collections::HashMap::new();
        for b in all_books.iter() {
            for t in &b.tags {
                *map.entry(t.name.clone()).or_insert(0) += 1;
            }
        }
        let mut list: Vec<_> = map.into_iter().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list
    };

    // Reading stats
    let reading_stats = book_service.get_reading_stats().ok();

    // Filter and sort books
    let filtered_books: Vec<Book> = {
        let all = books.read();
        let filter = *current_filter.read();
        let sort = *current_sort.read();
        let tag_filter = selected_tag.read().clone();
        let query = search_query.read().trim().to_lowercase();

        let mut list: Vec<Book> = all
            .iter()
            .filter(|b| {
                // Status Filter
                let matches_status = match filter {
                    FilterStatus::All => true,
                    FilterStatus::Reading => b.progress_percent > 0.0 && !b.is_finished,
                    FilterStatus::Finished => b.is_finished,
                    FilterStatus::ToRead => b.progress_percent == 0.0 && !b.is_finished,
                };
                if !matches_status {
                    return false;
                }

                // Tag Filter
                if let Some(ref t) = tag_filter {
                    if !b.tags.iter().any(|bt| &bt.name == t) {
                        return false;
                    }
                }

                // Query Search
                if !query.is_empty() {
                    let in_title = b.title.to_lowercase().contains(&query);
                    let in_author = b.author.to_lowercase().contains(&query);
                    let in_tags = b.tags.iter().any(|t| t.name.to_lowercase().contains(&query));
                    if !in_title && !in_author && !in_tags {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        match sort {
            SortOption::TitleAsc => list.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
            SortOption::TitleDesc => list.sort_by(|a, b| b.title.to_lowercase().cmp(&a.title.to_lowercase())),
            SortOption::AuthorAsc => list.sort_by(|a, b| a.author.to_lowercase().cmp(&b.author.to_lowercase()).then_with(|| a.title.cmp(&b.title))),
            SortOption::RecentAdded => list.sort_by(|a, b| b.added_at.cmp(&a.added_at)),
            SortOption::ProgressDesc => list.sort_by(|a, b| b.progress_percent.partial_cmp(&a.progress_percent).unwrap_or(std::cmp::Ordering::Equal)),
        }

        list
    };

    // Handle book file import via native file picker
    let handle_import = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |()| {
            let files = rfd::FileDialog::new()
                .set_title("Importer des livres ou bandes dessinées dans Vellum")
                .add_filter(
                    "Fichiers pris en charge",
                    &["epub", "pdf", "cbz", "cbr", "mobi", "azw3", "txt", "mp3", "m4b"],
                )
                .pick_files();

            if let Some(paths) = files {
                for path in paths {
                    tracing::info!("Importing file: {:?}", path);
                    match bs.import_book(&path) {
                        Ok(book) => {
                            tracing::info!("Livre importé avec succès: {} par {}", book.title, book.author);
                        }
                        Err(e) => {
                            tracing::error!("Erreur lors de l'import de {:?}: {}", path, e);
                        }
                    }
                }
                refresh();
            }
        }
    };

    // Handle read book launcher
    let handle_read = {
        let bs = book_service.clone();
        move |book: Book| {
            if let Some(first_fmt) = book.formats.first() {
                let abs_path = bs.library_path().join(&first_fmt.filepath);
                if abs_path.exists() {
                    let session = ReaderSession::new(book, abs_path);
                    active_reader.set(Some(session));
                    selected_book.set(None);
                }
            }
        }
    };

    // Handle progress update from reader
    let handle_progress = {
        let bs = book_service.clone();
        move |(page, total, pct): (usize, usize, f64)| {
            if let Some(ref sess) = *active_reader.read() {
                let book_id = &sess.book.id;
                let finished = page + 1 >= total && total > 0;
                let _ = bs.update_reading_progress(
                    book_id,
                    page as i32,
                    total as i32,
                    pct,
                    finished,
                );
            }
        }
    };

    // Handle book delete
    let handle_delete = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |id: String| {
            let _ = bs.delete_book(&id, true);
            selected_book.set(None);
            refresh();
        }
    };

    // Handle book format conversion via pivot pipeline
    let handle_convert = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |(b_id, target_fmt): (String, String)| {
            match bs.convert_book_to_format(&b_id, &target_fmt) {
                Ok(updated) => {
                    tracing::info!("Conversion réussie vers le format {target_fmt}");
                    selected_book.set(Some(updated));
                    refresh();
                }
                Err(e) => {
                    tracing::error!("Erreur lors de la conversion en {target_fmt}: {e}");
                }
            }
        }
    };

    // Handle book metadata save
    let handle_save_metadata = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |(b_id, title, author, publ, desc, tags): (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Vec<String>,
        )| {
            match bs.update_book_metadata(
                &b_id,
                &title,
                &author,
                publ.as_deref(),
                desc.as_deref(),
                tags,
            ) {
                Ok(updated) => {
                    tracing::info!("Métadonnées mises à jour avec succès pour: {}", updated.title);
                    selected_book.set(Some(updated));
                    refresh();
                }
                Err(e) => {
                    tracing::error!("Erreur lors de la mise à jour des métadonnées: {e}");
                }
            }
        }
    };

    // Handle cover replacement
    let handle_change_cover = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |(b_id, path): (String, PathBuf)| {
            match bs.update_book_cover(&b_id, &path) {
                Ok(updated) => {
                    tracing::info!("Couverture mise à jour avec succès pour: {}", updated.title);
                    selected_book.set(Some(updated));
                    refresh();
                }
                Err(e) => {
                    tracing::error!("Erreur lors du changement de couverture: {e}");
                }
            }
        }
    };

    // Handle AI translation launch
    let handle_start_translate = {
        let bs = book_service.clone();
        let refresh = refresh_library.clone();
        move |req: TranslationRequest| {
            is_translating.set(true);
            let bs_cloned = bs.clone();
            let mut refresh_cloned = refresh.clone();
            dioxus::prelude::spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    bs_cloned.translate_book(
                        &req.book_id,
                        &req.target_lang,
                        &req.provider,
                        &req.model,
                        req.base_url.as_deref(),
                        req.api_key.as_deref(),
                        req.bilingual_mode,
                    )
                })
                .await;

                is_translating.set(false);
                match res {
                    Ok(Ok(translated_book)) => {
                        tracing::info!("Livre traduit avec succès: {}", translated_book.title);
                        translating_book.set(None);
                        selected_book.set(Some(translated_book));
                        refresh_cloned();
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Erreur lors de la traduction: {e}");
                    }
                    Err(e) => {
                        tracing::error!("Erreur de tâche en arrière-plan: {e}");
                    }
                }
            });
        }
    };


    // Handle eReader export
    let handle_export_to_device = {
        let ds = device_service.clone();
        move |(b_id, kind): (String, DeviceKind)| {
            match ds.export_book_to_device(&b_id, kind, None) {
                Ok(dest) => {
                    tracing::info!("Livre exporté avec succès vers {:?}: {:?}", kind, dest);
                }
                Err(e) => {
                    tracing::error!("Erreur lors de l'exportation vers {:?}: {e}", kind);
                }
            }
        }
    };

    // Handle eReader sync (import new books & sync reading progress)
    let handle_sync_device = {
        let ds = device_service.clone();
        let mut refresh = refresh_library.clone();
        move |kind: DeviceKind| {
            let _ = ds.sync_reading_progress(kind, None);
            let _ = ds.import_books_from_device(kind, None);
            refresh();
            connected_devices.set(ds.detect_connected_devices());
        }
    };

    let litestream_status_str = match litestream.status() {
        vellum_core::litestream_service::LitestreamStatus::Running => "En cours de synchronisation",
        vellum_core::litestream_service::LitestreamStatus::Stopped => "Arrêté (en attente)",
        vellum_core::litestream_service::LitestreamStatus::Disabled => "Désactivé",
        vellum_core::litestream_service::LitestreamStatus::BinaryNotFound => "Exécutable introuvable",
        vellum_core::litestream_service::LitestreamStatus::Error => "Erreur de processus",
    }
    .to_string();

    let plugin_descriptors = book_service.plugin_service().list_descriptors();

    rsx! {
        style { "{APP_CSS}" }

        div { class: "app-container",
            // Header
            Header {
                search_query: search_query,
                connected_devices: connected_devices.read().clone(),
                on_import: handle_import.clone(),
                on_sync_device: handle_sync_device.clone(),
                on_open_settings: move |_| settings_open.set(true),
            }

            // Main Layout
            div { class: "main-content",
                // Sidebar
                Sidebar {
                    current_filter: current_filter,
                    selected_tag: selected_tag,
                    tags_counts: tags_counts,

                    stats: reading_stats,
                }

                // Books Library
                main { class: "library-area",
                    div { class: "library-header",
                        div { class: "library-title-group",
                            h2 { "{current_filter.read().label()}" }
                            div { class: "library-count",
                                "{filtered_books.len()} livre(s) affiché(s)"
                            }
                        }

                        div { style: "display: flex; align-items: center; gap: 8px;",
                            span { style: "font-size: 12px; color: var(--text-muted); font-weight: 500;", "Trier par :" }
                            select {
                                style: "padding: 6px 12px; font-size: 12px; border-radius: 8px; background: rgba(255,255,255,0.05); color: var(--text-primary); border: 1px solid var(--border-subtle); outline: none; cursor: pointer;",
                                value: match *current_sort.read() {
                                    SortOption::RecentAdded => "recent",
                                    SortOption::TitleAsc => "title_asc",
                                    SortOption::TitleDesc => "title_desc",
                                    SortOption::AuthorAsc => "author_asc",
                                    SortOption::ProgressDesc => "progress",
                                },
                                onchange: move |evt| {
                                    let val = evt.value();
                                    let sort = match val.as_str() {
                                        "title_asc" => SortOption::TitleAsc,
                                        "title_desc" => SortOption::TitleDesc,
                                        "author_asc" => SortOption::AuthorAsc,
                                        "progress" => SortOption::ProgressDesc,
                                        _ => SortOption::RecentAdded,
                                    };
                                    current_sort.set(sort);
                                },
                                option { value: "recent", "Récemment ajoutés" }
                                option { value: "title_asc", "Titre (A → Z)" }
                                option { value: "title_desc", "Titre (Z → A)" }
                                option { value: "author_asc", "Auteur (A → Z)" }
                                option { value: "progress", "Progression de lecture" }
                            }
                        }
                    }

                    if filtered_books.is_empty() {
                        div {
                            style: "flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; color: var(--text-muted); padding: 60px 0;",
                            span { style: "font-size: 40px;", "📚" }
                            p { style: "font-size: 15px; font-weight: 500;", "Aucun livre trouvé dans cette vue" }
                            p { style: "font-size: 13px;", "Glissez-déposez des fichiers EPUB, PDF, CBZ ou cliquez sur 'Importer'." }
                            button {
                                class: "btn btn-primary",
                                style: "margin-top: 8px;",
                                onclick: {
                                    let mut h = handle_import.clone();
                                    move |_| h(())
                                },
                                "Ajouter un livre"
                            }
                        }
                    } else {
                        div { class: "books-grid",
                            for b in filtered_books.iter() {
                                BookCard {
                                    key: "{b.id}",
                                    book: b.clone(),
                                    on_select: move |selected| selected_book.set(Some(selected)),
                                    on_read: handle_read.clone(),
                                }
                            }
                        }
                    }
                }
            }

            // Detail Drawer
            if let Some(book) = selected_book.read().as_ref() {
                {
                    let convertible = book_service.get_convertible_formats_for_book(book);
                    rsx! {
                        DetailDrawer {
                            book: book.clone(),
                            convertible_formats: convertible,
                            connected_devices: connected_devices.read().clone(),
                            on_close: move |_| selected_book.set(None),
                            on_read: handle_read.clone(),
                            on_delete: handle_delete.clone(),
                            on_convert: handle_convert.clone(),
                            on_change_cover: handle_change_cover.clone(),
                            on_save_metadata: handle_save_metadata.clone(),
                            on_open_translate: move |b| translating_book.set(Some(b)),
                            on_export_to_device: handle_export_to_device.clone(),
                        }

                    }
                }
            }

            // Translate Modal
            if let Some(book) = translating_book.read().as_ref() {
                TranslateModal {
                    book: book.clone(),
                    is_translating: *is_translating.read(),
                    on_close: move |_| translating_book.set(None),
                    on_start_translate: handle_start_translate.clone(),
                }
            }

            // Fullscreen Reader
            if active_reader.read().is_some() {
                ReaderModal {
                    session: active_reader,
                    on_close: {
                        let mut refresh = refresh_library.clone();
                        move |_| {
                            active_reader.set(None);
                            refresh();
                        }
                    },
                    on_progress_update: handle_progress,
                }
            }

            // Settings Modal
            if *settings_open.read() {
                SettingsModal {
                    opds_port: opds_port,
                    opds_running: true,
                    litestream_status_text: litestream_status_str,
                    plugins: plugin_descriptors,
                    on_close: move |_| settings_open.set(false),
                }
            }
        }

    }
}
