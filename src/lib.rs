//! XML Disassembler - Disassemble large XML files into smaller files and reassemble the original XML.

pub mod builders;
pub mod constants;
pub mod handlers;
pub mod parsers;
pub mod transformers;
pub mod types;
pub mod utils;

pub use builders::build_xml_string;
pub use handlers::{DisassembleXmlFileHandler, ReassembleXmlFileHandler};
pub use parsers::parse_xml;
pub use transformers::{
    transform_to_ini, transform_to_json, transform_to_json5, transform_to_toml, transform_to_yaml,
};
pub use types::XmlElement;
