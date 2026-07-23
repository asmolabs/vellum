import os
import shutil
import subprocess

class ConverterService:
    @staticmethod
    def is_calibre_available() -> bool:
        """Checks if calibre ebook-convert CLI tool is installed."""
        return shutil.which("ebook-convert") is not None

    @staticmethod
    def convert_ebook(input_path: str, target_format: str) -> tuple[str, str]:
        """Converts input ebook to target_format (epub, mobi, pdf, txt, azw3)."""
        if not os.path.exists(input_path):
            raise Exception(f"Input file not found: {input_path}")

        target_format = target_format.lower().replace(".", "")
        if target_format not in ["epub", "mobi", "pdf", "txt", "azw3"]:
            raise Exception(f"Unsupported target format: {target_format}")

        dir_name = os.path.dirname(input_path)
        base, _ = os.path.splitext(os.path.basename(input_path))
        output_filename = f"{base}.{target_format}"
        output_path = os.path.join(dir_name, output_filename)

        if shutil.which("ebook-convert"):
            # Use Calibre CLI ebook-convert tool for high precision
            cmd = ["ebook-convert", input_path, output_path]
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            if res.returncode != 0:
                raise Exception(f"Conversion failed: {res.stderr}")
            return output_path, output_filename
        else:
            # Basic Python fallback: copy file if same or create text summary
            if input_path.endswith(f".{target_format}"):
                shutil.copyfile(input_path, output_path)
                return output_path, output_filename
            raise Exception("Le convertisseur Calibre CLI 'ebook-convert' n'a pas été détecté sur votre système. Veuillez installer Calibre pour activer la conversion universelle.")
