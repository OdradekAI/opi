use opi_ai::stream::{CumulativeUsage, Usage};

#[test]
fn cumulative_usage_accumulates() {
    let mut cu = CumulativeUsage::default();
    cu.accumulate(&Usage::reported(100, 50, 0, 0));
    assert_eq!(cu.total_input_tokens(), 100);
    assert_eq!(cu.total_output_tokens(), 50);
    assert_eq!(cu.turn_count(), 1);

    cu.accumulate(&Usage::reported(200, 75, 10, 5));
    assert_eq!(cu.total_input_tokens(), 300);
    assert_eq!(cu.total_output_tokens(), 125);
    assert_eq!(cu.turn_count(), 2);
}
