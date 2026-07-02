import os
import shutil

class LibraryPlugin:
    def __init__(self, id: str, name: str, description: str, version: str, type: str):
        self.id = id
        self.name = name
        self.description = description
        self.version = version
        self.type = type

class PluginsService:
    def __init__(self):
        self.plugins = []
        self.register_default_plugins()
        
    def register_default_plugins(self):
        # 1. Google Books Metadata Enricher Plugin (Mock Import Plugin)
        google_books = LibraryPlugin(
            id="google-books-enricher",
            name="Google Books Metadata Enricher",
            description="Enrichit automatiquement les métadonnées et télécharge des tags supplémentaires depuis Google Books à l'importation.",
            version="1.0.0",
            type="import"
        )
        
        # 2. Mock EPUB Translation Plugin (Mock Transformation Plugin)
        epub_translator = LibraryPlugin(
            id="epub-translator-fr",
            name="Traducteur EPUB (FR)",
            description="Traduit le contenu d'un livre EPUB en Français à l'aide d'un modèle d'IA.",
            version="1.0.0",
            type="transformation"
        )
        
        self.plugins.extend([google_books, epub_translator])
        
    def get_plugins(self):
        return [
            {
                "id": p.id,
                "name": p.name,
                "description": p.description,
                "version": p.version,
                "type": p.type
            }
            for p in self.plugins
        ]
        
    def get_plugin_by_id(self, id: str):
        for p in self.plugins:
            if p.id == id:
                return p
        return None

    def run_import_plugins(self, temp_filepath: str, metadata: dict) -> dict:
        # Simulate import plugins (like google-books-enricher)
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

    def run_transformation_plugin(self, input_path: str) -> tuple[str, str]:
        # Simulates translation process
        # Copy input file to output file with " - FR" suffix
        dir_name = os.path.dirname(input_path)
        base, ext = os.path.splitext(os.path.basename(input_path))
        output_filename = f"{base} - FR{ext}"
        output_path = os.path.join(dir_name, output_filename)
        
        shutil.copyfile(input_path, output_path)
        return output_path, output_filename
