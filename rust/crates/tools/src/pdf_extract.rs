use std::path::Path;

pub fn extract_text(path: &Path) -> Result<String, String> {
    match std::panic::catch_unwind(|| pdf_extract::extract_text(path)) {
        Ok(Ok(text)) => Ok(text.trim().to_string()),
        Ok(Err(e)) => Err(format!("pdf-extract error: {e}")),
        Err(panic) => {
            let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            Err(format!("pdf-extract panic: {msg}"))
        }
    }
}

pub fn extract_text_from_bytes(data: &[u8]) -> String {
    match std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(data)) {
        Ok(Ok(text)) => text.trim().to_string(),
        _ => String::new(),
    }
}

pub fn looks_like_pdf_path(text: &str) -> Option<&str> {
    for token in text.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| c == '\'' || c == '"' || c == '`');
        if let Some(dot_pos) = cleaned.rfind('.') {
            if cleaned[dot_pos + 1..].eq_ignore_ascii_case("pdf") && dot_pos > 0 {
                return Some(cleaned);
            }
        }
    }
    None
}

pub fn maybe_extract_pdf_from_prompt(prompt: &str) -> Option<(String, String)> {
    let pdf_path = looks_like_pdf_path(prompt)?;
    let path = Path::new(pdf_path);
    if !path.exists() {
        return None;
    }
    let text = extract_text(path).ok()?;
    if text.is_empty() {
        return None;
    }
    Some((pdf_path.to_string(), text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_minimal_pdf(text: &str) -> Vec<u8> {
        let content = format!("BT\n/F1 12 Tf\n({text}) Tj\nET");
        let stream_bytes = content.as_bytes();
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        let o3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
        );

        let o4 = pdf.len();
        let header = format!("4 0 obj\n<< /Length {} >>\nstream\n", stream_bytes.len());
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(stream_bytes);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");

        let o5 = pdf.len();
        pdf.extend_from_slice(
            b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        );

        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 6\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o5:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    #[test]
    fn extracts_text_from_pdf() {
        let pdf_bytes = build_minimal_pdf("Hello World");
        let text = extract_text_from_bytes(&pdf_bytes);
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn returns_empty_for_non_pdf_data() {
        let data = b"This is not a PDF file at all";
        let text = extract_text_from_bytes(data);
        assert!(text.is_empty());
    }

    #[test]
    fn extracts_text_from_file_on_disk() {
        let pdf_bytes = build_minimal_pdf("Disk Test");
        let dir = std::env::temp_dir().join("clawd-pdf-extract-test");
        std::fs::create_dir_all(&dir).unwrap();
        let pdf_path = dir.join("test.pdf");
        std::fs::write(&pdf_path, &pdf_bytes).unwrap();

        let text = extract_text(&pdf_path).unwrap();
        assert_eq!(text, "Disk Test");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn looks_like_pdf_path_detects_pdf_references() {
        assert_eq!(
            looks_like_pdf_path("Please read /tmp/report.pdf"),
            Some("/tmp/report.pdf")
        );
        assert_eq!(looks_like_pdf_path("Check file.PDF now"), Some("file.PDF"));
        assert_eq!(looks_like_pdf_path("no pdf here"), None);
    }

    #[test]
    fn maybe_extract_pdf_from_prompt_returns_none_for_missing_file() {
        let prompt = "Read /tmp/nonexistent-abc123.pdf please";
        let result = maybe_extract_pdf_from_prompt(prompt);
        assert!(result.is_none());
    }

    #[test]
    fn maybe_extract_pdf_from_prompt_extracts_existing_file() {
        let pdf_bytes = build_minimal_pdf("Auto Extracted");
        let dir = std::env::temp_dir().join("clawd-pdf-auto-extract-test");
        std::fs::create_dir_all(&dir).unwrap();
        let pdf_path = dir.join("auto.pdf");
        std::fs::write(&pdf_path, &pdf_bytes).unwrap();
        let prompt = format!("Summarize {}", pdf_path.display());

        let result = maybe_extract_pdf_from_prompt(&prompt);
        let (path, text) = result.expect("should extract");
        assert_eq!(path, pdf_path.display().to_string());
        assert_eq!(text, "Auto Extracted");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
