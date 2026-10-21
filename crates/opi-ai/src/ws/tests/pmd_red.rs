use crate::ws::pmd::Decoder;
fn strict(payload: &[u8]) -> Result<Vec<u8>, crate::ws::pmd::Fault> {
    Decoder::new(false)?.decode(payload)
}
#[test]
fn truncated_stored_body_must_fail() {
    assert!(strict(&[0, 10, 0, 245, 255, 65]).is_err());
}
#[test]
fn missing_block_header_must_fail() {
    assert!(strict(&[]).is_err());
}
#[test]
fn valid_empty_control() {
    assert_eq!(strict(&[0]).unwrap(), b"");
}
