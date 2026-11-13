//! Interface en ligne de commande (mode headless) pour Vellum.

use std::path::{Path, PathBuf};

use vellum_core::{
    init_services, CatalogExporter, CatalogFormat, OpdsServer, PolishOptions, WatchFolderService,
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

    match args[0].as_str() {
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
            match bs.get_books(search, tag) {
                Ok(books) => {
                    println!("{} livre(s) :", books.len());
                    for b in books {
                        let fmts: Vec<&str> = b.formats.iter().map(|f| f.format.as_str()).collect();
                        println!("- [{}] {} — {} ({})", b.id, b.title, b.author, fmts.join(", "));
                    }
                }
                Err(e) => exit_err(&format!("Erreur list : {e}")),
            }
        }
        "import" => {
            let Some(path) = args.get(1) else {
                exit_err("Usage: vellum-cli import <fichier>");
            };
            match bs.import_book(Path::new(path)) {
                Ok(book) => println!("Importé : [{}] {} — {}", book.id, book.title, book.author),
                Err(e) => exit_err(&format!("Erreur import : {e}")),
            }
        }
        "polish" => {
            let Some(book_id) = args.get(1) else {
                exit_err("Usage: vellum-cli polish <book_id>");
            };
            match bs.polish_book(book_id, PolishOptions::default()) {
                Ok(r) => println!(
                    "Polish terminé : metadata={}, cover={}, chapitres={}, toc={}, repaired={}",
                    r.metadata_updated, r.cover_embedded, r.chapters_smartened, r.toc_rebuilt, r.repaired
                ),
                Err(e) => exit_err(&format!("Erreur polish : {e}")),
            }
        }
        "audit" => match bs.audit_library() {
            Ok(report) => {
                println!(
                    "Audit (score {}%) : {} livres, {} doublons potentiels, {} fiches incomplètes, {} fichiers manquants",
                    report.overall_score,
                    report.total_books,
                    report.duplicate_pairs.len(),
                    report.incomplete_issues.len(),
                    report.missing_file_issues.len()
                );
            }
            Err(e) => exit_err(&format!("Erreur audit : {e}")),
        },
        "export" => {
            if args.len() < 3 {
                exit_err("Usage: vellum-cli export <csv|json|bibtex> <fichier_sortie>");
            }
            let fmt = match args[1].to_lowercase().as_str() {
                "csv" => CatalogFormat::Csv,
                "json" => CatalogFormat::Json,
                "bib" | "bibtex" => CatalogFormat::Bibtex,
                other => exit_err(&format!("Format inconnu : {other} (attendu: csv, json, bibtex)")),
            };
            let books = bs.get_books(None, None).unwrap_or_default();
            if let Err(e) = CatalogExporter::export_to_file(&books, fmt, Path::new(&args[2])) {
                exit_err(&format!("Erreur export : {e}"));
            }
            println!("Catalogue exporté ({} livres) -> {}", books.len(), args[2]);
        }
        "watch" => {
            let Some(dir) = args.get(1) else {
                exit_err("Usage: vellum-cli watch <dossier> [--keep]");
            };
            let move_processed = !args.iter().any(|a| a == "--keep");
            match WatchFolderService::scan_once(&bs, &PathBuf::from(dir), move_processed) {
                Ok(r) => println!(
                    "Watch Folder : {} scanné(s), {} importé(s), {} doublon(s), {} erreur(s)",
                    r.scanned_files,
                    r.imported_book_ids.len(),
                    r.skipped_duplicates,
                    r.errors.len()
                ),
                Err(e) => exit_err(&format!("Erreur watch : {e}")),
            }
        }
        "index" => {
            let Some(book_id) = args.get(1) else {
                exit_err("Usage: vellum-cli index <book_id>");
            };
            match bs.index_book_content(book_id) {
                Ok(n) => println!("Indexé {n} chapitre(s) pour {book_id}"),
                Err(e) => exit_err(&format!("Erreur index : {e}")),
            }
        }
        "search-content" => {
            let Some(query) = args.get(1) else {
                exit_err("Usage: vellum-cli search-content <requête>");
            };
            match bs.search_inside_books(query, 20) {
                Ok(hits) => {
                    println!("{} résultat(s) :", hits.len());
                    for h in hits {
                        println!("- [{}#{}] {} : {}", h.book_id, h.chapter_index, h.chapter_title, h.snippet);
                    }
                }
                Err(e) => exit_err(&format!("Erreur search-content : {e}")),
            }
        }
        "serve" => {
            let port = args
                .windows(2)
                .find(|w| w[0] == "--port")
                .and_then(|w| w[1].parse::<u16>().ok())
                .unwrap_or(services.opds_port);
            println!("Serveur OPDS / Web Companion démarré sur http://0.0.0.0:{port}");
            if let Err(e) = OpdsServer::run_server((*bs).clone(), port).await {
                exit_err(&format!("Erreur serveur : {e}"));
            }
        }
        other => {
            eprintln!("Commande inconnue : {other}");
            print_help();
            std::process::exit(1);
        }
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
           audit                                   Auditer la santé de la bibliothèque\n\
           export <csv|json|bibtex> <sortie>       Exporter le catalogue\n\
           watch <dossier> [--keep]                Scanner un dossier Watch Folder\n\
           index <book_id>                         Indexer le texte d'un livre (FTS5)\n\
           search-content <requête>                Rechercher dans le texte des livres\n\
           serve [--port <p>]                      Démarrer le serveur OPDS / Web Companion"
    );
}
