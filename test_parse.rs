fn main() {
    let xml = std::fs::read_to_string("fixtures/general/HR_Admin.permissionset-meta.xml").unwrap();
    let parsed = xml_disassembler::parsers::parse_xml_cdata::parse_xml_with_cdata(&xml).unwrap();
    println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
}
