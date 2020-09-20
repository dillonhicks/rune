use crate::testing::*;

#[test]
fn test_function_argument_types_not_supported() {
    assert_compile_error! {
        r#"fn main(argv, argc: usize) { 0 }"#,
        span, Internal {..} => {
            assert_eq!(span, Span::new(18, 25));
        }
    };
}

#[test]
fn test_function_return_types_not_supported() {
    assert_compile_error! {
        r#"fn main() -> i32 { 0 }"#,
        span, Internal {..} => {
            assert_eq!(span, Span::new(10, 16));
        }
    };
}
