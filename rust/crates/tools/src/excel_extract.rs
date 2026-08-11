use calamine::Reader as _;
use std::fmt::Write as _;
use std::path::Path;

fn format_float(f: f64) -> String {
    if !f.is_finite() {
        return f.to_string();
    }
    if f != f.trunc() {
        return f.to_string();
    }
    if f >= i64::MIN as f64 && f <= i64::MAX as f64 {
        let truncated = f as i64;
        if (truncated as f64) == f {
            return truncated.to_string();
        }
    }
    f.to_string()
}

fn is_excel_path(s: &str) -> bool {
    if let Some(dot_pos) = s.rfind('.') {
        let ext = &s[dot_pos + 1..];
        dot_pos > 0
            && (ext.eq_ignore_ascii_case("xlsx")
                || ext.eq_ignore_ascii_case("xls")
                || ext.eq_ignore_ascii_case("xlsb")
                || ext.eq_ignore_ascii_case("ods"))
    } else {
        false
    }
}

pub fn read_excel(path: &Path) -> Result<String, String> {
    let mut workbook = calamine::open_workbook_auto(path)
        .map_err(|e| format!("failed to open Excel file: {e}"))?;

    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();

    for (si, name) in sheet_names.iter().enumerate() {
        if si > 0 {
            output.push('\n');
        }
        if sheet_names.len() > 1 {
            let _ = writeln!(output, "=== Sheet: {name} ===");
        }

        let range = workbook
            .worksheet_range(name)
            .map_err(|e| format!("failed to read sheet '{name}': {e}"))?;

        for row in range.rows() {
            let mut first = true;
            for cell in row {
                if !first {
                    output.push('\t');
                }
                first = false;
                match cell {
                    calamine::Data::Empty => {}
                    calamine::Data::String(s) => output.push_str(s.as_str()),
                    calamine::Data::Float(f) => output.push_str(&format_float(*f)),
                    calamine::Data::Int(i) => output.push_str(&i.to_string()),
                    calamine::Data::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
                    calamine::Data::DateTime(d) => output.push_str(&d.to_string()),
                    calamine::Data::Error(e) => output.push_str(&format!("#{e:?}")),
                    calamine::Data::DateTimeIso(s) => output.push_str(s.as_str()),
                    calamine::Data::DurationIso(s) => output.push_str(s.as_str()),
                }
            }
            output.push('\n');
        }
    }

    Ok(output)
}

pub fn write_excel(path: &Path, data: &[Vec<String>]) -> Result<String, String> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    for (row_idx, row) in data.iter().enumerate() {
        for (col_idx, value) in row.iter().enumerate() {
            worksheet
                .write(row_idx as u32, col_idx as u16, value.as_str())
                .map_err(|e| format!("failed to write cell ({row_idx},{col_idx}): {e}"))?;
        }
    }

    workbook
        .save(path)
        .map_err(|e| format!("failed to save Excel file: {e}"))?;

    Ok(format!(
        "Wrote {} rows x {} columns to {}",
        data.len(),
        data.iter().map(|r| r.len()).max().unwrap_or(0),
        path.display()
    ))
}

pub fn looks_like_excel_path(text: &str) -> Option<&str> {
    // First, try to extract quoted paths (handles spaces in paths)
    let mut chars = text.char_indices().peekable();
    while let Some((start, quote)) = chars.find(|&(_, c)| c == '\'' || c == '"' || c == '`') {
        let remaining = &text[start + 1..];
        if let Some(end) = remaining.find(quote) {
            let candidate = &text[start + 1..start + 1 + end];
            if is_excel_path(candidate) {
                return Some(candidate);
            }
        }
    }

    // Fallback: whitespace-split unquoted tokens
    for token in text.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| c == '\'' || c == '"' || c == '`');
        if is_excel_path(cleaned) {
            return Some(cleaned);
        }
    }
    None
}

pub fn maybe_extract_excel_from_prompt(prompt: &str) -> Option<(String, String)> {
    let excel_path = looks_like_excel_path(prompt)?;
    let path = Path::new(excel_path);
    if !path.exists() {
        return None;
    }
    let text = read_excel(path).ok()?;
    if text.is_empty() {
        return None;
    }
    Some((excel_path.to_string(), text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_simple_xlsx(data: &[&[&str]]) -> Vec<u8> {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        for (r, row) in data.iter().enumerate() {
            for (c, val) in row.iter().enumerate() {
                worksheet
                    .write(r as u32, c as u16, *val)
                    .expect("write cell");
            }
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        workbook.save_to_writer(&mut buf).expect("save to buffer");
        buf.into_inner()
    }

    #[test]
    fn reads_excel_data() {
        let xlsx = build_simple_xlsx(&[&["A", "B"], &["1", "2"]]);
        let dir = std::env::temp_dir().join("clawd-excel-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.xlsx");
        std::fs::write(&path, &xlsx).unwrap();

        let text = read_excel(&path).unwrap();
        assert!(text.contains("A"));
        assert!(text.contains("B"));
        assert!(text.contains("1"));
        assert!(text.contains("2"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_excel_data() {
        let dir = std::env::temp_dir().join("clawd-excel-write-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("output.xlsx");

        let data = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
        ];
        let result = write_excel(&path, &data).expect("write");
        assert!(result.contains("Wrote 2 rows"));

        let text = read_excel(&path).expect("read back");
        assert!(text.contains("Name"));
        assert!(text.contains("Alice"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn looks_like_excel_path_detects_references() {
        assert_eq!(
            looks_like_excel_path("Open /tmp/data.xlsx"),
            Some("/tmp/data.xlsx")
        );
        assert_eq!(
            looks_like_excel_path("Check file.XLS now"),
            Some("file.XLS")
        );
        assert_eq!(looks_like_excel_path("file.ods here"), Some("file.ods"));
        assert_eq!(looks_like_excel_path("no excel here"), None);
    }

    #[test]
    fn looks_like_excel_path_handles_quoted_spaces() {
        assert_eq!(
            looks_like_excel_path("Read '/path/with space/file.xlsx' please"),
            Some("/path/with space/file.xlsx")
        );
        assert_eq!(
            looks_like_excel_path("Process \"C:\\my docs\\data.xls\" now"),
            Some("C:\\my docs\\data.xls")
        );
    }

    #[test]
    fn maybe_extract_excel_from_prompt_returns_none_for_missing_file() {
        let prompt = "Read /tmp/nonexistent-abc123.xlsx please";
        let result = maybe_extract_excel_from_prompt(prompt);
        assert!(result.is_none());
    }

    #[test]
    fn returns_empty_for_non_excel_data() {
        let data = b"not an excel file";
        let dir = std::env::temp_dir().join("clawd-excel-err-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.xlsx");
        std::fs::write(&path, data).unwrap();

        let result = read_excel(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
