use super::*;

#[test]
fn debug_redacts_synthetic_user_argument_values() {
    let synthetic = "synthetic-private-value-7f314b";
    let config = config().with_user_args(vec![synthetic.into()]);
    let debug = format!("{config:?}");
    assert!(!debug.contains(synthetic));
    assert!(debug.contains("user_argument_count"));
}
