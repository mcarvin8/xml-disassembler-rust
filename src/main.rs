//! XML Disassembler CLI - Disassemble large XML files into smaller files and reassemble.

use std::env;
use xml_disassembler::{
    build_xml_string, parse_xml, DisassembleXmlFileHandler, ReassembleXmlFileHandler,
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
        eprintln!("    --format <fmt>                 - Output format: xml, ini, json, json5, toml, yaml (default: xml)");
        eprintln!(
            "    --strategy <name>              - unique-id or grouped-by-tag (default: unique-id)"
        );
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
            let mut handler = DisassembleXmlFileHandler::new();
            handler
                .disassemble(
                    path,
                    opts.unique_id_elements,
                    opts.strategy.or(Some("unique-id")),
                    opts.pre_purge,
                    opts.post_purge,
                    opts.ignore_path,
                    opts.format,
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
