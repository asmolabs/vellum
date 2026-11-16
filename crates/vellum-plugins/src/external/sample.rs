//! Génération du plugin externe d'exemple lors du premier démarrage.

use std::fs;
use std::path::Path;

pub(super) fn ensure_sample_plugin_exists(plugins_dir: &Path) {
    let sample_dir = plugins_dir.join("sample-epub-stamper");
    let manifest_file = sample_dir.join("plugin.json");
    if manifest_file.exists() {
        return;
    }
    let _ = fs::create_dir_all(&sample_dir);

    let manifest_json = r#"{
  "id": "ext-sample-stamper",
  "name": "Sample Ex-Libris Stamper (Script Externe)",
  "version": "1.0.0",
  "author": "Vellum Community",
  "description": "Exemple de plugin externe (Python/Shell) qui copie un fichier en ajoutant un ex-libris configurable.",
  "type": "TRANSFORMATION",
  "command": ["python3", "plugin.py"],
  "supportedExtensions": ["epub", "txt", "pdf"],
  "outputSuffix": "stamped",
  "attachToBook": true,
  "configSchema": [
    {
      "key": "ownerName",
      "label": "Ex-Libris (Propriétaire)",
      "type": "TEXT",
      "defaultValue": "Bibliothèque Vellum",
      "description": "Nom inscrit dans les métadonnées ou le journal d'exécution",
      "options": []
    }
  ]
}"#;
    let script_py = r#"#!/usr/bin/env python3
import sys
import shutil

def main():
    args = sys.argv[1:]
    if "--test" in args:
        print("Script Python Ex-Libris opérationnel !")
        sys.exit(0)

    input_path = None
    output_path = None
    for i, a in enumerate(args):
        if a == "--input" and i + 1 < len(args):
            input_path = args[i + 1]
        elif a == "--output" and i + 1 < len(args):
            output_path = args[i + 1]

    if input_path and output_path:
        shutil.copyfile(input_path, output_path)
        sys.exit(0)

    sys.exit(1)

if __name__ == "__main__":
    main()
"#;
    let _ = fs::write(&manifest_file, manifest_json);
    let _ = fs::write(sample_dir.join("plugin.py"), script_py);
}
