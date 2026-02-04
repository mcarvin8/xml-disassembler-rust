//! XML Disassembler CLI - Disassemble large XML files into smaller files and reassemble.

use std::env;
use xml_disassembler::{
    build_xml_string, parse_xml, DisassembleXmlFileHandler, ReassembleXmlFileHandler,
};

/// Parse disassemble args: <path> [--postpurge].
/// Returns (path, post_purge). path is None if no positional arg given.
fn parse_disassemble_args(args: &[String]) -> (Option<&str>, bool) {
    let mut path = None;
    let mut post_purge = false;
    for arg in args {
        if arg == "--postpurge" {
            post_purge = true;
        } else if path.is_none() {
            path = Some(arg.as_str());
        }
    }
    (path, post_purge)
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
        eprintln!("  disassemble <path> [--postpurge] - Disassemble XML file or directory (--postpurge: delete original after)");
        eprintln!("  reassemble <path> [extension] [--postpurge]  - Reassemble directory (default extension: xml)");
        eprintln!("  parse <path>                    - Parse and rebuild XML (test)");
        return Ok(());
    }

    let command = &args[1];
    let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");

    match command.as_str() {
        "disassemble" => {
            let (path, post_purge) = parse_disassemble_args(&args[2..]);
            let path = path.unwrap_or(".");
            let mut handler = DisassembleXmlFileHandler::new();
            handler
                .disassemble(
                    path,
                    None,
                    Some("unique-id"),
                    false,
                    post_purge,
                    ".xmldisassemblerignore",
                    "xml",
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
