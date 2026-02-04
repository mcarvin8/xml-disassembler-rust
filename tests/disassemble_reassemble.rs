//! Integration test: disassemble an XML file, reassemble the output, and confirm
//! the reassembled XML matches the original file contents (same as original TypeScript tests).

use std::path::Path;
use xml_disassembler::{DisassembleXmlFileHandler, ReassembleXmlFileHandler};

#[tokio::test]
async fn disassemble_then_reassemble_matches_original_xml() {
    let _ = env_logger::try_init();

    let fixture = "fixtures/general/HR_Admin.permissionset-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture {} must exist (run from project root)",
        fixture
    );

    let original_content = std::fs::read_to_string(fixture).expect("read original fixture");

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let disassembled_dir = base.join("HR_Admin");

    // Disassemble: output goes to dirname(file_path). We copy the fixture to temp
    // and disassemble from there so output is temp/HR_Admin.
    let source_in_temp = base.join("HR_Admin.permissionset-meta.xml");
    std::fs::copy(fixture, &source_in_temp).expect("copy fixture to temp");

    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            source_in_temp.to_str().unwrap(),
            None,
            Some("unique-id"),
            false,
            false,
            ".xmldisassemblerignore",
            "xml",
        )
        .await
        .expect("disassemble");

    assert!(
        disassembled_dir.exists(),
        "Disassembled directory should exist"
    );

    let reassemble_handler = ReassembleXmlFileHandler::new();
    reassemble_handler
        .reassemble(
            disassembled_dir.to_str().unwrap(),
            Some("xml"),
            false,
        )
        .await
        .expect("reassemble");

    let reassembled_path = base.join("HR_Admin.xml");
    assert!(
        reassembled_path.exists(),
        "Reassembled file should exist"
    );

    let reassembled_content = std::fs::read_to_string(&reassembled_path).expect("read reassembled");

    assert_eq!(
        original_content,
        reassembled_content,
        "Reassembled XML must match original file contents (round-trip)"
    );
}
