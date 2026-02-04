//! XML Disassembler CLI - Disassemble large XML files into smaller files and reassemble.

use std::env;
use xml_disassembler::{
    build_xml_string, parse_xml, DisassembleXmlFileHandler, ReassembleXmlFileHandler,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: xml-disassembler <command> [options]");
        eprintln!("  disassemble <path>              - Disassemble XML file or directory");
        eprintln!(
            "  reassemble <path> [extension]  - Reassemble directory (default extension: xml)"
        );
        eprintln!("  parse <path>                    - Parse and rebuild XML (test)");
        return Ok(());
    }

    let command = &args[1];
    let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");
    let extension = args.get(3).map(|s| s.as_str());

    match command.as_str() {
        "disassemble" => {
            let mut handler = DisassembleXmlFileHandler::new();
            handler
                .disassemble(
                    path,
                    None,
                    Some("unique-id"),
                    false,
                    false,
                    ".xmldisassemblerignore",
                    "xml",
                )
                .await?;
        }
        "reassemble" => {
            let handler = ReassembleXmlFileHandler::new();
            handler
                .reassemble(path, extension.or(Some("xml")), false)
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
