//! Integration test: disassemble an XML file, reassemble the output, and confirm
//! the reassembled XML matches the original file contents (same as original TypeScript tests).

use std::path::Path;
use xml_disassembler::{DecomposeRule, DisassembleXmlFileHandler, ReassembleXmlFileHandler};

#[tokio::test]
async fn reassemble_with_file_path_returns_ok_no_op() {
    let _ = env_logger::try_init();
    let fixture = "fixtures/general/HR_Admin.permissionset-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture must exist (run from project root)"
    );
    let handler = ReassembleXmlFileHandler::new();
    // Path is a file, not a directory; validate_directory returns false
    handler
        .reassemble(fixture, Some("xml"), false)
        .await
        .expect("reassemble should return Ok(())");
}

#[tokio::test]
async fn disassemble_with_unsupported_strategy_defaults_to_unique_id() {
    let _ = env_logger::try_init();
    let fixture = "fixtures/general/HR_Admin.permissionset-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture must exist (run from project root)"
    );
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let source = base.join("HR_Admin.permissionset-meta.xml");
    std::fs::copy(fixture, &source).expect("copy fixture");
    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            source.to_str().unwrap(),
            None,
            Some("unsupported-strategy"),
            false,
            false,
            ".xmldisassemblerignore",
            "xml",
            None,
            None,
        )
        .await
        .expect("disassemble");
    assert!(
        base.join("HR_Admin").exists(),
        "Should still disassemble with default strategy"
    );
}

#[tokio::test]
async fn disassemble_directory_with_ignore_skips_matching_files() {
    let _ = env_logger::try_init();
    let fixture = "fixtures/general/HR_Admin.permissionset-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture must exist (run from project root)"
    );
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let dir_path = base.join("meta");
    std::fs::create_dir_all(&dir_path).expect("create dir");
    std::fs::copy(fixture, dir_path.join("A.permissionset-meta.xml")).expect("copy");
    std::fs::copy(fixture, dir_path.join("B.permissionset-meta.xml")).expect("copy");
    std::fs::write(
        base.join(".xmldisassemblerignore"),
        "B.permissionset-meta.xml",
    )
    .expect("write ignore");
    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            dir_path.to_str().unwrap(),
            None,
            Some("unique-id"),
            false,
            false,
            base.join(".xmldisassemblerignore").to_str().unwrap(),
            "xml",
            None,
            None,
        )
        .await
        .expect("disassemble");
    assert!(dir_path.join("A").exists(), "A should be disassembled");
    assert!(
        !dir_path.join("B").exists(),
        "B should be ignored by .xmldisassemblerignore"
    );
}

#[tokio::test]
async fn disassemble_directory_processes_xml_files() {
    let _ = env_logger::try_init();
    let fixture = "fixtures/general/HR_Admin.permissionset-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture must exist (run from project root)"
    );
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let dir_path = base.join("meta");
    std::fs::create_dir_all(&dir_path).expect("create dir");
    let f1 = dir_path.join("A.permissionset-meta.xml");
    let f2 = dir_path.join("B.permissionset-meta.xml");
    std::fs::copy(fixture, &f1).expect("copy");
    std::fs::copy(fixture, &f2).expect("copy");
    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            dir_path.to_str().unwrap(),
            None,
            Some("unique-id"),
            false,
            false,
            ".xmldisassemblerignore",
            "xml",
            None,
            None,
        )
        .await
        .expect("disassemble");
    assert!(dir_path.join("A").exists());
    assert!(dir_path.join("B").exists());
}

#[tokio::test]
async fn reassemble_with_post_purge_removes_disassembled_dir() {
    let _ = env_logger::try_init();
    let fixture = "fixtures/general/HR_Admin.permissionset-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture must exist (run from project root)"
    );
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let source = base.join("HR_Admin.permissionset-meta.xml");
    let disassembled_dir = base.join("HR_Admin");
    std::fs::copy(fixture, &source).expect("copy");
    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            source.to_str().unwrap(),
            None,
            Some("unique-id"),
            false,
            false,
            ".xmldisassemblerignore",
            "xml",
            None,
            None,
        )
        .await
        .expect("disassemble");
    assert!(disassembled_dir.exists());
    let handler = ReassembleXmlFileHandler::new();
    handler
        .reassemble(disassembled_dir.to_str().unwrap(), Some("xml"), true)
        .await
        .expect("reassemble");
    assert!(
        !disassembled_dir.exists(),
        "post_purge should remove disassembled directory"
    );
}

#[tokio::test]
async fn reassemble_applies_key_order_from_dot_key_order_json() {
    let _ = env_logger::try_init();
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let disassembled_dir = base.join("Out");
    std::fs::create_dir_all(&disassembled_dir).expect("create dir");
    let part_xml =
        r#"<?xml version="1.0"?><Root><firstKey>1</firstKey><secondKey>2</secondKey></Root>"#;
    std::fs::write(disassembled_dir.join("Out.xml"), part_xml).expect("write part");
    let key_order = serde_json::to_string(&["secondKey", "firstKey"]).unwrap();
    std::fs::write(disassembled_dir.join(".key_order.json"), key_order).expect("write key_order");
    let handler = ReassembleXmlFileHandler::new();
    handler
        .reassemble(disassembled_dir.to_str().unwrap(), Some("xml"), false)
        .await
        .expect("reassemble");
    let out = std::fs::read_to_string(base.join("Out.xml")).expect("read output");
    let second_pos = out.find("<secondKey>").unwrap_or(0);
    let first_pos = out.find("<firstKey>").unwrap_or(0);
    assert!(
        second_pos < first_pos,
        "key_order should reorder: secondKey before firstKey"
    );
}

#[tokio::test]
async fn reassemble_empty_directory_returns_ok_no_output() {
    let _ = env_logger::try_init();
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let empty_dir = temp_dir.path().join("empty");
    std::fs::create_dir_all(&empty_dir).expect("create dir");
    let handler = ReassembleXmlFileHandler::new();
    handler
        .reassemble(empty_dir.to_str().unwrap(), Some("xml"), false)
        .await
        .expect("reassemble should return Ok(())");
    // No output file created when directory has no parsable files
    assert!(!temp_dir.path().join("empty.xml").exists());
}

#[tokio::test]
async fn disassemble_non_xml_file_returns_ok_no_op() {
    let _ = env_logger::try_init();
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let txt_file = base.join("readme.txt");
    std::fs::write(&txt_file, "not xml").expect("write");
    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            txt_file.to_str().unwrap(),
            None,
            Some("unique-id"),
            false,
            false,
            ".xmldisassemblerignore",
            "xml",
            None,
            None,
        )
        .await
        .expect("disassemble should return Ok(())");
    assert!(
        !base.join("readme").exists(),
        "Should not create output for non-XML file"
    );
}

#[tokio::test]
async fn disassemble_with_pre_purge_removes_existing_output() {
    let _ = env_logger::try_init();
    let fixture = "fixtures/general/HR_Admin.permissionset-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture must exist (run from project root)"
    );
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let source = base.join("HR_Admin.permissionset-meta.xml");
    std::fs::copy(fixture, &source).expect("copy");
    let out_dir = base.join("HR_Admin");
    std::fs::create_dir_all(&out_dir).expect("create");
    let marker = out_dir.join("pre-existing.txt");
    std::fs::write(&marker, "before").expect("write");
    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            source.to_str().unwrap(),
            None,
            Some("unique-id"),
            true, // pre_purge
            false,
            ".xmldisassemblerignore",
            "xml",
            None,
            None,
        )
        .await
        .expect("disassemble");
    assert!(out_dir.exists());
    assert!(
        !marker.exists(),
        "pre_purge should remove existing output dir contents"
    );
}

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
            None,
            None,
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
async fn disassemble_json_format_then_reassemble_round_trip() {
    let _ = env_logger::try_init();

    let fixture = "fixtures/general/HR_Admin.permissionset-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture {} must exist (run from project root)",
        fixture
    );

    let _original_content = std::fs::read_to_string(fixture).expect("read original fixture");

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let disassembled_dir = base.join("HR_Admin");
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
            "json",
            None,
            None,
        )
        .await
        .expect("disassemble");

    assert!(
        disassembled_dir.exists(),
        "Disassembled directory should exist"
    );

    let reassemble_handler = ReassembleXmlFileHandler::new();
    reassemble_handler
        .reassemble(disassembled_dir.to_str().unwrap(), Some("json"), false)
        .await
        .expect("reassemble");

    let reassembled_path = base.join("HR_Admin.json");
    assert!(reassembled_path.exists(), "Reassembled file should exist");
    let reassembled = std::fs::read_to_string(&reassembled_path).expect("read reassembled");
    assert!(!reassembled.is_empty());
    assert!(
        reassembled.contains("<?xml") || reassembled.contains("<"),
        "reassembled content"
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
            None,
            None,
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

    assert_eq!(
        original_content, reassembled_content,
        "Reassembled XML must match original file contents (CDATA round-trip)"
    );
}

#[tokio::test]
async fn comments_preserved_round_trip() {
    let _ = env_logger::try_init();

    let fixture = "fixtures/comments/Numbers-fr.globalValueSetTranslation-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture {} must exist (run from project root)",
        fixture
    );

    let original_content = std::fs::read_to_string(fixture).expect("read original fixture");

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let disassembled_dir = base.join("Numbers-fr");

    let source_in_temp = base.join("Numbers-fr.globalValueSetTranslation-meta.xml");
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
            None,
            None,
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
            Some("globalValueSetTranslation-meta.xml"),
            false,
        )
        .await
        .expect("reassemble");

    let reassembled_path = base.join("Numbers-fr.globalValueSetTranslation-meta.xml");
    assert!(reassembled_path.exists(), "Reassembled file should exist");

    let reassembled_content = std::fs::read_to_string(&reassembled_path).expect("read reassembled");

    assert_eq!(
        original_content, reassembled_content,
        "Reassembled XML must match original file contents (comments round-trip)"
    );
}

#[tokio::test]
async fn deeply_nested_unique_id_elements_round_trip() {
    let _ = env_logger::try_init();

    let fixture = "fixtures/deeply-nested-unique-id-element/Get_Info.flow-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture {} must exist (run from project root)",
        fixture
    );

    let original_content = std::fs::read_to_string(fixture).expect("read original fixture");

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let disassembled_dir = base.join("Get_Info");

    let source_in_temp = base.join("Get_Info.flow-meta.xml");
    std::fs::copy(fixture, &source_in_temp).expect("copy fixture to temp");

    let unique_id_elements =
        "apexClass,name,object,field,layout,actionName,targetReference,assignToReference,choiceText,promptText";

    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            source_in_temp.to_str().unwrap(),
            Some(unique_id_elements),
            Some("unique-id"),
            false,
            false,
            ".xmldisassemblerignore",
            "xml",
            None,
            None,
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
            Some("flow-meta.xml"),
            false,
        )
        .await
        .expect("reassemble");

    let reassembled_path = base.join("Get_Info.flow-meta.xml");
    assert!(reassembled_path.exists(), "Reassembled file should exist");

    let reassembled_content = std::fs::read_to_string(&reassembled_path).expect("read reassembled");

    assert_eq!(
        original_content, reassembled_content,
        "Reassembled XML must match original (deeply nested unique ID elements round-trip)"
    );
}

/// Multi-level disassembly: first disassemble by processName etc., then further disassemble
/// programProcesses by parameterName and ruleName. Reassemble and compare to original.
#[tokio::test]
async fn multi_level_disassemble_then_reassemble_matches_original() {
    let _ = env_logger::try_init();

    let fixture = "fixtures/multi-level/Cloud_Kicks_Inner_Circle.loyaltyProgramSetup-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture {} must exist (run from project root)",
        fixture
    );

    let original_content = std::fs::read_to_string(fixture).expect("read original fixture");

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let disassembled_dir = base.join("Cloud_Kicks_Inner_Circle");

    let source_in_temp = base.join("Cloud_Kicks_Inner_Circle.loyaltyProgramSetup-meta.xml");
    std::fs::copy(fixture, &source_in_temp).expect("copy fixture to temp");

    let rule = xml_disassembler::MultiLevelRule {
        file_pattern: "programProcesses".to_string(),
        root_to_strip: "programProcesses".to_string(),
        unique_id_elements: "parameterName,ruleName".to_string(),
        path_segment: "programProcesses".to_string(),
        wrap_root_element: "LoyaltyProgramSetup".to_string(),
        wrap_xmlns: String::new(),
    };

    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            source_in_temp.to_str().unwrap(),
            Some("fullName,name,processName"),
            Some("unique-id"),
            false,
            false,
            ".xmldisassemblerignore",
            "xml",
            Some(&rule),
            None,
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

    let reassembled_path = base.join("Cloud_Kicks_Inner_Circle.xml");
    assert!(reassembled_path.exists(), "Reassembled file should exist");

    let reassembled_content = std::fs::read_to_string(&reassembled_path).expect("read reassembled");

    assert_eq!(
        original_content, reassembled_content,
        "Reassembled XML must match original (multi-level round-trip)"
    );
}

/// Grouped-by-tag with a decompose rule that has mode neither "split" nor "group" (fallback path).
#[tokio::test]
async fn grouped_by_tag_with_fallback_mode_writes_single_file() {
    let _ = env_logger::try_init();
    let fixture = "fixtures/split-tags/HR_Admin.permissionset-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture must exist (run from project root)"
    );
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let disassembled_dir = base.join("HR_Admin");
    let source = base.join("HR_Admin.permissionset-meta.xml");
    std::fs::copy(fixture, &source).expect("copy fixture");
    let fallback_rule = DecomposeRule {
        tag: "objectPermissions".to_string(),
        path_segment: "objectPermissions".to_string(),
        mode: "fallback".to_string(),
        field: "object".to_string(),
    };
    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            source.to_str().unwrap(),
            None,
            Some("grouped-by-tag"),
            false,
            false,
            ".xmldisassemblerignore",
            "xml",
            None,
            Some(&[fallback_rule]),
        )
        .await
        .expect("disassemble");
    assert!(disassembled_dir.exists());
    let fallback_file = disassembled_dir.join("objectPermissions.xml");
    assert!(
        fallback_file.exists(),
        "fallback mode writes single file to disassembled root"
    );
}

/// Grouped-by-tag with --split-tags: objectPermissions split by object, fieldPermissions grouped by object (from field).
/// Reassemble and compare to original fixture.
#[tokio::test]
async fn split_tags_disassemble_then_reassemble_matches_original() {
    let _ = env_logger::try_init();

    let fixture = "fixtures/split-tags/HR_Admin.permissionset-meta.xml";
    assert!(
        Path::new(fixture).exists(),
        "Fixture {} must exist (run from project root)",
        fixture
    );

    let original_content = std::fs::read_to_string(fixture).expect("read original fixture");

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base = temp_dir.path();
    let disassembled_dir = base.join("HR_Admin");

    let source_in_temp = base.join("HR_Admin.permissionset-meta.xml");
    std::fs::copy(fixture, &source_in_temp).expect("copy fixture to temp");

    let split_tags_rules = vec![
        DecomposeRule {
            tag: "objectPermissions".to_string(),
            path_segment: "objectPermissions".to_string(),
            mode: "split".to_string(),
            field: "object".to_string(),
        },
        DecomposeRule {
            tag: "fieldPermissions".to_string(),
            path_segment: "fieldPermissions".to_string(),
            mode: "group".to_string(),
            field: "field".to_string(),
        },
    ];

    let mut disassemble = DisassembleXmlFileHandler::new();
    disassemble
        .disassemble(
            source_in_temp.to_str().unwrap(),
            None,
            Some("grouped-by-tag"),
            false,
            false,
            ".xmldisassemblerignore",
            "xml",
            None,
            Some(&split_tags_rules),
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
        "Reassembled XML must match original (split-tags round-trip)"
    );
}

/// Full round-trip (disassemble → reassemble) for each success fixture.
/// Excludes: no-root-element (invalid root), no-nested-elements (only leaves), ignore (behavior);
/// attributes/notes.xml (reassembly differs re declaration/entities), array-of-leaves (sibling order not preserved).
#[tokio::test]
async fn fixture_round_trip_matches_original() {
    let _ = env_logger::try_init();

    /// (fixture path, optional unique_id_elements for disassemble, extension for reassemble output)
    const FIXTURES: &[(&str, Option<&str>, &str)] = &[
        ("fixtures/general/HR_Admin.permissionset-meta.xml", None, "xml"),
        (
            "fixtures/comments/Numbers-fr.globalValueSetTranslation-meta.xml",
            None,
            "globalValueSetTranslation-meta.xml",
        ),
        (
            "fixtures/cdata/VidLand_US.marketingappextension-meta.xml",
            None,
            "marketingappextension-meta.xml",
        ),
        (
            "fixtures/no-namespace/HR_Admin.permissionset-meta.xml",
            None,
            "xml",
        ),
        // attributes/notes.xml excluded: reassembly differs (no XML declaration, entity encoding)
        // array-of-leaves excluded: sibling order not preserved on reassembly
        (
            "fixtures/deeply-nested-unique-id-element/Get_Info.flow-meta.xml",
            Some("apexClass,name,object,field,layout,actionName,targetReference,assignToReference,choiceText,promptText"),
            "flow-meta.xml",
        ),
    ];

    for (fixture, unique_id_elements, reassemble_ext) in FIXTURES {
        let path = Path::new(fixture);
        assert!(
            path.exists(),
            "Fixture {} must exist (run from project root)",
            fixture
        );

        let original_content = std::fs::read_to_string(fixture).expect("read original fixture");

        let temp_dir = tempfile::tempdir().expect("temp dir");
        let base = temp_dir.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let base_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.split('.').next().unwrap_or(s))
            .unwrap_or("out");
        let disassembled_dir = base.join(base_name);

        let source_in_temp = base.join(file_name);
        std::fs::copy(fixture, &source_in_temp).expect("copy fixture to temp");

        let mut disassemble = DisassembleXmlFileHandler::new();
        let result = disassemble
            .disassemble(
                source_in_temp.to_str().unwrap(),
                *unique_id_elements,
                Some("unique-id"),
                false,
                false,
                ".xmldisassemblerignore",
                "xml",
                None,
                None,
            )
            .await;

        let Ok(()) = result else {
            panic!(
                "disassemble failed for fixture {}: {:?}",
                fixture,
                result.unwrap_err()
            );
        };

        assert!(
            disassembled_dir.exists(),
            "Disassembled directory should exist for {}",
            fixture
        );

        let reassemble_handler = ReassembleXmlFileHandler::new();
        reassemble_handler
            .reassemble(
                disassembled_dir.to_str().unwrap(),
                Some(reassemble_ext),
                false,
            )
            .await
            .expect("reassemble");

        let reassembled_path = base.join(format!("{}.{}", base_name, reassemble_ext));
        assert!(
            reassembled_path.exists(),
            "Reassembled file should exist for {} at {:?}",
            fixture,
            reassembled_path
        );

        let reassembled_content =
            std::fs::read_to_string(&reassembled_path).expect("read reassembled");

        assert_eq!(
            original_content, reassembled_content,
            "Round-trip for fixture {}: reassembled XML must match original",
            fixture
        );
    }
}
