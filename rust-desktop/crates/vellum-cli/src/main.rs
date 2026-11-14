//! Interface en ligne de commande (mode headless) pour Vellum.

use std::path::{Path, PathBuf};

use vellum_core::{
    init_services, BookService, CatalogExporter, CatalogFormat, OpdsServer, PolishOptions,
    WatchFolderService,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print_help();
        return;
    }

    let services = init_services();
    let bs = services.book_service.clone();

    if args[0] == "serve" {
        let port = args
            .windows(2)
            .find(|w| w[0] == "--port")
            .and_then(|w| w[1].parse::<u16>().ok())
            .unwrap_or(services.opds_port);
        println!("Serveur OPDS / Web Companion démarré sur http://0.0.0.0:{port}");
        if let Err(e) = OpdsServer::run_server((*bs).clone(), port).await {
            exit_err(&format!("Erreur serveur : {e}"));
        }
        return;
    }

    match dispatch_sync_command(&args, &bs) {
        Ok(out) => println!("{out}"),
        Err(err) => {
            if err.starts_with("Commande inconnue") {
                eprintln!("{err}");
                print_help();
                std::process::exit(1);
            }
            exit_err(&err);
        }
    }
}

fn dispatch_sync_command(args: &[String], bs: &BookService) -> Result<String, String> {
    let Some(cmd) = args.first().map(String::as_str) else {
        return Err("Aucune commande fournie".to_string());
    };

    match cmd {
        "list" => {
            let mut search = None;
            let mut tag = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--search" if i + 1 < args.len() => {
                        search = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--tag" if i + 1 < args.len() => {
                        tag = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let books = bs
                .get_books(search, tag)
                .map_err(|e| format!("Erreur list : {e}"))?;
            let mut lines = vec![format!("{} livre(s) :", books.len())];
            for b in books {
                let fmts: Vec<&str> = b.formats.iter().map(|f| f.format.as_str()).collect();
                lines.push(format!("- [{}] {} — {} ({})", b.id, b.title, b.author, fmts.join(", ")));
            }
            Ok(lines.join("\n"))
        }
        "import" => {
            let path = args
                .get(1)
                .ok_or_else(|| "Usage: vellum-cli import <fichier>".to_string())?;
            let book = bs
                .import_book(Path::new(path))
                .map_err(|e| format!("Erreur import : {e}"))?;
            Ok(format!("Importé : [{}] {} — {}", book.id, book.title, book.author))
        }
        "polish" => {
            let book_id = args
                .get(1)
                .ok_or_else(|| "Usage: vellum-cli polish <book_id>".to_string())?;
            let r = bs
                .polish_book(book_id, PolishOptions::default())
                .map_err(|e| format!("Erreur polish : {e}"))?;
            Ok(format!(
                "Polish terminé : metadata={}, cover={}, chapitres={}, toc={}, repaired={}",
                r.metadata_updated, r.cover_embedded, r.chapters_smartened, r.toc_rebuilt, r.repaired
            ))
        }
        "convert" => {
            if args.len() < 3 {
                return Err(
                    "Usage: vellum-cli convert <epub|pdf|cbz|txt> [--all | <book_id>...]"
                        .to_string(),
                );
            }
            let fmt = &args[1];
            let ids: Vec<String> = if args[2] == "--all" {
                bs.get_books(None, None)
                    .map_err(|e| format!("Erreur lecture catalogue : {e}"))?
                    .into_iter()
                    .map(|b| b.id)
                    .collect()
            } else {
                args[2..].to_vec()
            };
            let report = bs.bulk_convert_books(&ids, fmt);
            let mut out = format!(
                "Conversion batch -> {} : {} converti(s), {} ignoré(s), {} erreur(s)",
                report.target_format,
                report.converted_count,
                report.skipped_count,
                report.errors.len()
            );
            for err in report.errors {
                out.push_str(&format!("\n  - {err}"));
            }
            Ok(out)
        }
        "audit" => {
            let report = bs
                .audit_library()
                .map_err(|e| format!("Erreur audit : {e}"))?;
            Ok(format!(
                "Audit (score {}%) : {} livres, {} doublons potentiels, {} fiches incomplètes, {} fichiers manquants",
                report.overall_score,
                report.total_books,
                report.duplicate_pairs.len(),
                report.incomplete_issues.len(),
                report.missing_file_issues.len()
            ))
        }
        "export" => {
            if args.len() < 3 {
                return Err("Usage: vellum-cli export <csv|json|bibtex> <fichier_sortie>".to_string());
            }
            let fmt = match args[1].to_lowercase().as_str() {
                "csv" => CatalogFormat::Csv,
                "json" => CatalogFormat::Json,
                "bib" | "bibtex" => CatalogFormat::Bibtex,
                other => {
                    return Err(format!(
                        "Format inconnu : {other} (attendu: csv, json, bibtex)"
                    ));
                }
            };
            let books = bs.get_books(None, None).unwrap_or_default();
            CatalogExporter::export_to_file(&books, fmt, Path::new(&args[2]))
                .map_err(|e| format!("Erreur export : {e}"))?;
            Ok(format!("Catalogue exporté ({} livres) -> {}", books.len(), args[2]))
        }
        "watch" => {
            let dir = args
                .get(1)
                .ok_or_else(|| "Usage: vellum-cli watch <dossier> [--keep]".to_string())?;
            let move_processed = !args.iter().any(|a| a == "--keep");
            let r = WatchFolderService::scan_once(bs, &PathBuf::from(dir), move_processed)
                .map_err(|e| format!("Erreur watch : {e}"))?;
            Ok(format!(
                "Watch Folder : {} scanné(s), {} importé(s), {} doublon(s), {} erreur(s)",
                r.scanned_files,
                r.imported_book_ids.len(),
                r.skipped_duplicates,
                r.errors.len()
            ))
        }
        "index" => {
            let book_id = args
                .get(1)
                .ok_or_else(|| "Usage: vellum-cli index <book_id>".to_string())?;
            let n = bs
                .index_book_content(book_id)
                .map_err(|e| format!("Erreur index : {e}"))?;
            Ok(format!("Indexé {n} chapitre(s) pour {book_id}"))
        }
        "search-content" => {
            let query = args
                .get(1)
                .ok_or_else(|| "Usage: vellum-cli search-content <requête>".to_string())?;
            let hits = bs
                .search_inside_books(query, 20)
                .map_err(|e| format!("Erreur search-content : {e}"))?;
            let mut lines = vec![format!("{} résultat(s) :", hits.len())];
            for h in hits {
                lines.push(format!(
                    "- [{}#{}] {} : {}",
                    h.book_id, h.chapter_index, h.chapter_title, h.snippet
                ));
            }
            Ok(lines.join("\n"))
        }
        other => Err(format!("Commande inconnue : {other}")),
    }
}

fn exit_err(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

fn print_help() {
    println!(
        "vellum-cli — Gestion headless de bibliothèque Vellum\n\
         \n\
         COMMANDES :\n\
           list [--search <q>] [--tag <t>]         Lister les livres\n\
           import <fichier>                        Importer un ebook\n\
           polish <book_id>                        Polir un EPUB (métadonnées, couverture, TOC)\n\
           convert <fmt> <--all | id...>           Convertir un lot de livres (epub, pdf, cbz, txt)\n\
           audit                                   Auditer la santé de la bibliothèque\n\
           export <csv|json|bibtex> <sortie>       Exporter le catalogue\n\
           watch <dossier> [--keep]                Scanner un dossier Watch Folder\n\
           index <book_id>                         Indexer le texte d'un livre (FTS5)\n\
           search-content <requête>                Rechercher dans le texte des livres\n\
           serve [--port <p>]                      Démarrer le serveur OPDS / Web Companion"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use vellum_core::conversion_service::ConversionService;
    use vellum_core::metadata_service::MetadataService;
    use vellum_core::plugin_service::PluginService;
    use vellum_storage::{BookRepository, Database, ParamRepository, TagRepository};

    fn setup_test_book_service() -> (tempfile::TempDir, BookService) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        let db = Database::open_in_memory().expect("in memory db");
        let book_repo = BookRepository::new(db.clone());
        let param_repo = ParamRepository::new(db.clone());
        let tag_repo = TagRepository::new(db);
        let plugin_service = PluginService::new(param_repo.clone(), Some(tmp.path().join("plugins")));
        let metadata_service = MetadataService::new(plugin_service.clone());
        let conversion_service = ConversionService::new(plugin_service.clone());
        let bs = BookService::new(
            book_repo,
            param_repo,
            tag_repo,
            metadata_service,
            plugin_service,
            conversion_service,
            lib_dir,
        );
        (tmp, bs)
    }

    #[test]
    fn test_cli_import_list_audit_and_export() {
        let (tmp, bs) = setup_test_book_service();
        let txt_path = tmp.path().join("Le Petit Prince.txt");
        std::fs::write(&txt_path, "Dessine-moi un mouton sur l'astéroïde B612.").unwrap();

        let imp_out = dispatch_sync_command(
            &["import".to_string(), txt_path.to_string_lossy().to_string()],
            &bs,
        )
        .expect("import");
        assert!(imp_out.contains("Importé :"));

        let list_out = dispatch_sync_command(&["list".to_string()], &bs).expect("list");
        assert!(list_out.starts_with("1 livre(s) :"));

        let audit_out = dispatch_sync_command(&["audit".to_string()], &bs).expect("audit");
        assert!(audit_out.contains("Audit (score"));

        let csv_out = tmp.path().join("catalog.csv");
        let exp_out = dispatch_sync_command(
            &[
                "export".to_string(),
                "csv".to_string(),
                csv_out.to_string_lossy().to_string(),
            ],
            &bs,
        )
        .expect("export");
        assert!(exp_out.contains("Catalogue exporté (1 livres)"));
        assert!(csv_out.exists());
    }

    #[test]
    fn test_cli_index_and_search_content() {
        let (tmp, bs) = setup_test_book_service();
        let txt_path = tmp.path().join("Dune.txt");
        std::fs::write(
            &txt_path,
            "L'épice gériatrique prolonge la vie sur la planète Arrakis.",
        )
        .unwrap();

        dispatch_sync_command(
            &["import".to_string(), txt_path.to_string_lossy().to_string()],
            &bs,
        )
        .expect("import");
        let books = bs.get_books(None, None).unwrap();
        let book_id = books[0].id.clone();

        let idx_out =
            dispatch_sync_command(&["index".to_string(), book_id], &bs).expect("index");
        assert!(idx_out.contains("Indexé 1 chapitre(s)"));

        let search_out = dispatch_sync_command(
            &["search-content".to_string(), "Arrakis".to_string()],
            &bs,
        )
        .expect("search-content");
        assert!(search_out.contains("1 résultat(s) :"));
        assert!(search_out.contains("Arrakis"));
    }

    #[test]
    fn test_cli_unknown_command_and_missing_args() {
        let (_tmp, bs) = setup_test_book_service();
        let err1 = dispatch_sync_command(&["foobar".to_string()], &bs).unwrap_err();
        assert!(err1.contains("Commande inconnue"));

        let err2 = dispatch_sync_command(&["import".to_string()], &bs).unwrap_err();
        assert!(err2.contains("Usage: vellum-cli import"));
    }

    #[test]
    fn test_cli_bulk_convert_txt_to_epub() {
        let (tmp, bs) = setup_test_book_service();
        let txt1 = tmp.path().join("Livre1.txt");
        let txt2 = tmp.path().join("Livre2.txt");
        std::fs::write(&txt1, "Chapitre 1\n\nContenu du premier livre.").unwrap();
        std::fs::write(&txt2, "Chapitre 1\n\nContenu du second livre.").unwrap();

        dispatch_sync_command(
            &["import".to_string(), txt1.to_string_lossy().to_string()],
            &bs,
        )
        .unwrap();
        dispatch_sync_command(
            &["import".to_string(), txt2.to_string_lossy().to_string()],
            &bs,
        )
        .unwrap();

        let out = dispatch_sync_command(
            &["convert".to_string(), "epub".to_string(), "--all".to_string()],
            &bs,
        )
        .expect("bulk convert");
        assert!(out.contains("2 converti(s), 0 ignoré(s), 0 erreur(s)"));

        // Second pass must skip both since EPUB already exists
        let out2 = dispatch_sync_command(
            &["convert".to_string(), "epub".to_string(), "--all".to_string()],
            &bs,
        )
        .expect("bulk convert second pass");
        assert!(out2.contains("0 converti(s), 2 ignoré(s), 0 erreur(s)"));
    }
}
