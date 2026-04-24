//! XML Disassembler CLI - Disassemble large XML files into smaller files and reassemble.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();
    xml_disassembler::cli::run(args).await
}
