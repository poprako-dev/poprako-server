use super::ListLimit;

#[test]
fn deserialize_accepts_limit_inside_compile_time_range() {
    let lower = serde_json::from_str::<ListLimit<200>>("1").unwrap();

    let upper = serde_json::from_str::<ListLimit<200>>("200").unwrap();

    assert_eq!(lower.get(), 1);

    assert_eq!(upper.get(), 200);
}

#[test]
fn deserialize_rejects_limit_outside_compile_time_range() {
    let zero = serde_json::from_str::<ListLimit<200>>("0").unwrap_err();

    let overflow = serde_json::from_str::<ListLimit<200>>("201").unwrap_err();

    assert!(zero.to_string().contains("1..=200"));

    assert!(overflow.to_string().contains("1..=200"));
}

#[test]
fn default_const_parameter_uses_twenty_as_maximum() {
    let upper: Option<ListLimit> = ListLimit::new(20);

    let overflow: Option<ListLimit> = ListLimit::new(21);

    assert!(upper.is_some());

    assert!(overflow.is_none());
}

#[cfg(feature = "swagger")]
#[test]
fn openapi_schema_uses_compile_time_maximum() {
    use utoipa::{PartialSchema as _, ToSchema as _};

    let schema = serde_json::to_value(ListLimit::<37>::schema()).unwrap();

    assert_eq!(schema["minimum"], 1);

    assert_eq!(schema["maximum"], 37);

    assert_eq!(ListLimit::<37>::name(), "ListLimit37");
}
