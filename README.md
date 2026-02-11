# xml-disassembler

[![Crates.io](https://img.shields.io/crates/v/xml-disassembler.svg)](https://crates.io/crates/xml-disassembler)
[![Docs.rs](https://docs.rs/xml-disassembler/badge.svg)](https://docs.rs/xml-disassembler)
[![CI](https://github.com/mcarvin8/xml-disassembler-rust/workflows/CI/badge.svg)](https://github.com/mcarvin8/xml-disassembler-rust/actions)

Disassemble large XML files into smaller files and reassemble the original XML. Preserves the XML declaration, root namespace, and element order so that a full round-trip (disassemble → reassemble) reproduces the original file contents.

> **Note:** This is a Rust implementation of the original [TypeScript xml-disassembler](https://github.com/mcarvin8/xml-disassembler).

---

## Table of contents

- [Quick start](#quick-start)
- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [As a library](#as-a-library)
- [Disassembly strategies](#disassembly-strategies)
- [Ignore file](#ignore-file)
- [Logging](#logging)
- [XML parser](#xml-parser)
- [Testing](#testing)
- [License](#license)
- [Contribution](#contribution)

---

## Quick start

```bash
# Disassemble: one XML → many small files
xml-disassembler disassemble path/to/YourFile.permissionset-meta.xml \
  --unique-id-elements "application,apexClass,name,flow,object,recordType,tab,field" \
  --format json \
  --strategy unique-id

# Reassemble: many small files → one XML
xml-disassembler reassemble path/to/YourFile permissionset-meta.xml
```

---

## Features

- **Disassemble** – Split a single XML file (or directory of XML files) into many smaller files, grouped by structure.
- **Reassemble** – Merge disassembled files back into the original XML. Uses the XML declaration and root attributes from the disassembled files, with sensible defaults when missing.
- **Multiple formats** – Output (and reassemble from) XML, JSON, JSON5, or YAML.
- **Strategies** – `unique-id` (one file per nested element) or `grouped-by-tag` (one file per tag).
- **Ignore rules** – Exclude paths via a `.xmldisassemblerignore` file (same style as `.gitignore`).
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
| `--format <fmt>` | Output format: xml, json, json5, yaml | xml |
| `--strategy <name>` | unique-id or grouped-by-tag | unique-id |
| `-p`, `--split-tags <spec>` | With grouped-by-tag: split or group nested tags into subdirs (e.g. `objectPermissions:split:object,fieldPermissions:group:field`) | (none) |
| `--multi-level <spec>` | Further disassemble matching files: `file_pattern:root_to_strip:unique_id_elements` | (none) |

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

## Disassembly strategies

### unique-id (default)

Each nested element is written to its own file, named by a unique identifier (or an 8-character SHA-256 hash if no UID is available). Leaf content stays in a file named after the original XML.

Best for fine-grained diffs and version control.

- **UID-based layout** – When you provide `--unique-id-elements` (e.g. `name,id,apexClass`), nested elements are named by the first matching field value. For Salesforce flows, a typical list might be: `apexClass,name,object,field,layout,actionName,targetReference,assignToReference,choiceText,promptText`. Using unique ID elements also ensures predictable sorting in the reassembled output.
- **Hash-based layout** – When no unique ID is found, elements are named with an 8-character hash of their content (e.g. `419e0199.botMlDomain-meta.xml`).

### grouped-by-tag

All nested elements with the same tag go into one file per tag. Leaf content stays in the base file named after the original XML.

Best for fewer files and quick inspection.

```bash
xml-disassembler disassemble ./my.xml --strategy grouped-by-tag --format yaml
```

Reassembly preserves element content and structure.

#### Split tags (`-p` / `--split-tags`)

With `--strategy grouped-by-tag`, you can optionally **split** or **group** specific nested tags into subdirectories instead of a single file per tag. Useful for permission sets and similar metadata: e.g. one file per `objectPermissions` under `objectPermissions/`, and `fieldPermissions` grouped by object under `fieldPermissions/`.

Spec: comma-separated rules. Each rule is `tag:mode:field` or `tag:path:mode:field` (path defaults to tag). **mode** is `split` (one file per array item, filename from `field`) or `group` (group array items by `field`, one file per group).

```bash
# Permission set: objectPermissions → one file per object; fieldPermissions → one file per field value
xml-disassembler disassemble fixtures/split-tags/HR_Admin.permissionset-meta.xml \
  --strategy grouped-by-tag \
  -p "objectPermissions:split:object,fieldPermissions:group:field"
```

Creates `HR_Admin/` with e.g. `objectPermissions/Job_Request__c.objectPermissions-meta.xml`, `objectPermissions/Account.objectPermissions-meta.xml`, `fieldPermissions/<fieldValue>.fieldPermissions-meta.xml`, plus the main `HR_Admin.permissionset-meta.xml` with the rest. Reassembly requires no changes: the existing reassemble command merges subdirs and files back into one XML.

### Multi-level disassembly

For advanced use cases (e.g. Salesforce Loyalty Program Setup metadata), you can further disassemble specific output files by stripping a root element and re-running disassembly with different unique-id elements.

Use `--multi-level <spec>` where the spec is:

`file_pattern:root_to_strip:unique_id_elements`

- **file_pattern** – Match XML files whose name or path contains this (e.g. `programProcesses` or `programProcesses-meta`).
- **root_to_strip** – Element to strip/unwrap: if it is the root, its inner content becomes the new document; if it is a child (e.g. `programProcesses` under `LoyaltyProgramSetup`), it is unwrapped so its inner content becomes the root’s direct children.
- **unique_id_elements** – Comma-separated element names for the second-level disassembly (e.g. `parameterName,ruleName`).

Example (loyalty program): strip the child `programProcesses` in each process file so parameters/rules can be disassembled:

```bash
xml-disassembler disassemble ./Cloud_Kicks_Inner_Circle.loyaltyProgramSetup-meta.xml \
  --unique-id-elements "fullName,name,processName" \
  --multi-level "programProcesses:programProcesses:parameterName,ruleName"
```

A `.multi_level.json` config is written in the disassembly root so **reassemble** automatically does inner-level reassembly first, wraps files with the original root, then reassembles the top level. No extra flags are needed for reassembly.

**Caveat:** Multi-level reassembly removes disassembled directories after reassembling each level, even when you do not pass `--postpurge`. This is required so the next level can merge the reassembled XML files. Use version control (e.g. Git) to recover the tree if needed, or run reassembly only in a pipeline where these changes can be discarded.

## Ignore file

Exclude files or directories from disassembly using an ignore file (default: `.xmldisassemblerignore`). The Rust implementation uses the [ignore](https://crates.io/crates/ignore) crate with `.gitignore`-style syntax.

Place the file in the directory you run disassembly from (or specify a path with `--ignore-path`).

Example `.xmldisassemblerignore`:

```
# Skip these paths
**/secret.xml
**/generated/
```

## Logging

Logging uses the [log](https://crates.io/crates/log) crate with [env_logger](https://crates.io/crates/env_logger). Control verbosity via the `RUST_LOG` environment variable.

```bash
# Default: only errors
xml-disassembler disassemble ./my.xml

# Verbose logging (debug level)
RUST_LOG=debug xml-disassembler disassemble ./my.xml

# Log only xml_disassembler crate
RUST_LOG=xml_disassembler=debug xml-disassembler disassemble ./my.xml
```

When using the library, call `env_logger::init()` early in your binary (as in the CLI) and set `RUST_LOG` as needed.

## XML parser

Parsing is done with [quick-xml](https://github.com/tafia/quick-xml), with support for:

- **CDATA** – Preserved and output as `#cdata` in the parsed structure.
- **Comments** – Preserved in the XML output.
- **Attributes** – Stored with `@` prefix (e.g. `@version`, `@encoding`).

## Testing

Run all tests:

```bash
cargo test
```

- **Unit tests** – In-module tests for parsers, builders, and merge logic (e.g. `strip_whitespace`, `merge_xml_elements`, `extract_root_attributes`, `parse_xml`).
- **Integration test** – `tests/disassemble_reassemble.rs` runs a full round-trip: disassemble a fixture XML, reassemble it, and assert the reassembled content equals the original file.

## License

Licensed under [MIT](LICENSE.md).

## Contribution

See [CONTRIBUTING.md](CONTRIBUTING.md).
