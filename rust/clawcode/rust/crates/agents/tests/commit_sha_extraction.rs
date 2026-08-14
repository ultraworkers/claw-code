use agents::extract_commit_sha;

#[test]
fn extracts_full_sha1() {
    let result = "landed in commit deadbeef1234567890abcdef1234567890abcdef cleanly";
    assert_eq!(
        extract_commit_sha(result).as_deref(),
        Some("deadbeef1234567890abcdef1234567890abcdef"),
    );
}

#[test]
fn extracts_short_sha_after_commit_word() {
    let result = "landed as commit abc1234def and pushed";
    assert_eq!(extract_commit_sha(result).as_deref(), Some("abc1234def"));
}

#[test]
fn extracts_short_sha_after_at_marker() {
    let result = "tagged as @abc1234def5";
    assert_eq!(extract_commit_sha(result).as_deref(), Some("abc1234def5"));
}

#[test]
fn rejects_uuid_fragment_without_context() {
    let result = "see request id deadbeef-1234-5678-9abc-def012345678 in logs";
    assert_eq!(extract_commit_sha(result), None);
}

#[test]
fn rejects_seven_char_hex_surrounded_by_digits() {
    let result = "the previous build was 1234567890abcdef in sequence";
    assert_eq!(extract_commit_sha(result), None);
}

#[test]
fn rejects_seven_char_hex_in_markdown_link() {
    let result = "see [the diff](https://github.com/x/y/commit/abc1234) for context";
    assert_eq!(extract_commit_sha(result), None);
}

#[test]
fn rejects_short_sha_below_seven_chars() {
    let result = "pinned to commit abc12";
    assert_eq!(extract_commit_sha(result), None);
}
