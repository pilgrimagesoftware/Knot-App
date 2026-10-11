use super::*;

const FIXTURE: &str = include_str!("../../tests/fixtures/index.json");

#[test]
fn parses_the_fixture_index() {
    let index = Index::parse(FIXTURE.as_bytes()).expect("fixture parses");
    assert_eq!(index.format, SUPPORTED_FORMAT);
    assert_eq!(index.items.len(), 3);
    assert_eq!(index.items[0].kind, ItemKind::Persona);
    assert_eq!(index.items[1].kind, ItemKind::Prompt);
    assert_eq!(index.items[2].kind, ItemKind::Other("bench".to_string()));
}

#[test]
fn rejects_an_unsupported_format() {
    let json = FIXTURE.replace("\"format\": 1", "\"format\": 2");
    let err = Index::parse(json.as_bytes()).expect_err("format 2 is refused");
    assert!(matches!(err, LibraryError::UnsupportedFormat(2)));
}

#[test]
fn round_trips_item_kind() {
    for kind in [ItemKind::Persona,
                 ItemKind::Prompt,
                 ItemKind::Other("bench".to_string())]
    {
        let json = serde_json::to_string(&kind).unwrap();
        let back: ItemKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }
}
