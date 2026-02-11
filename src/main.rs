//! XML Disassembler CLI - Disassemble large XML files into smaller files and reassemble.

use std::env;
use xml_disassembler::{
    build_xml_string, parse_xml, DecomposeRule, DisassembleXmlFileHandler, MultiLevelRule,
    ReassembleXmlFileHandler,
};

/// Options parsed from disassemble CLI args.
struct DisassembleOpts<'a> {
    path: Option<&'a str>,
    unique_id_elements: Option<&'a str>,
    pre_purge: bool,
    post_purge: bool,
    ignore_path: &'a str,
    format: &'a str,
    strategy: Option<&'a str>,
    multi_level: Option<String>,
    split_tags: Option<String>,
}

/// Parse --split-tags spec for grouped-by-tag. Comma-separated rules; each rule:
/// tag:mode:field (path_segment defaults to tag) or tag:path:mode:field.
/// mode = "split" (one file per item, filename from field) or "group" (group by field).
/// e.g. "objectPermissions:split:object,fieldPermissions:group:object"
fn parse_decompose_spec(spec: &str) -> Vec<DecomposeRule> {
    let mut rules = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        let segments: Vec<&str> = part.splitn(4, ':').collect();
        if segments.len() >= 3 {
            let tag = segments[0].to_string();
            let (path_segment, mode, field) = if segments.len() == 3 {
                (
                    tag.clone(),
                    segments[1].to_string(),
                    segments[2].to_string(),
                )
            } else {
                (
                    segments[1].to_string(),
                    segments[2].to_string(),
                    segments[3].to_string(),
                )
            };
            if !tag.is_empty() && !mode.is_empty() && !field.is_empty() {
                rules.push(DecomposeRule {
                    tag,
                    path_segment,
                    mode,
                    field,
                });
            }
        }
    }
    rules
}

/// Parse --multi-level spec: "file_pattern:root_to_strip:unique_id_elements"
/// e.g. "programProcesses-meta:LoyaltyProgramSetup:parameterName,ruleName"
fn parse_multi_level_spec(spec: &str) -> Option<MultiLevelRule> {
    let parts: Vec<&str> = spec.splitn(3, ':').collect();
    if parts.len() != 3 {
        return None;
    }
    let (file_pattern, root_to_strip, unique_id_elements) = (parts[0], parts[1], parts[2]);
    if file_pattern.is_empty() || root_to_strip.is_empty() || unique_id_elements.is_empty() {
        return None;
    }
    let path_segment = xml_disassembler::path_segment_from_file_pattern(file_pattern);
    Some(MultiLevelRule {
        file_pattern: file_pattern.to_string(),
        root_to_strip: root_to_strip.to_string(),
        unique_id_elements: unique_id_elements.to_string(),
        path_segment: path_segment.clone(),
        wrap_root_element: root_to_strip.to_string(),
        wrap_xmlns: String::new(),
    })
}

/// Parse disassemble args: <path> [options].
/// Options: --postpurge, --prepurge, --unique-id-elements=<val>, --ignore-path=<val>, --format=<val>, --strategy=<val>
fn parse_disassemble_args(args: &[String]) -> DisassembleOpts<'_> {
    let mut path = None;
    let mut unique_id_elements = None;
    let mut pre_purge = false;
    let mut post_purge = false;
    let mut ignore_path = ".xmldisassemblerignore";
    let mut format = "xml";
    let mut strategy = None;
    let mut multi_level = None;
    let mut split_tags = None;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--postpurge" {
            post_purge = true;
            i += 1;
        } else if arg == "--prepurge" {
            pre_purge = true;
            i += 1;
        } else if arg.starts_with("--unique-id-elements=") {
            unique_id_elements = Some(arg.trim_start_matches("--unique-id-elements="));
            i += 1;
        } else if arg == "--unique-id-elements" {
            i += 1;
            if i < args.len() {
                unique_id_elements = Some(args[i].as_str());
                i += 1;
            }
        } else if arg.starts_with("--ignore-path=") {
            ignore_path = arg.trim_start_matches("--ignore-path=");
            i += 1;
        } else if arg == "--ignore-path" {
            i += 1;
            if i < args.len() {
                ignore_path = args[i].as_str();
                i += 1;
            }
        } else if arg.starts_with("--format=") {
            format = arg.trim_start_matches("--format=");
            i += 1;
        } else if arg == "--format" {
            i += 1;
            if i < args.len() {
                format = args[i].as_str();
                i += 1;
            }
        } else if arg.starts_with("--strategy=") {
            strategy = Some(arg.trim_start_matches("--strategy="));
            i += 1;
        } else if arg == "--strategy" {
            i += 1;
            if i < args.len() {
                strategy = Some(args[i].as_str());
                i += 1;
            }
        } else if arg.starts_with("--multi-level=") {
            multi_level = Some(arg.trim_start_matches("--multi-level=").to_string());
            i += 1;
        } else if arg == "--multi-level" {
            i += 1;
            if i < args.len() {
                multi_level = Some(args[i].clone());
                i += 1;
            }
        } else if arg.starts_with("--split-tags=") {
            split_tags = Some(arg.trim_start_matches("--split-tags=").to_string());
            i += 1;
        } else if arg == "--split-tags" || arg == "-p" {
            i += 1;
            if i < args.len() {
                split_tags = Some(args[i].clone());
                i += 1;
            }
        } else if arg.starts_with("--") {
            i += 1;
        } else if path.is_none() {
            path = Some(arg.as_str());
            i += 1;
        } else {
            i += 1;
        }
    }

    DisassembleOpts {
        path,
        unique_id_elements,
        pre_purge,
        post_purge,
        ignore_path,
        format,
        strategy,
        multi_level,
        split_tags,
    }
}

/// Parse reassemble args: <path> [extension] [--postpurge].
/// Returns (path, extension, post_purge). path is None if no positional arg given.
fn parse_reassemble_args(args: &[String]) -> (Option<&str>, Option<&str>, bool) {
    let mut path = None;
    let mut extension = None;
    let mut post_purge = false;
    for arg in args {
        if arg == "--postpurge" {
            post_purge = true;
        } else if path.is_none() {
            path = Some(arg.as_str());
        } else if extension.is_none() {
            extension = Some(arg.as_str());
        }
    }
    (path, extension, post_purge)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: xml-disassembler <command> [options]");
        eprintln!("  disassemble <path> [options]     - Disassemble XML file or directory");
        eprintln!("    --postpurge                    - Delete original file/dir after disassembling (default: false)");
        eprintln!("    --prepurge                     - Remove existing disassembly output before running (default: false)");
        eprintln!("    --unique-id-elements <list>    - Comma-separated element names for nested filenames");
        eprintln!("    --ignore-path <path>           - Path to ignore file (default: .xmldisassemblerignore)");
        eprintln!("    --format <fmt>                 - Output format: xml, json, json5, yaml (default: xml)");
        eprintln!(
            "    --strategy <name>              - unique-id or grouped-by-tag (default: unique-id)"
        );
        eprintln!("    --multi-level <spec>          - Further disassemble matching files: file_pattern:root_to_strip:unique_id_elements");
        eprintln!("    -p, --split-tags <spec>       - With grouped-by-tag: split/group nested tags (e.g. objectPermissions:split:object,fieldPermissions:group:field)");
        eprintln!("  reassemble <path> [extension] [--postpurge]  - Reassemble directory (default extension: xml)");
        eprintln!("  parse <path>                    - Parse and rebuild XML (test)");
        return Ok(());
    }

    let command = &args[1];
    let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");

    match command.as_str() {
        "disassemble" => {
            let opts = parse_disassemble_args(&args[2..]);
            let path = opts.path.unwrap_or(".");
            let strategy = opts.strategy.unwrap_or("unique-id");
            let multi_level_rule = opts
                .multi_level
                .as_ref()
                .and_then(|s| parse_multi_level_spec(s));
            if opts.multi_level.is_some() && multi_level_rule.is_none() {
                eprintln!(
                    "Invalid --multi-level spec; use file_pattern:root_to_strip:unique_id_elements"
                );
            }
            let decompose_rules: Vec<DecomposeRule> = if strategy == "grouped-by-tag" {
                opts.split_tags
                    .as_ref()
                    .map(|s| parse_decompose_spec(s))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let decompose_rules_ref = if decompose_rules.is_empty() {
                None
            } else {
                Some(decompose_rules.as_slice())
            };
            let mut handler = DisassembleXmlFileHandler::new();
            handler
                .disassemble(
                    path,
                    opts.unique_id_elements,
                    Some(strategy),
                    opts.pre_purge,
                    opts.post_purge,
                    opts.ignore_path,
                    opts.format,
                    multi_level_rule.as_ref(),
                    decompose_rules_ref,
                )
                .await?;
        }
        "reassemble" => {
            let (path, extension, post_purge) = parse_reassemble_args(&args[2..]);
            let path = path.unwrap_or(".");
            let handler = ReassembleXmlFileHandler::new();
            handler
                .reassemble(path, extension.or(Some("xml")), post_purge)
                .await?;
        }
        "parse" => {
            if let Some(parsed) = parse_xml(path).await {
                let xml = build_xml_string(&parsed);
                println!("{}", xml);
            } else {
                eprintln!("Failed to parse {}", path);
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
        }
    }

    Ok(())
}
