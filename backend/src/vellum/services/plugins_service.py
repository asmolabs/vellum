import os
import shutil
import json
import re
import zipfile
from typing import List, Optional
from sqlalchemy.orm import Session
from ..models.param import Param

class LibraryPlugin:
    def __init__(self, id: str, name: str, description: str, version: str, type: str, config_schema: Optional[List[dict]] = None):
        self.id = id
        self.name = name
        self.description = description
        self.version = version
        self.type = type
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

    # Patterns to match OceanOFPDF links, URLs, text mentions, and paragraphs
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
        # 1. Google Books Metadata Enricher Plugin
        google_books = LibraryPlugin(
            id="google-books-enricher",
            name="Google Books Metadata Enricher",
            description="Enrichit automatiquement les métadonnées et télécharge des tags supplémentaires depuis Google Books à l'importation.",
            version="1.0.0",
            type="import",
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

        # 2. EPUB Translation Plugin
        epub_translator = LibraryPlugin(
            id="epub-translator-fr",
            name="Traducteur EPUB (FR)",
            description="Traduit le contenu d'un livre EPUB à l'aide d'un moteur d'IA.",
            version="1.0.0",
            type="transformation",
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
                },
                {
                    "key": "api_token",
                    "label": "Jeton / Clé API (Optionnel)",
                    "type": "text",
                    "default": "",
                    "description": "Jeton d'authentification pour le service distant de traduction."
                }
            ]
        )

        # 3. OceanOFPDF Cleaner Plugin
        oceanofpdf_cleaner = LibraryPlugin(
            id="oceanofpdf-cleaner",
            name="Nettoyeur OceanOFPDF",
            description="Supprime automatiquement les liens, filigranes et paragraphes 'OceanofPDF' des fichiers EPUB.",
            version="1.0.0",
            type="transformation",
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
                    "description": "Remplace le lien/texte OceanOFPDF par ce texte. Laisser vide pour supprimer silencieusement."
                }
            ]
        )

        self.plugins.extend([google_books, epub_translator, oceanofpdf_cleaner])

    def get_plugins(self) -> List[dict]:
        return [
            {
                "id": p.id,
                "name": p.name,
                "description": p.description,
                "version": p.version,
                "type": p.type,
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

        # Default fallback / Translation plugin
        dir_name = os.path.dirname(input_path)
        base, ext = os.path.splitext(os.path.basename(input_path))
        output_filename = f"{base} - FR{ext}"
        output_path = os.path.join(dir_name, output_filename)

        shutil.copyfile(input_path, output_path)
        return output_path, output_filename
