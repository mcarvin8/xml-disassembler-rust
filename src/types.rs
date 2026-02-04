//! Type definitions for XML element representation.
//!
//! Uses serde_json::Value for flexible representation matching the TypeScript structure:
//! - Object with keys: element names, @attr for attributes, #text for text content, ?xml for declaration
//! - Values: string, nested object, or array of objects/strings

use serde_json::Value as JsonValue;

/// XmlElement is a flexible representation of XML - equivalent to TypeScript's XmlElement type.
/// Uses serde_json::Value for compatibility with quickxml_to_serde output.
pub type XmlElement = JsonValue;

/// Parameters for parsing an element during disassembly.
#[derive(Debug, Clone)]
pub struct XmlElementParams<'a> {
    pub element: XmlElement,
    pub disassembled_path: &'a str,
    pub unique_id_elements: Option<&'a str>,
    pub root_element_name: &'a str,
    pub root_attributes: XmlElement,
    pub key: &'a str,
    pub leaf_content: XmlElement,
    pub leaf_count: usize,
    pub has_nested_elements: bool,
    pub format: &'a str,
    pub xml_declaration: Option<XmlElement>,
    pub strategy: &'a str,
}

/// Options for building a single disassembled file.
#[derive(Debug, Clone)]
pub struct BuildDisassembledFileOptions<'a> {
    pub content: XmlElement,
    pub disassembled_path: &'a str,
    pub output_file_name: Option<&'a str>,
    pub subdirectory: Option<&'a str>,
    pub wrap_key: Option<&'a str>,
    pub is_grouped_array: bool,
    pub root_element_name: &'a str,
    pub root_attributes: XmlElement,
    pub format: &'a str,
    pub xml_declaration: Option<XmlElement>,
    pub unique_id_elements: Option<&'a str>,
}

/// Result from unified element parsing.
#[derive(Debug, Clone, Default)]
pub struct UnifiedParseResult {
    pub leaf_content: XmlElement,
    pub leaf_count: usize,
    pub has_nested_elements: bool,
    pub nested_groups: Option<XmlElementArrayMap>,
}

/// Map of tag name to array of elements.
pub type XmlElementArrayMap = std::collections::HashMap<String, Vec<XmlElement>>;

/// Options for building disassembled files from a source file.
#[derive(Debug, Clone)]
pub struct BuildDisassembledFilesOptions<'a> {
    pub file_path: &'a str,
    pub disassembled_path: &'a str,
    pub base_name: &'a str,
    pub post_purge: bool,
    pub format: &'a str,
    pub unique_id_elements: Option<&'a str>,
    pub strategy: &'a str,
}

/// Parameters for writing leaf content.
#[derive(Debug, Clone)]
pub struct LeafWriteParams<'a> {
    pub leaf_count: usize,
    pub leaf_content: XmlElement,
    pub strategy: &'a str,
    pub key_order: Vec<String>,
    pub options: LeafWriteOptions<'a>,
}

#[derive(Debug, Clone)]
pub struct LeafWriteOptions<'a> {
    pub disassembled_path: &'a str,
    pub output_file_name: &'a str,
    pub root_element_name: &'a str,
    pub root_attributes: XmlElement,
    pub xml_declaration: Option<XmlElement>,
    pub format: &'a str,
}
