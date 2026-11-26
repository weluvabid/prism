use prism::normalize::{normalize, wrap_lines};

#[test]
fn every_ascii_control_byte_is_handled_without_panicking() {
    let input = (0_u8..=127).map(char::from).collect::<String>();
    let result = normalize(&input, 4, 1024 * 1024).unwrap();
    assert!(!result.contains('\u{001b}'));
    assert!(!result.contains('\u{0000}'));
}

#[test]
fn unterminated_escape_sequences_do_not_leak_control_payloads() {
    assert_eq!(
        normalize("before\u{001b}]0;hidden title", 4, 1024).unwrap(),
        "before"
    );
    assert_eq!(normalize("before\u{001b}[31", 4, 1024).unwrap(), "before");
    assert_eq!(
        normalize("before\u{001b}Pdevice payload", 4, 1024).unwrap(),
        "before"
    );
}

#[test]
fn wrapping_preserves_all_text_and_blank_lines() {
    let source = "first line\n\nsecond 你好";
    let wrapped = wrap_lines(source, 6);
    assert_eq!(
        wrapped,
        vec![
            "first ".to_owned(),
            "line".to_owned(),
            "".to_owned(),
            "second".to_owned(),
            " 你好".to_owned(),
        ]
    );
}

#[test]
fn repeated_carriage_returns_do_not_accumulate_progress_rows() {
    let mut input = String::new();
    for step in 0..250 {
        input.push_str(&format!("progress {step:03}%\r"));
    }
    input.push_str("complete\n");
    assert_eq!(normalize(&input, 4, 1024).unwrap(), "complete");
}
