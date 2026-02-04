fn main() {
    let xml = std::fs::read_to_string("fixtures/general/HR_Admin.permissionset-meta.xml").unwrap();
    let config = quickxml_to_serde::Config::new_with_custom_values(true, "@", "#text", quickxml_to_serde::NullValue::EmptyObject);
    let parsed = quickxml_to_serde::xml_string_to_json(xml, &config).unwrap();
    println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
}
