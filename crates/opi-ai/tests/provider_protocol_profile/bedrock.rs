#[test]
fn provider_bedrock_protocol_source_is_constructible() {
    assert!(opi_ai::providers::bedrock::is_standard_bedrock_endpoint(
        "https://bedrock-runtime.us-east-1.amazonaws.com"
    ));
}
