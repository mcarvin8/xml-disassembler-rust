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
        .reassemble(disassembled_dir.to_str().unwrap(), Some("xml"), false)
        .await
        .expect("reassemble");

    let reassembled_path = base.join("HR_Admin.xml");
    assert!(reassembled_path.exists(), "Reassembled file should exist");

    let reassembled_content = std::fs::read_to_string(&reassembled_path).expect("read reassembled");

    assert_eq!(
        original_content, reassembled_content,
        "Reassembled XML must match original file contents (round-trip)"
    );
}

#[tokio::test]
async fn cdata_preserved_round_trip() {
    let _ = env_logger::try_init();

    let fixture = "fixtures/cdata/VidLand_US.marketingappextension-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture {} must exist (run from project root)",
        fixture
    );

    let original_content = std::fs::read_to_string(fixture).expect("read original fixture");

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let disassembled_dir = base.join("VidLand_US");

    let source_in_temp = base.join("VidLand_US.marketingappextension-meta.xml");
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
            Some("marketingappextension-meta.xml"),
            false,
        )
        .await
        .expect("reassemble");

    let reassembled_path = base.join("VidLand_US.marketingappextension-meta.xml");
    assert!(reassembled_path.exists(), "Reassembled file should exist");

    let reassembled_content = std::fs::read_to_string(&reassembled_path).expect("read reassembled");

    // CDATA sections must be preserved (not escaped to &quot; etc.)
    assert!(
        reassembled_content.contains("<![CDATA["),
        "Reassembled XML must contain CDATA sections"
    );
    assert!(
        !reassembled_content.contains("&quot;"),
        "CDATA content must not be escaped (no &quot; in JSON)"
    );
    // Verify all CDATA content matches (whitespace may differ due to formatting)
    let extract_all_cdata = |s: &str| {
        let mut result = Vec::new();
        let mut rest = s;
        while let Some(start) = rest.find("<![CDATA[") {
            let cdata_start = start + 9;
            if let Some(end) = rest[cdata_start..].find("]]>") {
                result.push(rest[cdata_start..cdata_start + end].to_string());
                rest = &rest[cdata_start + end + 3..];
            } else {
                break;
            }
        }
        result
    };
    assert_eq!(
        extract_all_cdata(&reassembled_content),
        extract_all_cdata(&original_content),
        "CDATA content must match original"
    );
}
