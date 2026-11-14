#![allow(clippy::unwrap_used)]

use serde_json::json;
use std::collections::HashMap;
use vellum_domain::{Book, BookFormat, ConfigField, Param, PluginDescriptor, PluginType, Tag};

#[test]
fn test_book_entity_and_formats() {
    let mut book = Book::new("L'Étranger", "Albert Camus");
    assert_eq!(book.title, "L'Étranger");
    assert_eq!(book.author, "Albert Camus");
    assert_eq!(book.progress_percent, 0.0);
    assert!(!book.is_finished);

    let fmt = BookFormat::new("epub", "etranger.epub", "Camus/etranger.epub", 120_000);
    book.add_format(fmt);
    assert_eq!(book.formats.len(), 1);
    assert_eq!(book.formats[0].book_id.as_deref(), Some(book.id.as_str()));

    book.add_tag(Tag::new("Classique"));
    book.add_tag(Tag::new("Classique")); // duplicate ignored
    assert_eq!(book.tags.len(), 1);
}

#[test]
fn test_param_serialization() {
    let mut param = Param::new("plugin:test:config", "test", "config", "{}");
    let mut data = HashMap::new();
    data.insert("key".to_string(), json!("value"));
    param.set_data(&data);

    let parsed = param.get_data();
    assert_eq!(parsed.get("key"), Some(&json!("value")));
}

#[test]
fn test_plugin_descriptor_deserialization() {
    let raw = r#"{
        "id": "custom-script",
        "name": "Custom Script",
        "description": "Test",
        "version": "1.0.0",
        "type": "transformation",
        "command": "sh run.sh",
        "configSchema": [
            {
                "key": "threshold",
                "label": "Seuil",
                "type": "slider",
                "defaultValue": 0.5,
                "min": 0.0,
                "max": 1.0,
                "step": 0.05
            }
        ]
    }"#;

    let desc: PluginDescriptor = serde_json::from_str(raw).expect("valid plugin.json");
    assert_eq!(desc.id, "custom-script");
    assert_eq!(desc.plugin_type, PluginType::Transformation);
    assert_eq!(desc.config_schema.len(), 1);
    let field: &ConfigField = &desc.config_schema[0];
    assert_eq!(field.min, Some(0.0));
    assert_eq!(field.max, Some(1.0));
}
