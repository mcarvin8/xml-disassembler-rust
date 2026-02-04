//! XML Disassembler CLI - Disassemble large XML files into smaller files and reassemble.

use xml_disassembler::{parse_xml, build_xml_string, DisassembleXmlFileHandler, ReassembleXmlFileHandler};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: xml-disassembler <command> [options]");
        eprintln!("  disassemble <path>  - Disassemble XML file or directory");
        eprintln!("  reassemble <path>   - Reassemble disassembled directory");
        eprintln!("  parse <path>        - Parse and rebuild XML (test)");
        return Ok(());
    }

    let command = &args[1];
    let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");

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
            handler.reassemble(path, Some("xml"), false).await?;
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
