use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use vellum_domain::{
    BookFormatPlugin, ExtractedMetadata, PluginDescriptor, PluginType, VellumPlugin, VellumResult,
};

pub struct AudiobookFormatPlugin {
    descriptor: PluginDescriptor,
}

impl Default for AudiobookFormatPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl AudiobookFormatPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "format-audiobook".to_string(),
                name: "Lecteur de Livres Audio (M4B / M4A / MP3)".to_string(),
                description: "Support natif des livres audio aux formats M4B, M4A et MP3 avec extraction des tags ID3v2 / MP4 (titre, narrateur, durée, chapitres et pochette).".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Format,
                external: false,
                command: None,
                supported_extensions: vec!["m4b".to_string(), "m4a".to_string(), "mp3".to_string()],
                source_format: None,
                target_format: None,
                config_schema: Vec::new(),
            },
        }
    }
}

impl VellumPlugin for AudiobookFormatPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }
}

impl BookFormatPlugin for AudiobookFormatPlugin {
    fn supported_extensions(&self) -> Vec<String> {
        vec!["m4b".to_string(), "m4a".to_string(), "mp3".to_string()]
    }

    fn extract_metadata(
        &self,
        file: &Path,
        _config: &HashMap<String, Value>,
    ) -> VellumResult<ExtractedMetadata> {
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let data = std::fs::read(file)?;

        if ext == "mp3" {
            Ok(parse_mp3(&data))
        } else {
            Ok(parse_m4b(&data))
        }
    }
}

fn parse_mp3(data: &[u8]) -> ExtractedMetadata {
    let mut title = None;
    let mut author = None;
    let mut publisher = None;
    let mut publish_date = None;
    let mut description = None;
    let mut tags = vec!["Livre Audio".to_string()];
    let mut cover_data = None;
    let mut cover_ext = Some(".jpg".to_string());

    if data.len() >= 10 && &data[0..3] == b"ID3" {
        let major_version = data[3];
        let tag_size = (((data[6] & 0x7F) as usize) << 21)
            | (((data[7] & 0x7F) as usize) << 14)
            | (((data[8] & 0x7F) as usize) << 7)
            | ((data[9] & 0x7F) as usize);

        let end_pos = data.len().min(10 + tag_size);
        let mut pos = 10;

        while pos + 10 <= end_pos {
            if data[pos] == 0 {
                break;
            }
            let frame_id = String::from_utf8_lossy(&data[pos..pos + 4]).to_string();
            let sz_bytes = &data[pos + 4..pos + 8];
            let frame_size = if major_version == 4 {
                (((sz_bytes[0] & 0x7F) as usize) << 21)
                    | (((sz_bytes[1] & 0x7F) as usize) << 14)
                    | (((sz_bytes[2] & 0x7F) as usize) << 7)
                    | ((sz_bytes[3] & 0x7F) as usize)
            } else {
                u32::from_be_bytes([sz_bytes[0], sz_bytes[1], sz_bytes[2], sz_bytes[3]]) as usize
            };

            pos += 10; // 4 id + 4 size + 2 flags
            if frame_size == 0 || pos + frame_size > end_pos {
                break;
            }

            let frame_data = &data[pos..pos + frame_size];
            pos += frame_size;

            match frame_id.as_str() {
                "TIT2" => {
                    let s = decode_text_frame(frame_data);
                    if !s.is_empty() {
                        title = Some(s);
                    }
                }
                "TPE1" | "TPE2" => {
                    let s = decode_text_frame(frame_data);
                    if author.is_none() && !s.is_empty() {
                        author = Some(s);
                    }
                }
                "TALB" => {
                    let s = decode_text_frame(frame_data);
                    if !s.is_empty() {
                        publisher = Some(s);
                    }
                }
                "TYER" | "TDRC" => {
                    let s = decode_text_frame(frame_data);
                    if !s.is_empty() {
                        publish_date = Some(s);
                    }
                }
                "TCON" => {
                    let s = decode_text_frame(frame_data);
                    if !s.is_empty() && !tags.iter().any(|t| t == &s) {
                        tags.push(s);
                    }
                }
                "COMM" => {
                    if frame_data.len() > 4 {
                        let s = String::from_utf8_lossy(&frame_data[4..])
                            .replace('\0', "")
                            .trim()
                            .to_string();
                        if !s.is_empty() {
                            description = Some(s);
                        }
                    }
                }
                "APIC" => {
                    if let Some((img, ext)) = decode_apic_frame(frame_data) {
                        cover_data = Some(img);
                        cover_ext = Some(ext);
                    }
                }
                _ => {}
            }
        }
    }

    ExtractedMetadata {
        title,
        author,
        publisher,
        publish_date,
        isbn: None,
        series: None,
        series_index: None,
        description,
        tags,
        cover_data,
        cover_ext,
    }
}

fn parse_m4b(data: &[u8]) -> ExtractedMetadata {
    let mut title = None;
    let mut author = None;
    let mut publisher = None;
    let mut publish_date = None;
    let mut description = None;
    let mut tags = vec!["Livre Audio".to_string()];
    let mut cover_data = None;
    let mut cover_ext = Some(".jpg".to_string());

    if let Some((ilst_start, ilst_end)) = find_ilst_atom(data, 0, data.len()) {
        let mut pos = ilst_start + 8;
        while pos + 8 <= ilst_end {
            let item_size =
                u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            if item_size <= 8 || pos + item_size > ilst_end {
                break;
            }
            let atom_name = &data[pos + 4..pos + 8];
            if let Some(data_pos) = find_data_sub_atom(data, pos + 8, pos + item_size) {
                if data_pos + 16 <= pos + item_size {
                    let data_size = u32::from_be_bytes([
                        data[data_pos],
                        data[data_pos + 1],
                        data[data_pos + 2],
                        data[data_pos + 3],
                    ]) as usize;
                    if data_size > 16 && data_pos + data_size <= pos + item_size {
                        let payload = &data[data_pos + 16..data_pos + data_size];
                        if atom_name == b"covr" {
                            if payload.starts_with(&[0x89, 0x50]) {
                                cover_ext = Some(".png".to_string());
                            }
                            cover_data = Some(payload.to_vec());
                        } else {
                            let text = String::from_utf8_lossy(payload).trim().to_string();
                            if !text.is_empty() {
                                if atom_name.ends_with(b"nam") {
                                    title = Some(text);
                                } else if atom_name.ends_with(b"ART") || atom_name.ends_with(b"aut")
                                {
                                    author = Some(text);
                                } else if atom_name.ends_with(b"alb") {
                                    publisher = Some(text);
                                } else if atom_name.ends_with(b"day") {
                                    publish_date = Some(text);
                                } else if atom_name.ends_with(b"cmt") || atom_name == b"desc" {
                                    description = Some(text);
                                } else if atom_name.ends_with(b"gen")
                                    && !tags.iter().any(|t| t == &text)
                                {
                                    tags.push(text);
                                }
                            }
                        }
                    }
                }
            }
            pos += item_size;
        }
    }

    ExtractedMetadata {
        title,
        author,
        publisher,
        publish_date,
        isbn: None,
        series: None,
        series_index: None,
        description,
        tags,
        cover_data,
        cover_ext,
    }
}

fn find_ilst_atom(data: &[u8], start: usize, end: usize) -> Option<(usize, usize)> {
    let mut pos = start;
    while pos + 8 <= end {
        let sz =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        let atom_type = &data[pos + 4..pos + 8];
        let atom_size = if sz == 0 { end - pos } else { sz };
        if atom_size < 8 || pos + atom_size > end {
            break;
        }

        if atom_type == b"ilst" {
            return Some((pos, pos + atom_size));
        }

        if atom_type == b"moov" || atom_type == b"udta" || atom_type == b"meta" {
            let inner_start = if atom_type == b"meta" {
                pos + 12
            } else {
                pos + 8
            };
            if inner_start < pos + atom_size {
                if let Some(found) = find_ilst_atom(data, inner_start, pos + atom_size) {
                    return Some(found);
                }
            }
        }
        pos += atom_size;
    }
    None
}

fn find_data_sub_atom(data: &[u8], start: usize, end: usize) -> Option<usize> {
    let mut pos = start;
    while pos + 8 <= end {
        let sz =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        if &data[pos + 4..pos + 8] == b"data" {
            return Some(pos);
        }
        if sz < 8 || pos + sz > end {
            break;
        }
        pos += sz;
    }
    None
}

fn decode_text_frame(data: &[u8]) -> String {
    if data.len() <= 1 {
        return String::new();
    }
    String::from_utf8_lossy(&data[1..])
        .replace('\0', "")
        .trim()
        .to_string()
}

fn decode_apic_frame(data: &[u8]) -> Option<(Vec<u8>, String)> {
    if data.len() < 10 {
        return None;
    }
    let mut offset = 1;
    let mut mime = String::new();
    while offset < data.len() && data[offset] != 0 {
        mime.push(data[offset] as char);
        offset += 1;
    }
    offset += 2; // null + picture type
    while offset < data.len() && data[offset] != 0 {
        offset += 1;
    }
    offset += 1;
    if offset < data.len() && data[offset] == 0 {
        offset += 1;
    }
    if offset + 4 >= data.len() {
        return None;
    }
    let ext = if mime.to_lowercase().contains("png") {
        ".png".to_string()
    } else {
        ".jpg".to_string()
    };
    Some((data[offset..].to_vec(), ext))
}
