use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vellum_domain::{BookConverterPlugin, VellumError, VellumResult};

use crate::plugin_service::PluginService;

pub const PIVOT_FORMAT_TEXT: &str = "EPUB";
pub const PIVOT_FORMAT_COMIC: &str = "CBZ";

pub fn canonical_format(fmt: &str) -> String {
    let s = fmt.trim().to_uppercase();
    match s.as_str() {
        "TEXT" => "TXT".to_string(),
        "MARKDOWN" => "MD".to_string(),
        "JPEG" => "JPG".to_string(),
        _ => s,
    }
}

#[derive(Clone)]
pub struct ConversionService {
    plugin_service: PluginService,
}

impl ConversionService {
    pub fn new(plugin_service: PluginService) -> Self {
        Self { plugin_service }
    }

    /// Finds the shortest conversion path (direct or via pivot formats)
    pub fn find_conversion_path(
        &self,
        source_format: &str,
        target_format: &str,
    ) -> Option<Vec<Arc<dyn BookConverterPlugin>>> {
        let src_upper = canonical_format(source_format);
        let dst_upper = canonical_format(target_format);

        if src_upper.is_empty() || dst_upper.is_empty() {
            return None;
        }

        if src_upper == dst_upper {
            return Some(Vec::new());
        }

        let plugins = self.plugin_service.get_converter_plugins();

        // 1. Direct 1-step conversion
        if let Some(direct) = plugins.iter().find(|c| {
            c.source_format().eq_ignore_ascii_case(&src_upper)
                && c.target_format().eq_ignore_ascii_case(&dst_upper)
        }) {
            return Some(vec![Arc::clone(direct)]);
        }

        // 2. Canonical 2-step conversion via designated pivot formats (EPUB for text, CBZ for comics)
        for &pivot in &[PIVOT_FORMAT_TEXT, PIVOT_FORMAT_COMIC] {
            if !src_upper.eq_ignore_ascii_case(pivot) && !dst_upper.eq_ignore_ascii_case(pivot) {
                let step1 = plugins.iter().find(|c| {
                    c.source_format().eq_ignore_ascii_case(&src_upper)
                        && c.target_format().eq_ignore_ascii_case(pivot)
                });
                let step2 = plugins.iter().find(|c| {
                    c.source_format().eq_ignore_ascii_case(pivot)
                        && c.target_format().eq_ignore_ascii_case(&dst_upper)
                });

                if let (Some(s1), Some(s2)) = (step1, step2) {
                    return Some(vec![Arc::clone(s1), Arc::clone(s2)]);
                }
            }
        }

        // 3. General Breadth-First Search (BFS) graph path search
        let mut queue: VecDeque<(String, Vec<Arc<dyn BookConverterPlugin>>)> = VecDeque::new();
        let mut visited: HashSet<String> = HashSet::new();

        queue.push_back((src_upper.clone(), Vec::new()));
        visited.insert(src_upper.clone());

        while let Some((curr_fmt, path)) = queue.pop_front() {
            for p in &plugins {
                if p.source_format().eq_ignore_ascii_case(&curr_fmt) {
                    let next_fmt = p.target_format().to_uppercase();
                    let mut new_path = path.clone();
                    new_path.push(Arc::clone(p));

                    if next_fmt.eq_ignore_ascii_case(&dst_upper) {
                        return Some(new_path);
                    }

                    if !visited.contains(&next_fmt) {
                        visited.insert(next_fmt.clone());
                        queue.push_back((next_fmt, new_path));
                    }
                }
            }
        }

        None
    }

    /// Lists all achievable target formats from a given source format (including via pivot formats)
    pub fn available_target_formats(&self, source_format: &str) -> Vec<String> {
        let src_upper = canonical_format(source_format);
        let plugins = self.plugin_service.get_converter_plugins();
        let mut reachable: HashSet<String> = HashSet::new();

        let mut queue: VecDeque<String> = VecDeque::new();
        let mut visited: HashSet<String> = HashSet::new();

        queue.push_back(src_upper.clone());
        visited.insert(src_upper.clone());

        while let Some(curr) = queue.pop_front() {
            for p in &plugins {
                let p_src = canonical_format(p.source_format());
                if p_src.eq_ignore_ascii_case(&curr) {
                    let next = canonical_format(p.target_format());
                    reachable.insert(next.clone());
                    if !visited.contains(&next) {
                        visited.insert(next.clone());
                        queue.push_back(next);
                    }
                }
            }
        }

        reachable.remove(&src_upper);
        let mut list: Vec<String> = reachable.into_iter().collect();
        list.sort();
        list
    }

    /// Converts a file from source_format to target_format, using direct or pivot pipeline
    pub fn convert_file(
        &self,
        source_format: &str,
        target_format: &str,
        source_file: &Path,
        target_file: &Path,
        custom_config: Option<&HashMap<String, serde_json::Value>>,
    ) -> VellumResult<PathBuf> {
        let src_upper = canonical_format(source_format);
        let dst_upper = canonical_format(target_format);

        let path = self
            .find_conversion_path(&src_upper, &dst_upper)
            .ok_or_else(|| {
                VellumError::Unsupported(format!(
                    "Aucun chemin de conversion disponible de {src_upper} vers {dst_upper}"
                ))
            })?;

        if path.is_empty() {
            if source_file != target_file {
                std::fs::copy(source_file, target_file)?;
            }
            return Ok(target_file.to_path_buf());
        }

        if path.len() == 1 {
            let converter = &path[0];
            let plugin_id = converter.descriptor().id.clone();
            let mut config = self
                .plugin_service
                .get_plugin_config(&plugin_id)
                .unwrap_or_default();
            if let Some(overrides) = custom_config {
                for (k, v) in overrides {
                    config.insert(k.clone(), v.clone());
                }
            }
            return converter.convert(source_file, target_file, &config);
        }

        // Multi-step pipeline via pivot format(s)
        let mut current_input = source_file.to_path_buf();
        let total_steps = path.len();
        let mut temp_guards = Vec::new();

        for (idx, step) in path.iter().enumerate() {
            let is_last = idx == total_steps - 1;
            let plugin_id = step.descriptor().id.clone();
            let mut config = self
                .plugin_service
                .get_plugin_config(&plugin_id)
                .unwrap_or_default();
            if let Some(overrides) = custom_config {
                for (k, v) in overrides {
                    config.insert(k.clone(), v.clone());
                }
            }

            if is_last {
                step.convert(&current_input, target_file, &config)?;
            } else {
                let pivot_ext = step.target_format().to_lowercase();
                let temp_file = tempfile::Builder::new()
                    .prefix("vellum_pivot_")
                    .suffix(&format!(".{pivot_ext}"))
                    .tempfile()?;

                let next_input = temp_file.path().to_path_buf();
                temp_guards.push(temp_file);
                step.convert(&current_input, &next_input, &config)?;
                current_input = next_input;
            }
        }

        Ok(target_file.to_path_buf())
    }
}
