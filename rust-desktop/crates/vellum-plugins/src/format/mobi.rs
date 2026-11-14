use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use vellum_domain::{
    BookFormatPlugin, ExtractedMetadata, PluginDescriptor, PluginType, VellumError, VellumPlugin,
    VellumResult,
};

pub struct MobiFormatPlugin {
    descriptor: PluginDescriptor,
}

impl Default for MobiFormatPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MobiFormatPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "format-mobi".to_string(),
                name: "Lecteur de format Kindle (MOBI / AZW3)".to_string(),
                description: "Support natif des livres numériques Amazon Kindle (MOBI, AZW, AZW3/KF8) avec extraction des métadonnées EXTH et de la couverture.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Format,
                external: false,
                command: None,
                supported_extensions: vec![
                    "mobi".to_string(),
                    "azw3".to_string(),
                    "azw".to_string(),
                ],
                source_format: None,
                target_format: None,
                config_schema: Vec::new(),
            },
        }
    }
}

impl VellumPlugin for MobiFormatPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn as_format_plugin(&self) -> Option<&dyn BookFormatPlugin> {
        Some(self)
    }
}

impl BookFormatPlugin for MobiFormatPlugin {
    fn supported_extensions(&self) -> Vec<String> {
        vec!["mobi".to_string(), "azw3".to_string(), "azw".to_string()]
    }

    fn extract_metadata(
        &self,
        file: &Path,
        _config: &HashMap<String, Value>,
    ) -> VellumResult<ExtractedMetadata> {
        let data = std::fs::read(file)?;
        if data.len() < 78 {
            return Err(VellumError::InvalidArgument(
                "Fichier trop court pour un format MOBI/PDB valide".to_string(),
            ));
        }

        let num_records = u16::from_be_bytes([data[76], data[77]]) as usize;
        if num_records == 0 || 78 + num_records * 8 > data.len() {
            return Err(VellumError::InvalidArgument(
                "Aucun enregistrement PDB trouvé dans le fichier MOBI".to_string(),
            ));
        }

        let mut record_offsets = Vec::with_capacity(num_records);
        for i in 0..num_records {
            let base = 78 + i * 8;
            let offset =
                u32::from_be_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]])
                    as usize;
            record_offsets.push(offset);
        }

        let rec0 = record_offsets[0];
        if rec0 + 32 > data.len() {
            return Err(VellumError::InvalidArgument(
                "En-tête Record 0 invalide".to_string(),
            ));
        }

        let mut title = None;
        let mut author = None;
        let mut publisher = None;
        let mut publish_date = None;
        let mut isbn = None;
        let mut description = None;
        let mut tags = vec!["Kindle".to_string()];
        let mut cover_data = None;
        let mut cover_ext = Some(".jpg".to_string());

        let mobi_header_len = read_u32(&data, rec0 + 20) as usize;

        // Title offset (rec0 + 84) & length (rec0 + 88)
        if rec0 + 92 <= data.len() {
            let t_off = read_u32(&data, rec0 + 84) as usize;
            let t_len = read_u32(&data, rec0 + 88) as usize;
            if t_off > 0 && t_len > 0 && rec0 + t_off + t_len <= data.len() {
                let s = String::from_utf8_lossy(&data[rec0 + t_off..rec0 + t_off + t_len])
                    .trim()
                    .to_string();
                if !s.is_empty() {
                    title = Some(s);
                }
            }
        }

        let first_image_index = if rec0 + 112 <= data.len() {
            Some(read_u32(&data, rec0 + 108) as usize)
        } else {
            None
        };

        let exth_flags = if rec0 + 132 <= data.len() {
            read_u32(&data, rec0 + 128)
        } else {
            0
        };

        let mut cover_offset: Option<usize> = None;
        let has_exth = (exth_flags & 0x40) != 0;
        let exth_offset = rec0 + 16 + mobi_header_len;

        if has_exth
            && exth_offset + 12 <= data.len()
            && &data[exth_offset..exth_offset + 4] == b"EXTH"
        {
            let exth_len = read_u32(&data, exth_offset + 4) as usize;
            let exth_count = read_u32(&data, exth_offset + 8) as usize;
            let mut pos = exth_offset + 12;
            let exth_end = (exth_offset + exth_len).min(data.len());

            for _ in 0..exth_count {
                if pos + 8 > exth_end {
                    break;
                }
                let tag = read_u32(&data, pos);
                let len = read_u32(&data, pos + 4) as usize;
                if len < 8 || pos + len > exth_end {
                    break;
                }
                let val_bytes = &data[pos + 8..pos + len];
                let val_str = String::from_utf8_lossy(val_bytes).trim().to_string();

                match tag {
                    100 if author.is_none() && !val_str.is_empty() => author = Some(val_str),
                    101 if !val_str.is_empty() => publisher = Some(val_str),
                    103 if !val_str.is_empty() => description = Some(val_str),
                    104 if !val_str.is_empty() => isbn = Some(val_str),
                    105 if !val_str.is_empty() => {
                        if !tags.iter().any(|t| t == &val_str) {
                            tags.push(val_str);
                        }
                    }
                    106 if !val_str.is_empty() => publish_date = Some(val_str),
                    201 if val_bytes.len() == 4 => {
                        cover_offset = Some(u32::from_be_bytes([
                            val_bytes[0],
                            val_bytes[1],
                            val_bytes[2],
                            val_bytes[3],
                        ]) as usize);
                    }
                    503 if !val_str.is_empty() => title = Some(val_str),
                    _ => {}
                }
                pos += len;
            }
        }

        if let (Some(c_off), Some(first_img)) = (cover_offset, first_image_index) {
            let idx = first_img.saturating_add(c_off);
            if idx < num_records {
                let start = record_offsets[idx];
                let end = if idx + 1 < num_records {
                    record_offsets[idx + 1]
                } else {
                    data.len()
                };
                if end > start + 4 && end <= data.len() {
                    let img = data[start..end].to_vec();
                    if img.starts_with(&[0x89, 0x50]) {
                        cover_ext = Some(".png".to_string());
                    }
                    cover_data = Some(img);
                }
            }
        }

        Ok(ExtractedMetadata {
            title,
            author,
            publisher,
            publish_date,
            isbn,
            series: None,
            series_index: None,
            description,
            tags,
            cover_data,
            cover_ext,
        })
    }
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}
