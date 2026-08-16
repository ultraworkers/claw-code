use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::io::{self, IsTerminal, Write};

use crate::image_compressor;
use base64::Engine;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::history::{DefaultHistory, History};
use rustyline::validate::Validator;
use rustyline::{
    Cmd, CompletionType, Config, Context, EditMode, Editor, Helper, KeyCode, KeyEvent, Modifiers,
};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOutcome {
    Submit(String),
    Cancel,
    Exit,
}

struct SlashCommandHelper {
    pub completions: Vec<String>,
    pub mention_names: Vec<String>,
    pub skill_names: Vec<String>,
    current_line: RefCell<String>,
}

impl SlashCommandHelper {
    fn new(completions: Vec<String>, mention_names: Vec<String>, skill_names: Vec<String>) -> Self {
        Self {
            completions: normalize_completions(completions),
            mention_names: normalize_mention_names(mention_names),
            skill_names: normalize_mention_names(skill_names),
            current_line: RefCell::new(String::new()),
        }
    }

    fn reset_current_line(&self) {
        self.current_line.borrow_mut().clear();
    }

    fn current_line(&self) -> String {
        self.current_line.borrow().clone()
    }

    fn set_current_line(&self, line: &str) {
        let mut current = self.current_line.borrow_mut();
        current.clear();
        current.push_str(line);
    }

    fn set_completions(&mut self, completions: Vec<String>) {
        self.completions = normalize_completions(completions);
    }

    fn set_mention_names(&mut self, names: Vec<String>) {
        self.mention_names = normalize_mention_names(names);
    }

    fn set_skill_names(&mut self, names: Vec<String>) {
        self.skill_names = normalize_mention_names(names);
    }
}

impl Completer for SlashCommandHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        // Try slash command completion first
        if let Some(prefix) = slash_command_prefix(line, pos) {
            let matches = self
                .completions
                .iter()
                .filter(|candidate| candidate.starts_with(prefix))
                .map(|candidate| Pair {
                    display: candidate.clone(),
                    replacement: candidate.clone(),
                })
                .collect();
            return Ok((0, matches));
        }

        // Try @ mention completion
        let before_cursor = &line[..pos.min(line.len())];
        if let Some(at_pos) = before_cursor.rfind('@') {
            let mention_prefix = &before_cursor[at_pos + 1..];
            if !mention_prefix.contains(char::is_whitespace) {
                let matches: Vec<Pair> = self
                    .mention_names
                    .iter()
                    .filter(|name| name.starts_with(mention_prefix))
                    .map(|name| Pair {
                        display: format!("@{}", name),
                        replacement: format!("@{}", name),
                    })
                    .collect();
                return Ok((at_pos, matches));
            }
        }

        // Try $ skill completion
        if let Some(dollar_pos) = before_cursor.rfind('$') {
            let skill_prefix = &before_cursor[dollar_pos + 1..];
            if !skill_prefix.contains(char::is_whitespace) {
                let matches: Vec<Pair> = self
                    .skill_names
                    .iter()
                    .filter(|name| name.starts_with(skill_prefix))
                    .map(|name| Pair {
                        display: format!("${}", name),
                        replacement: format!("${}", name),
                    })
                    .collect();
                return Ok((dollar_pos, matches));
            }
        }

        Ok((0, Vec::new()))
    }
}

impl Hinter for SlashCommandHelper {
    type Hint = String;
}

impl Highlighter for SlashCommandHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        self.set_current_line(line);
        Cow::Borrowed(line)
    }

    fn highlight_char(&self, line: &str, _pos: usize, _kind: CmdKind) -> bool {
        self.set_current_line(line);
        false
    }
}

impl Validator for SlashCommandHelper {}
impl Helper for SlashCommandHelper {}

pub struct LineEditor {
    prompt: String,
    editor: Editor<SlashCommandHelper, DefaultHistory>,
}

impl LineEditor {
    #[must_use]
    pub fn new(
        prompt: impl Into<String>,
        completions: Vec<String>,
        mention_names: Vec<String>,
        skill_names: Vec<String>,
    ) -> Self {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build();
        let mut editor = Editor::<SlashCommandHelper, DefaultHistory>::with_config(config)
            .expect("rustyline editor should initialize");
        editor.set_helper(Some(SlashCommandHelper::new(
            completions,
            mention_names,
            skill_names,
        )));
        editor.bind_sequence(KeyEvent(KeyCode::Char('J'), Modifiers::CTRL), Cmd::Newline);
        editor.bind_sequence(KeyEvent(KeyCode::Enter, Modifiers::SHIFT), Cmd::Newline);

        Self {
            prompt: prompt.into(),
            editor,
        }
    }

    pub fn push_history(&mut self, entry: impl Into<String>) {
        let entry = entry.into();
        if entry.trim().is_empty() {
            return;
        }

        let _ = self.editor.add_history_entry(entry);
    }

    pub fn set_completions(&mut self, completions: Vec<String>) {
        if let Some(helper) = self.editor.helper_mut() {
            helper.set_completions(completions);
        }
    }

    pub fn set_mention_names(&mut self, names: Vec<String>) {
        if let Some(helper) = self.editor.helper_mut() {
            helper.set_mention_names(names);
        }
    }

    pub fn set_skill_names(&mut self, names: Vec<String>) {
        if let Some(helper) = self.editor.helper_mut() {
            helper.set_skill_names(names);
        }
    }

    pub fn get_completions(&self) -> Vec<String> {
        self.editor
            .helper()
            .map_or_else(Vec::new, |h| h.completions.clone())
    }

    pub fn get_mention_names(&self) -> Vec<String> {
        self.editor
            .helper()
            .map_or_else(Vec::new, |h| h.mention_names.clone())
    }

    pub fn get_skill_names(&self) -> Vec<String> {
        self.editor
            .helper()
            .map_or_else(Vec::new, |h| h.skill_names.clone())
    }

    pub fn read_line_interactive(&mut self) -> io::Result<ReadOutcome> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return self.read_line_fallback();
        }

        let completions = self.get_completions();
        let mention_names = self.get_mention_names();
        let skill_names = self.get_skill_names();

        let history: Vec<String> = self.editor.history().iter().cloned().collect();
        match crate::picker::run_picker(&self.prompt, &completions, &mention_names, &skill_names, &history)? {
            crate::picker::PickerResult::Submit(line) => {
                if line.trim().is_empty() {
                    return Ok(ReadOutcome::Cancel);
                }
                Ok(ReadOutcome::Submit(line))
            }
            crate::picker::PickerResult::Cancel => Ok(ReadOutcome::Cancel),
            crate::picker::PickerResult::Exit => Ok(ReadOutcome::Exit),
        }
    }

    pub fn read_line(&mut self) -> io::Result<ReadOutcome> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return self.read_line_fallback();
        }

        if let Some(helper) = self.editor.helper_mut() {
            helper.reset_current_line();
        }

        match self.editor.readline(&self.prompt) {
            Ok(line) => Ok(ReadOutcome::Submit(line)),
            Err(ReadlineError::Interrupted) => {
                let has_input = !self.current_line().is_empty();
                self.finish_interrupted_read()?;
                if has_input {
                    Ok(ReadOutcome::Cancel)
                } else {
                    Ok(ReadOutcome::Exit)
                }
            }
            Err(ReadlineError::Eof) => {
                self.finish_interrupted_read()?;
                Ok(ReadOutcome::Exit)
            }
            Err(error) => Err(io::Error::other(error)),
        }
    }

    fn current_line(&self) -> String {
        self.editor
            .helper()
            .map_or_else(String::new, SlashCommandHelper::current_line)
    }

    fn finish_interrupted_read(&mut self) -> io::Result<()> {
        if let Some(helper) = self.editor.helper_mut() {
            helper.reset_current_line();
        }
        let mut stdout = io::stdout();
        writeln!(stdout)
    }

    fn read_line_fallback(&self) -> io::Result<ReadOutcome> {
        let mut stdout = io::stdout();
        write!(stdout, "{}", self.prompt)?;
        stdout.flush()?;

        let mut buffer = String::new();
        let bytes_read = io::stdin().read_line(&mut buffer)?;
        if bytes_read == 0 {
            return Ok(ReadOutcome::Exit);
        }

        while matches!(buffer.chars().last(), Some('\n' | '\r')) {
            buffer.pop();
        }
        Ok(ReadOutcome::Submit(buffer))
    }
}

fn slash_command_prefix(line: &str, pos: usize) -> Option<&str> {
    if pos != line.len() {
        return None;
    }

    let prefix = &line[..pos];
    if !prefix.starts_with('/') {
        return None;
    }

    Some(prefix)
}

fn normalize_completions(completions: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    completions
        .into_iter()
        .filter(|candidate| candidate.starts_with('/'))
        .filter(|candidate| seen.insert(candidate.clone()))
        .collect()
}

fn normalize_mention_names(names: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    names
        .into_iter()
        .filter(|name| !name.is_empty())
        .filter(|name| seen.insert(name.clone()))
        .collect()
}

fn extract_paths_from_input(input: &str) -> (Vec<String>, String) {
    let mut paths = Vec::new();
    let mut command_parts = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '"' || ch == '\'' {
            let quote = ch;
            chars.next();
            let mut content = String::new();
            let closed = chars.by_ref().any(|c| {
                if c == quote {
                    return true;
                }
                content.push(c);
                false
            });
            if closed && !content.is_empty() && looks_like_absolute_path(&content) {
                paths.push(content);
            } else if !content.is_empty() {
                if closed {
                    command_parts.push(content);
                } else {
                    command_parts.push(format!("{quote}{content}"));
                }
            }
            continue;
        }

        if ch == '&' || ch == '[' {
            let is_ps_ref = ch == '&' && chars.clone().nth(1) == Some('[');
            let is_bracket = ch == '[';

            if is_ps_ref || is_bracket {
                if is_ps_ref {
                    chars.next();
                }
                chars.next();
                let mut content = String::new();
                let closed = chars.by_ref().any(|c| {
                    if c == ']' {
                        return true;
                    }
                    content.push(c);
                    false
                });
                if closed && !content.is_empty() && looks_like_absolute_path(&content) {
                    paths.push(content);
                    continue;
                } else {
                    command_parts.push(if is_ps_ref {
                        format!("&[{content}]")
                    } else {
                        format!("[{content}]")
                    });
                    continue;
                }
            }
        }

        let mut token: String = chars
            .by_ref()
            .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
            .collect();

        if token.is_empty() {
            continue;
        }

        if looks_like_windows_drive_prefix(&token) {
            let mut full_path = token.clone();
            let mut consumed_extra = false;

            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_whitespace() {
                    chars.next();
                    let next_token: String = chars
                        .clone()
                        .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
                        .collect();

                    if next_token.starts_with('>')
                        || next_token.starts_with('|')
                        || next_token.starts_with('-')
                        || next_token.starts_with('/')
                        || next_token.starts_with('&')
                        || next_token.starts_with(';')
                    {
                        break;
                    }

                    if !next_token.is_empty() {
                        full_path.push(' ');
                        full_path.push_str(&next_token);
                        chars
                            .by_ref()
                            .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
                            .count();
                        consumed_extra = true;
                        continue;
                    }
                    break;
                } else {
                    break;
                }
            }

            if looks_like_absolute_path(&full_path) {
                paths.push(full_path);
                continue;
            } else if consumed_extra {
                command_parts.push(full_path);
                continue;
            }
            token = full_path;
        }

        if looks_like_absolute_path(&token) {
            paths.push(token);
        } else {
            command_parts.push(token);
        }
    }

    let remaining = command_parts.join(" ");
    (paths, remaining)
}

fn looks_like_windows_drive_prefix(s: &str) -> bool {
    if s.len() < 2 {
        return false;
    }
    let bytes = s.as_bytes();
    bytes[0].is_ascii_alphabetic() && (bytes[1] == b':' || bytes[1] == b'|')
}

pub fn looks_like_absolute_path(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.len() < 2 {
        return false;
    }
    if trimmed.starts_with("file://") {
        return true;
    }
    if trimmed.starts_with('/') {
        return true;
    }
    {
        let chars: Vec<char> = trimmed.chars().collect();
        if let Some(first) = chars.first() {
            if first.is_alphabetic() && (chars.get(1) == Some(&':') || chars.get(1) == Some(&'|')) {
                return true;
            }
        }
        if trimmed.starts_with(r"\\") || trimmed.starts_with("//") {
            return true;
        }
    }
    if trimmed.starts_with("./") || trimmed.starts_with("../") || trimmed.starts_with("~/") {
        return true;
    }
    if trimmed.starts_with(r"\\?\") {
        return true;
    }
    false
}

const MAX_INLINE_TEXT_BYTES: u64 = 10 * 1024;
const MAX_TEXT_FILE_CHARS: usize = 8000;
const MAX_INLINE_LINES: usize = 300;

#[derive(Debug, Clone)]
pub enum InputContent {
    Text(String),
    Image { mime_type: String, data: String },
    ImageStored { mime_type: String, hash_hex: String },
    File { text: String, source_path: String },
}

fn file_to_input_content(
    path: &std::path::Path,
    image_dir: Option<&std::path::Path>,
) -> Option<InputContent> {
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[FILE] Cannot access '{}': {}", path.display(), e);
            return None;
        }
    };
    let size = metadata.len();
    let path_str = path.display().to_string();
    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    if mime.starts_with("image/") {
        let image_data = match std::fs::read(path) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("[IMAGE] Failed to read file: {}", e);
                return None;
            }
        };

        match image_compressor::compress_image(&image_data) {
            Ok(result) => {
                //let size_kb = result.data.len() as f64 / 1024.0;
                //let label = if result.mime_type == "image/png" { "PNG" } else { "JPEG" };
                //eprintln!("[IMAGE] compressed to {label} ~{size_kb:.0} KB");
                // If we have an image store path, try to write directly
                if let Some(dir) = image_dir {
                    let mut hasher = Sha256::new();
                    hasher.update(&result.data);
                    let hash_bytes = hasher.finalize();
                    let hash_hex: String =
                        hash_bytes.iter().map(|b| format!("{:02x}", b)).collect();
                    let prefix = &hash_hex[..2];
                    let ext = mime_to_ext(&result.mime_type);
                    let store_path = dir.join(prefix).join(format!("{hash_hex}.{ext}"));
                    let b64_path = dir.join(prefix).join(format!("{hash_hex}.{ext}.b64"));
                    let stored_ok = std::fs::create_dir_all(store_path.parent().unwrap()).is_ok()
                        && std::fs::write(&store_path, &result.data).is_ok();
                    let b64_ok = stored_ok
                        && std::fs::write(
                            &b64_path,
                            &base64::engine::general_purpose::STANDARD.encode(&result.data),
                        )
                        .is_ok();
                    if b64_ok {
                        return Some(InputContent::ImageStored {
                            mime_type: result.mime_type,
                            hash_hex,
                        });
                    }
                    if !stored_ok {
                        eprintln!(
                            "[IMAGE] Failed to write image to disk at {}",
                            store_path.display()
                        );
                    } else {
                        eprintln!(
                            "[IMAGE] Stored raw but failed to write base64 sidecar at {}",
                            b64_path.display()
                        );
                    }
                }
                // Fall back to base64 transport
                let base64_data = base64::engine::general_purpose::STANDARD.encode(&result.data);
                return Some(InputContent::Image {
                    mime_type: result.mime_type,
                    data: base64_data,
                });
            }
            Err(e) => {
                eprintln!("[IMAGE] Compression failed: {}, using original", e);
                let base64_data = base64::engine::general_purpose::STANDARD.encode(&image_data);
                return Some(InputContent::Image {
                    mime_type: mime,
                    data: base64_data,
                });
            }
        }
    }

    if is_text_mime(&mime) {
        let raw_bytes = match std::fs::read(path) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("[FILE] Failed to read '{}': {}", path.display(), e);
                return None;
            }
        };
        let mut text = match std::str::from_utf8(&raw_bytes) {
            Ok(valid) => valid.to_string(),
            Err(_) => {
                let mut detector = chardetng::EncodingDetector::new();
                detector.feed(&raw_bytes, true);
                let encoding = detector.guess(None, true);
                let mut decoder = encoding.new_decoder_without_bom_handling();
                let mut decoded = String::with_capacity(raw_bytes.len());
                let (_, _, _, had_replacement) =
                    decoder.decode_to_str(&raw_bytes, &mut decoded, true);
                if decoded.trim().is_empty() || had_replacement {
                    String::from_utf8_lossy(&raw_bytes).into_owned()
                } else {
                    decoded
                }
            }
        };

        if text.chars().count() > MAX_TEXT_FILE_CHARS {
            text = text.chars().take(MAX_TEXT_FILE_CHARS).collect();
        }

        let lang = infer_language_hint(path, &mime);
        return Some(InputContent::File {
            text: format!("File: `{path_str}` ({size} bytes, {mime})\n```{lang}\n{text}\n```"),
            source_path: path_str,
        });
    }

    Some(InputContent::File {
        text: format!("Binary file: `{path_str}` ({size} bytes, {mime})"),
        source_path: path_str,
    })
}

fn is_text_mime(mime: &str) -> bool {
    mime.starts_with("text/")
        || matches!(
            mime,
            "application/json"
                | "application/xml"
                | "application/javascript"
                | "application/typescript"
                | "application/yaml"
                | "application/toml"
                | "application/x-shellscript"
        )
}
fn infer_language_hint(path: &std::path::Path, mime: &str) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" => "rust",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "py" => "python",
        "sh" | "bash" | "zsh" => "bash",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "md" => "markdown",
        "html" | "htm" => "html",
        "css" => "css",
        "xml" => "xml",
        "sql" => "sql",
        _ => "",
    }
}

pub fn resolve_drag_drop_files(input: &str, image_dir: Option<&std::path::Path>) -> String {
    let (paths, command) = extract_paths_from_input(input);

    if paths.is_empty() {
        return input.to_string();
    }

    let mut result = String::new();
    let mut has_content = false;

    for path_str in &paths {
        let decoded_path_str = url_decode(path_str);
        let clean_path_str = if let Some(rest) = decoded_path_str.strip_prefix("file://") {
            if rest.starts_with('/') {
                let rest_chars: Vec<char> = rest.chars().collect();
                if rest_chars.len() >= 3
                    && rest_chars[1].is_ascii_alphabetic()
                    && (rest_chars[2] == ':' || rest_chars[2] == '|')
                {
                    rest[1..].to_string()
                } else if rest_chars.len() >= 2 {
                    format!("/{}", rest)
                } else {
                    format!("/{}", rest)
                }
            } else {
                rest.to_string()
            }
        } else {
            decoded_path_str.to_string()
        };

        let path = std::path::PathBuf::from(&clean_path_str);
        if !path.is_file() {
            continue;
        }

        if let Some(content) = file_to_input_content(&path, image_dir) {
            match content {
                InputContent::Image { mime_type, data } => {
                    result.push_str(&format!(
                        "<input_image mime=\"{}\" base64=\"{}\"/>\n",
                        mime_type, data
                    ));
                    has_content = true;
                }
                InputContent::ImageStored {
                    mime_type,
                    hash_hex,
                } => {
                    result.push_str(&format!(
                        "<input_image mime=\"{}\" hash=\"{}\"/>\n",
                        mime_type, hash_hex
                    ));
                    has_content = true;
                }
                InputContent::File { text, source_path } => {
                    result.push_str(&format!(
                        "<input_file path=\"{}\">\nAttached file content:\n{}\n</input_file>\n",
                        source_path, text
                    ));
                    has_content = true;
                }
                InputContent::Text(s) => {
                    result.push_str(&s);
                    result.push('\n');
                    has_content = true;
                }
            }
        }
    }

    if !has_content {
        return input.to_string();
    }

    let cmd = command.trim();
    if !cmd.is_empty() {
        result.push_str(&cmd);
    }

    result.trim().to_string()
}

fn url_decode(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hex1 = chars.next();
            let hex2 = chars.next();
            if let (Some(h1), Some(h2)) = (hex1, hex2) {
                if let Ok(byte) = u8::from_str_radix(&format!("{h1}{h2}"), 16) {
                    bytes.push(byte);
                    continue;
                }
            }
        }
        bytes.push(ch as u8);
    }

    String::from_utf8(bytes).unwrap_or_else(|_| input.to_string())
}

fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    }
}

#[cfg(test)]
mod tests {
    use super::{slash_command_prefix, LineEditor, SlashCommandHelper};
    use rustyline::completion::Completer;
    use rustyline::highlight::Highlighter;
    use rustyline::history::{DefaultHistory, History};
    use rustyline::Context;

    #[test]
    fn extracts_terminal_slash_command_prefixes_with_arguments() {
        assert_eq!(slash_command_prefix("/he", 3), Some("/he"));
        assert_eq!(slash_command_prefix("/help me", 8), Some("/help me"));
        assert_eq!(
            slash_command_prefix("/session switch ses", 19),
            Some("/session switch ses")
        );
        assert_eq!(slash_command_prefix("hello", 5), None);
        assert_eq!(slash_command_prefix("/help", 2), None);
    }

    #[test]
    fn completes_matching_slash_commands() {
        let helper = SlashCommandHelper::new(
            vec![
                "/help".to_string(),
                "/hello".to_string(),
                "/status".to_string(),
            ],
            vec![],
            vec![],
        );
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);
        let (start, matches) = helper
            .complete("/he", 3, &ctx)
            .expect("completion should work");

        assert_eq!(start, 0);
        assert_eq!(
            matches
                .into_iter()
                .map(|candidate| candidate.replacement)
                .collect::<Vec<_>>(),
            vec!["/help".to_string(), "/hello".to_string()]
        );
    }

    #[test]
    fn completes_matching_slash_command_arguments() {
        let helper = SlashCommandHelper::new(
            vec![
                "/model".to_string(),
                "/model opus".to_string(),
                "/model sonnet".to_string(),
                "/session switch alpha".to_string(),
            ],
            vec![],
            vec![],
        );
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);
        let (start, matches) = helper
            .complete("/model o", 8, &ctx)
            .expect("completion should work");

        assert_eq!(start, 0);
        assert_eq!(
            matches
                .into_iter()
                .map(|candidate| candidate.replacement)
                .collect::<Vec<_>>(),
            vec!["/model opus".to_string()]
        );
    }

    #[test]
    fn ignores_non_slash_command_completion_requests() {
        let helper = SlashCommandHelper::new(vec!["/help".to_string()], vec![], vec![]);
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);
        let (_, matches) = helper
            .complete("hello", 5, &ctx)
            .expect("completion should work");

        assert!(matches.is_empty());
    }

    #[test]
    fn tracks_current_buffer_through_highlighter() {
        let helper = SlashCommandHelper::new(Vec::new(), vec![], vec![]);
        let _ = helper.highlight("draft", 5);

        assert_eq!(helper.current_line(), "draft");
    }

    #[test]
    fn push_history_ignores_blank_entries() {
        let mut editor = LineEditor::new("> ", vec!["/help".to_string()], vec![], vec![]);
        editor.push_history("   ");
        editor.push_history("/help");

        assert_eq!(editor.editor.history().len(), 1);
    }

    #[test]
    fn set_completions_replaces_and_normalizes_candidates() {
        let mut editor = LineEditor::new("> ", vec!["/help".to_string()], vec![], vec![]);
        editor.set_completions(vec![
            "/model opus".to_string(),
            "/model opus".to_string(),
            "status".to_string(),
        ]);

        let helper = editor.editor.helper().expect("helper should exist");
        assert_eq!(helper.completions, vec!["/model opus".to_string()]);
    }
}
