# xml-disassembler

[![Crates.io](https://img.shields.io/crates/v/xml-disassembler.svg)](https://crates.io/crates/xml-disassembler)
[![Docs.rs](https://docs.rs/xml-disassembler/badge.svg)](https://docs.rs/xml-disassembler)
[![CI](https://github.com/mcarvin8/xml-disassembler-rust/workflows/CI/badge.svg)](https://github.com/mcarvin8/xml-disassembler-rust/actions)

Disassemble large XML files into smaller files and reassemble the original XML. Preserves the XML declaration, root namespace, and element order so that a full round-trip (disassemble → reassemble) reproduces the original file contents.

> **Note:** This is a Rust implementation of the original [TypeScript xml-disassembler](https://github.com/mcarvin8/xml-disassembler).

## Features

- **Disassemble** – Split a single XML file (or directory of XML files) into many smaller files, grouped by structure.
- **Reassemble** – Merge disassembled files back into the original XML. Uses the XML declaration and root attributes from the disassembled files, with sensible defaults when missing.
- **Round-trip safe** – Disassembled output includes the original XML declaration and `xmlns` on the root; reassembly preserves order and content so the result matches the source.
- **Library API** – Use `DisassembleXmlFileHandler`, `ReassembleXmlFileHandler`, `parse_xml`, and `build_xml_string` from your own Rust code.

## Installation

### From crates.io

1. Install the Rust toolchain ([rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)).
2. Run:
   ```bash
   cargo install xml-disassembler
   ```

### From source

```bash
git clone https://github.com/mcarvin8/xml-disassembler-rust.git
cd xml-disassembler-rust
cargo build --release
```

The binary will be at `target/release/xml-disassembler` (or `xml-disassembler.exe` on Windows).

## Usage

### CLI

```bash
# Disassemble an XML file or directory (output written alongside the source)
xml-disassembler disassemble <path> [options]

# Reassemble a disassembled directory (writes one XML file next to the directory)
xml-disassembler reassemble <path> [extension] [--postpurge]

# Parse and rebuild a single XML file (useful for testing the parser)
xml-disassembler parse <path>
```

#### Disassemble options

| Option | Description | Default |
|--------|-------------|---------|
| `--unique-id-elements <list>` | Comma-separated element names used to derive filenames for nested elements | (none) |
| `--prepurge` | Remove existing disassembly output before running | false |
| `--postpurge` | Delete original file/directory after disassembling | false |
| `--ignore-path <path>` | Path to the ignore file | .xmldisassemblerignore |
| `--format <fmt>` | Output format: xml, ini, json, json5, toml, yaml | xml |
| `--strategy <name>` | unique-id or grouped-by-tag | unique-id |

#### Reassemble options

| Option | Description | Default |
|--------|-------------|---------|
| `<extension>` | File extension/suffix for the rebuilt XML (e.g. permissionset-meta.xml) | xml |
| `--postpurge` | Delete disassembled directory after successful reassembly | false |

**Examples:**

```bash
xml-disassembler disassemble fixtures/general/HR_Admin.permissionset-meta.xml
# Creates fixtures/general/HR_Admin/ with disassembled files

xml-disassembler disassemble ./my.xml --format yaml --strategy grouped-by-tag --prepurge
xml-disassembler disassemble ./my.xml --unique-id-elements "name,id" --postpurge

xml-disassembler reassemble fixtures/general/HR_Admin
# Creates fixtures/general/HR_Admin.xml

xml-disassembler reassemble fixtures/general/HR_Admin permissionset-meta.xml --postpurge
```

### As a library

```rust
use xml_disassembler::{DisassembleXmlFileHandler, ReassembleXmlFileHandler, parse_xml, build_xml_string};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Disassemble
    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble("path/to/file.xml", None, Some("unique-id"), false, false, ".xmldisassemblerignore", "xml")
        .await?;

    // Reassemble
    let reassemble = ReassembleXmlFileHandler::new();
    reassemble.reassemble("path/to/disassembled_dir", Some("xml"), false).await?;

    // Parse and rebuild a single file
    if let Some(parsed) = parse_xml("path/to/file.xml").await {
        let xml = build_xml_string(&parsed);
        println!("{}", xml);
    }
    Ok(())
}
```

## Testing

Run all tests:

```bash
cargo test
```

- **Unit tests** – In-module tests for parsers, builders, and merge logic (e.g. `strip_whitespace`, `merge_xml_elements`, `extract_root_attributes`, `parse_xml`).
- **Integration test** – `tests/disassemble_reassemble.rs` runs a full round-trip: disassemble a fixture XML, reassemble it, and assert the reassembled content equals the original file.

## License

Licensed under ([MIT](LICENSE.md)).

## Contribution

See [CONTRIBUTING.md](CONTRIBUTING.md).
