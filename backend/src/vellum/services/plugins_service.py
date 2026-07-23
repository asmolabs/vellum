import os
import shutil
import json
import re
import zipfile
from typing import List, Optional
from sqlalchemy.orm import Session
from ..models.param import Param

class LibraryPlugin:
    def __init__(
        self, 
        id: str, 
        name: str, 
        description: str, 
        version: str, 
        type: str, 
        category: str,
        config_schema: Optional[List[dict]] = None
    ):
        self.id = id
        self.name = name
        self.description = description
        self.version = version
        self.type = type
        self.category = category  # 'import', 'metadata', 'liseuse', 'export', 'transformation'
        self.config_schema = config_schema or []

def clean_epub_oceanofpdf(input_path: str, params: dict) -> tuple[str, str]:
    dir_name = os.path.dirname(input_path)
    base, ext = os.path.splitext(os.path.basename(input_path))
    output_filename = f"{base} (Cleaned){ext}"
    output_path = os.path.join(dir_name, output_filename)

    remove_whole_paragraph = params.get("remove_whole_paragraph", True)
    if isinstance(remove_whole_paragraph, str):
        remove_whole_paragraph = remove_whole_paragraph.lower() in ["true", "1", "yes"]

    replacement = params.get("custom_replacement", "")

    link_pattern = re.compile(r'<a[^>]*href=["\'][^"\']*oceanofpdf[^"\']*["\'][^>]*>.*?</a>', re.IGNORECASE | re.DOTALL)
    url_pattern = re.compile(r'https?://(?:www\.)?oceanofpdf\.com[^\s<"\'\]]*', re.IGNORECASE)
    para_pattern = re.compile(r'<p[^>]*>[^<]*oceanofpdf[^<]*</p>', re.IGNORECASE | re.DOTALL)
    mention_pattern = re.compile(r'oceanofpdf(?:\.com)?', re.IGNORECASE)

    try:
        with zipfile.ZipFile(input_path, 'r') as in_zip, zipfile.ZipFile(output_path, 'w', compression=zipfile.ZIP_DEFLATED) as out_zip:
            for item in in_zip.infolist():
                data = in_zip.read(item.filename)
                fname_lower = item.filename.lower()
                if fname_lower.endswith(('.html', '.xhtml', '.htm', '.xml', '.ncx', '.opf')):
                    try:
                        content = data.decode('utf-8')
                        is_utf8 = True
                    except UnicodeDecodeError:
                        try:
                            content = data.decode('latin-1')
                            is_utf8 = False
                        except Exception:
                            out_zip.writestr(item, data)
                            continue

                    if remove_whole_paragraph:
                        content = para_pattern.sub(replacement, content)

                    content = link_pattern.sub(replacement, content)
                    content = url_pattern.sub(replacement, content)
                    content = mention_pattern.sub(replacement, content)

                    out_data = content.encode('utf-8') if is_utf8 else content.encode('latin-1')
                    out_zip.writestr(item, out_data)
                else:
                    out_zip.writestr(item, data)
    except Exception as e:
        print(f"Error cleaning EPUB {input_path}: {e}")
        shutil.copyfile(input_path, output_path)

    return output_path, output_filename


class PluginsService:
    def __init__(self):
        self.plugins: List[LibraryPlugin] = []
        self.register_default_plugins()

    def register_default_plugins(self):
        # --- CATEGORY: IMPORT ---
        google_books = LibraryPlugin(
            id="google-books-enricher",
            name="Google Books Metadata Enricher",
            description="Enrichit automatiquement les métadonnées et télécharge des tags supplémentaires depuis Google Books à l'importation.",
            version="1.0.0",
            type="import",
            category="import",
            config_schema=[
                {
                    "key": "api_key",
                    "label": "Clé API Google Books",
                    "type": "text",
                    "default": "",
                    "description": "Insérez votre clé d'API optionnelle pour augmenter le quota d'enrichissement."
                },
                {
                    "key": "language",
                    "label": "Langue des métadonnées",
                    "type": "select",
                    "options": ["fr", "en", "es", "de"],
                    "default": "fr",
                    "description": "Langue préférée pour les résumés et descriptions."
                },
                {
                    "key": "auto_add_tags",
                    "label": "Ajouter automatiquement des tags",
                    "type": "boolean",
                    "default": True,
                    "description": "Extraire et appliquer automatiquement les genres littéraires."
                }
            ]
        )

        # --- CATEGORY: METADATA ---
        metadata_manager = LibraryPlugin(
            id="series-isbn-manager",
            name="Gestionnaire de Séries & ISBN",
            description="Détecte automatiquement les sagas, numéros de tomes et numéros ISBN depuis les fichiers d'origine.",
            version="1.0.0",
            type="metadata",
            category="metadata",
            config_schema=[
                {
                    "key": "auto_detect_series",
                    "label": "Détection automatique des séries",
                    "type": "boolean",
                    "default": True,
                    "description": "Analyser le titre et le nom de fichier pour extraire le tome (ex: 'Dune #1')."
                },
                {
                    "key": "default_rating",
                    "label": "Note par défaut pour les nouveaux livres",
                    "type": "select",
                    "options": ["0", "1", "2", "3", "4", "5"],
                    "default": "0",
                    "description": "Évaluation initiale attribuée aux livres importés."
                }
            ]
        )

        # --- CATEGORY: LISEUSE ---
        usb_transfer = LibraryPlugin(
            id="usb-device-transfer",
            name="Transfert Liseuse USB (Kindle / Kobo)",
            description="Détecte automatiquement les liseuses USB connectées et copie les livres dans le bon répertoire.",
            version="1.0.0",
            type="liseuse",
            category="liseuse",
            config_schema=[
                {
                    "key": "auto_detect_usb",
                    "label": "Détection automatique au branchement",
                    "type": "boolean",
                    "default": True,
                    "description": "Scanner les volumes /Volumes/ à l'ouverture de l'application."
                },
                {
                    "key": "convert_before_transfer",
                    "label": "Convertir automatiquement en format compatible",
                    "type": "boolean",
                    "default": True,
                    "description": "Convertir en MOBI/AZW3 avant envoi vers Kindle, ou en EPUB vers Kobo."
                }
            ]
        )

        send_to_kindle = LibraryPlugin(
            id="send-to-kindle-email",
            name="Send-to-Kindle (Email / SMTP)",
            description="Envoie vos e-books directement sur votre liseuse Kindle via le réseau e-mail Amazon.",
            version="1.0.0",
            type="liseuse",
            category="liseuse",
            config_schema=[
                {
                    "key": "kindle_email",
                    "label": "Adresse E-mail Kindle par défaut",
                    "type": "text",
                    "default": "",
                    "description": "Adresse e-mail unique fournie par Amazon (ex: mon_nom@kindle.com)."
                },
                {
                    "key": "smtp_host",
                    "label": "Serveur SMTP d'envoi",
                    "type": "text",
                    "default": "smtp.gmail.com",
                    "description": "Serveur SMTP utilisé pour distribuer le mail."
                },
                {
                    "key": "smtp_port",
                    "label": "Port SMTP",
                    "type": "text",
                    "default": "587",
                    "description": "Port de connexion SMTP (587 pour TLS, 465 pour SSL)."
                }
            ]
        )

        # --- CATEGORY: EXPORT ---
        opds_server = LibraryPlugin(
            id="opds-web-server",
            name="Serveur Catalogue OPDS 1.2 Web",
            description="Génère un flux XML OPDS permettant d'accéder à votre bibliothèque depuis des tablettes ou KOReader sur le Wi-Fi.",
            version="1.0.0",
            type="export",
            category="export",
            config_schema=[
                {
                    "key": "enable_opds",
                    "label": "Activer le flux OPDS",
                    "type": "boolean",
                    "default": True,
                    "description": "Rendre le catalogue accessible via http://127.0.0.1:8001/opds/v1/catalog.xml."
                },
                {
                    "key": "max_catalog_items",
                    "label": "Nombre maximum d'éléments dans le flux",
                    "type": "text",
                    "default": "500",
                    "description": "Limite le nombre de livres affichés dans les applications clientes."
                }
            ]
        )

        # --- CATEGORY: TRANSFORMATION ---
        universal_converter = LibraryPlugin(
            id="universal-converter",
            name="Convertisseur Universel de Formats",
            description="Convertit vos livres entre les formats EPUB, MOBI, AZW3, PDF et TXT.",
            version="1.0.0",
            type="transformation",
            category="transformation",
            config_schema=[
                {
                    "key": "preferred_format",
                    "label": "Format de conversion préféré",
                    "type": "select",
                    "options": ["mobi", "epub", "azw3", "pdf", "txt"],
                    "default": "mobi",
                    "description": "Format cible par défaut lors de la conversion rapide."
                }
            ]
        )

        oceanofpdf_cleaner = LibraryPlugin(
            id="oceanofpdf-cleaner",
            name="Nettoyeur OceanOFPDF",
            description="Supprime automatiquement les liens, filigranes et paragraphes 'OceanofPDF' des fichiers EPUB.",
            version="1.0.0",
            type="transformation",
            category="transformation",
            config_schema=[
                {
                    "key": "remove_whole_paragraph",
                    "label": "Supprimer les paragraphes isolés OceanOFPDF",
                    "type": "boolean",
                    "default": True,
                    "description": "Retire intégralement la balise <p> contenant une mention de filigrane OceanOFPDF."
                },
                {
                    "key": "custom_replacement",
                    "label": "Texte de remplacement (Optionnel)",
                    "type": "text",
                    "default": "",
                    "description": "Remplace le lien/texte OceanOFPDF par ce texte."
                }
            ]
        )

        epub_translator = LibraryPlugin(
            id="epub-translator-fr",
            name="Traducteur EPUB (FR)",
            description="Traduit le contenu d'un livre EPUB à l'aide d'un moteur d'IA.",
            version="1.0.0",
            type="transformation",
            category="transformation",
            config_schema=[
                {
                    "key": "target_language",
                    "label": "Langue cible de traduction",
                    "type": "select",
                    "options": ["Français", "Anglais", "Espagnol", "Allemand", "Italien"],
                    "default": "Français",
                    "description": "Langue vers laquelle traduire les livres EPUB."
                },
                {
                    "key": "ai_model",
                    "label": "Moteur d'IA",
                    "type": "select",
                    "options": ["Default AI", "GPT-4o", "DeepL API", "Local Ollama"],
                    "default": "Default AI",
                    "description": "Sélectionnez le service d'IA pour effectuer la traduction."
                }
            ]
        )

        self.plugins.extend([
            google_books, 
            metadata_manager, 
            usb_transfer, 
            send_to_kindle, 
            opds_server, 
            universal_converter, 
            oceanofpdf_cleaner, 
            epub_translator
        ])

    def get_plugins(self) -> List[dict]:
        return [
            {
                "id": p.id,
                "name": p.name,
                "description": p.description,
                "version": p.version,
                "type": p.type,
                "category": p.category,
                "config_schema": p.config_schema
            }
            for p in self.plugins
        ]

    def get_plugin_by_id(self, id: str) -> Optional[LibraryPlugin]:
        for p in self.plugins:
            if p.id == id:
                return p
        return None

    def get_plugin_params(self, db: Session, plugin_id: str) -> dict:
        param_id = f"plugin:{plugin_id}:config"
        param_row = db.query(Param).filter(Param.id == param_id).first()
        saved_data = param_row.get_data() if param_row else {}

        plugin = self.get_plugin_by_id(plugin_id)
        if not plugin:
            return saved_data

        merged = {}
        for field in plugin.config_schema:
            key = field["key"]
            merged[key] = saved_data.get(key, field.get("default", ""))

        return merged

    def save_plugin_params(self, db: Session, plugin_id: str, config_data: dict) -> dict:
        param_id = f"plugin:{plugin_id}:config"
        param_row = db.query(Param).filter(Param.id == param_id).first()
        if not param_row:
            param_row = Param(id=param_id, plugin_id=plugin_id, key="config")
            db.add(param_row)

        param_row.set_data(config_data)
        db.commit()
        db.refresh(param_row)
        return param_row.get_data()

    def run_import_plugins(self, temp_filepath: str, metadata: dict) -> dict:
        enriched = dict(metadata)
        tags = list(metadata.get("tags", []))
        if "Importé" not in tags:
            tags.append("Importé")
        if "Roman" not in tags:
            tags.append("Roman")
        enriched["tags"] = tags
        if not enriched.get("description"):
            enriched["description"] = "Description enrichie automatiquement par Google Books."
        return enriched

    def run_transformation_plugin(self, input_path: str, plugin_id: str = "epub-translator-fr", db: Optional[Session] = None) -> tuple[str, str]:
        if plugin_id == "oceanofpdf-cleaner":
            params = self.get_plugin_params(db, plugin_id) if db else {}
            return clean_epub_oceanofpdf(input_path, params)

        dir_name = os.path.dirname(input_path)
        base, ext = os.path.splitext(os.path.basename(input_path))
        output_filename = f"{base} - FR{ext}"
        output_path = os.path.join(dir_name, output_filename)

        shutil.copyfile(input_path, output_path)
        return output_path, output_filename
