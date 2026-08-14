use std::io::Read as _;
use std::path::Path;

pub fn read_word(path: &Path) -> Result<String, String> {
    try_docx_read(path)
}

fn try_docx_read(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("failed to open file: {e}"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| format!("failed to read file: {e}"))?;

    let docx = docx_rs::read_docx(&buf).map_err(|e| format!("failed to parse .docx: {e}"))?;

    let mut text = String::new();
    extract_text_from_docx(&docx, &mut text);
    if text.trim().is_empty() {
        return Err("no text content found in .docx".to_string());
    }
    Ok(text.trim().to_string())
}

fn extract_text_from_docx(docx: &docx_rs::Docx, out: &mut String) {
    for child in &docx.document.children {
        extract_document_child(child, out);
    }
}

fn extract_document_child(child: &docx_rs::DocumentChild, out: &mut String) {
    use docx_rs::DocumentChild;
    match child {
        DocumentChild::Paragraph(p) => {
            extract_paragraph(p, out);
            out.push('\n');
        }
        DocumentChild::Table(t) => extract_table(t, out),
        _ => {}
    }
}

fn extract_paragraph(p: &docx_rs::Paragraph, out: &mut String) {
    for child in &p.children {
        extract_paragraph_child(child, out);
    }
}

fn extract_paragraph_child(child: &docx_rs::ParagraphChild, out: &mut String) {
    use docx_rs::ParagraphChild;
    match child {
        ParagraphChild::Run(r) => extract_run(r, out),
        ParagraphChild::Hyperlink(h) => {
            for hc in &h.children {
                if let docx_rs::ParagraphChild::Run(r) = hc {
                    extract_run(r, out);
                }
            }
        }
        _ => {}
    }
}

fn extract_run(run: &docx_rs::Run, out: &mut String) {
    for child in &run.children {
        extract_run_child(child, out);
    }
}

fn extract_run_child(child: &docx_rs::RunChild, out: &mut String) {
    use docx_rs::RunChild;
    match child {
        RunChild::Text(t) => out.push_str(&t.text),
        RunChild::Tab(_) => out.push('\t'),
        RunChild::Break(_) => out.push('\n'),
        _ => {}
    }
}

fn extract_table(t: &docx_rs::Table, out: &mut String) {
    for child in &t.rows {
        extract_table_child(child, out);
    }
}

fn extract_table_child(child: &docx_rs::TableChild, out: &mut String) {
    use docx_rs::{TableChild, TableRowChild};
    match child {
        TableChild::TableRow(row) => {
            for child in &row.cells {
                let TableRowChild::TableCell(cell) = child;
                extract_table_cell(cell, out);
                out.push('\t');
            }
            out.push('\n');
        }
    }
}

fn extract_table_cell(cell: &docx_rs::TableCell, out: &mut String) {
    for child in &cell.children {
        extract_table_cell_content(child, out);
    }
}

fn extract_table_cell_content(child: &docx_rs::TableCellContent, out: &mut String) {
    use docx_rs::TableCellContent;
    match child {
        TableCellContent::Paragraph(p) => extract_paragraph(p, out),
        TableCellContent::Table(t) => extract_table(t, out),
        _ => {}
    }
}

pub fn write_word(path: &Path, content: &str) -> Result<String, String> {
    use docx_rs::*;

    let mut paragraphs = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            paragraphs.push(Paragraph::new().add_run(Run::new().add_text("")));
        } else {
            paragraphs.push(Paragraph::new().add_run(Run::new().add_text(line)));
        }
    }

    if paragraphs.is_empty() {
        paragraphs.push(Paragraph::new().add_run(Run::new().add_text("")));
    }

    let mut docx = Docx::new();
    for p in paragraphs {
        docx = docx.add_paragraph(p);
    }

    let xml_docx = docx.build();
    let mut cursor = std::io::Cursor::new(Vec::new());
    xml_docx
        .pack(&mut cursor)
        .map_err(|e| format!("failed to pack .docx: {e}"))?;
    let data = cursor.into_inner();
    std::fs::write(path, &data).map_err(|e| format!("failed to write .docx file: {e}"))?;

    Ok(format!("Wrote {} bytes to {}", data.len(), path.display()))
}

fn is_word_path(s: &str) -> bool {
    if let Some(dot_pos) = s.rfind('.') {
        dot_pos > 0 && s[dot_pos + 1..].eq_ignore_ascii_case("docx")
    } else {
        false
    }
}

pub fn looks_like_word_path(text: &str) -> Option<&str> {
    // First, try to extract quoted paths (handles spaces in paths)
    let mut chars = text.char_indices().peekable();
    while let Some((start, quote)) = chars.find(|&(_, c)| c == '\'' || c == '"' || c == '`') {
        let remaining = &text[start + 1..];
        if let Some(end) = remaining.find(quote) {
            let candidate = &text[start + 1..start + 1 + end];
            if is_word_path(candidate) {
                return Some(candidate);
            }
        }
    }

    // Fallback: whitespace-split unquoted tokens
    for token in text.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| c == '\'' || c == '"' || c == '`');
        if is_word_path(cleaned) {
            return Some(cleaned);
        }
    }
    None
}

pub fn maybe_extract_word_from_prompt(prompt: &str) -> Option<(String, String)> {
    let word_path = looks_like_word_path(prompt)?;
    let path = Path::new(word_path);
    if !path.exists() {
        return None;
    }
    let text = read_word(path).ok()?;
    if text.is_empty() {
        return None;
    }
    Some((word_path.to_string(), text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_roundtrip() {
        let content = "Hello\nWorld";
        let dir = std::env::temp_dir().join("clawd-word-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.docx");

        write_word(&path, content).expect("write docx");
        assert!(path.exists());

        let text = read_word(&path).expect("read docx");
        assert!(text.contains("Hello"), "got: {text}");
        assert!(text.contains("World"), "got: {text}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn looks_like_word_path_detects_references() {
        assert_eq!(
            looks_like_word_path("Open /tmp/report.docx"),
            Some("/tmp/report.docx")
        );
        assert_eq!(
            looks_like_word_path("Check file.DOCX now"),
            Some("file.DOCX")
        );
        assert_eq!(looks_like_word_path("no docx here"), None);
    }

    #[test]
    fn looks_like_word_path_handles_quoted_spaces() {
        assert_eq!(
            looks_like_word_path("Open '/path/with space/report.docx'"),
            Some("/path/with space/report.docx")
        );
        assert_eq!(
            looks_like_word_path("Read \"C:\\my docs\\file.docx\""),
            Some("C:\\my docs\\file.docx")
        );
    }

    #[test]
    fn maybe_extract_word_from_prompt_returns_none_for_missing_file() {
        let prompt = "Read /tmp/nonexistent-abc123.docx please";
        let result = maybe_extract_word_from_prompt(prompt);
        assert!(result.is_none());
    }

    #[test]
    fn returns_empty_for_non_word_data() {
        let data = b"not a docx file";
        let dir = std::env::temp_dir().join("clawd-word-err-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.docx");
        std::fs::write(&path, data).unwrap();

        let result = read_word(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
