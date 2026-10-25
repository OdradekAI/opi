//! Source-aware architecture and offline-test audit helpers.
//! Explicit acceptance markers include helper modules even without test functions.

// opi-runtime-acceptance
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) const RUNTIME_ACCEPTANCE_SOURCES: &[&str] = &[
    "crates/opi-agent/tests/agent_loop_semantics.rs",
    "crates/opi-agent/tests/agent_wrapper.rs",
    "crates/opi-agent/tests/evidence_contract.rs",
    "crates/opi-agent/tests/evidence_runtime.rs",
    "crates/opi-agent/tests/hooks_queues.rs",
    "crates/opi-agent/tests/retry_agent.rs",
    "crates/opi-agent/tests/tool_authority.rs",
    "crates/opi-ai/tests/auth_contracts.rs",
    "crates/opi-ai/tests/oauth_wire_shape.rs",
    "crates/opi-ai/tests/per_request_auth.rs",
    "crates/opi-ai/tests/provider_collection.rs",
    "crates/opi-coding-agent/src/rpc.rs",
    "crates/opi-coding-agent/tests/architecture_guards.rs",
    "crates/opi-coding-agent/tests/ci_contract.rs",
    "crates/opi-coding-agent/tests/common/auth_runtime.rs",
    "crates/opi-coding-agent/tests/common/runtime_evidence.rs",
    "crates/opi-coding-agent/tests/common/source_audit.rs",
    "crates/opi-coding-agent/tests/cross_mode_runtime.rs",
    "crates/opi-coding-agent/tests/evidence_artifacts.rs",
    "crates/opi-coding-agent/tests/interactive_mock.rs",
    "crates/opi-coding-agent/tests/json_mode.rs",
    "crates/opi-coding-agent/tests/non_interactive.rs",
    "crates/opi-coding-agent/tests/product_evidence.rs",
    "crates/opi-coding-agent/tests/provider_runtime.rs",
    "crates/opi-coding-agent/tests/rpc_jsonl.rs",
    "crates/opi-coding-agent/tests/runtime_failure_recovery.rs",
    "crates/opi-coding-agent/tests/session_migration.rs",
    "crates/opi-coding-agent/tests/session_runtime.rs",
    "crates/opi-coding-agent/tests/test_hermeticity.rs",
    "crates/opi-coding-agent/tests/tool_authority.rs",
];

pub(super) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Recursively collect every `.rs` file under `dir`.
fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read source directory {}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("read source directory entry in {}: {error}", dir.display())
        });
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Production source files (`crates/*/src/**/*.rs`) paired with their path for
/// failure messages.
pub(super) fn production_sources() -> Vec<(PathBuf, String)> {
    let root = workspace_root();
    let mut files = Vec::new();
    for crate_dir in [
        "opi-ai",
        "opi-tui",
        "opi-agent",
        "opi-protocol",
        "opi-sandbox",
        "opi-coding-agent",
    ] {
        collect_rust_files(&root.join("crates").join(crate_dir).join("src"), &mut files);
    }
    files
        .into_iter()
        .map(|p| {
            let raw = std::fs::read_to_string(&p)
                .unwrap_or_else(|error| panic!("read production source {}: {error}", p.display()));
            (p, raw)
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RustTokenKind {
    Ident(String),
    Literal(String),
    Punct(char),
    Group {
        delimiter: char,
        tokens: Vec<RustToken>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RustToken {
    kind: RustTokenKind,
    line: usize,
    raw_identifier: bool,
}

#[derive(Debug)]
struct LexedRust {
    tokens: Vec<RustToken>,
    acceptance_marker: bool,
}

struct RustLexer<'a> {
    source: &'a str,
    index: usize,
    line: usize,
    acceptance_marker: bool,
}

impl<'a> RustLexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            index: 0,
            line: 1,
            acceptance_marker: false,
        }
    }

    fn lex(mut self) -> Result<LexedRust, String> {
        let tokens = self.lex_group(None)?;
        Ok(LexedRust {
            tokens,
            acceptance_marker: self.acceptance_marker,
        })
    }

    fn lex_group(&mut self, closing: Option<char>) -> Result<Vec<RustToken>, String> {
        let mut tokens = Vec::new();
        while self.index < self.source.len() {
            let character = self.peek_char().expect("index is in bounds");
            if Some(character) == closing {
                self.bump_char();
                return Ok(tokens);
            }
            if character.is_whitespace() {
                self.bump_char();
                continue;
            }
            if self.starts_with("//") {
                self.skip_line_comment();
                continue;
            }
            if self.starts_with("/*") {
                self.skip_block_comment()?;
                continue;
            }
            if let Some((prefix_len, hashes)) = self.raw_literal_prefix() {
                let line = self.line;
                let literal = self.skip_raw_literal(prefix_len, hashes)?;
                tokens.push(RustToken {
                    kind: RustTokenKind::Literal(literal),
                    line,
                    raw_identifier: false,
                });
                continue;
            }
            if character == '"' {
                let line = self.line;
                let literal = decode_cooked_literal(&self.skip_quoted_literal(0, '"')?);
                tokens.push(RustToken {
                    kind: RustTokenKind::Literal(literal),
                    line,
                    raw_identifier: false,
                });
                continue;
            }
            if matches!(character, 'b' | 'c') && self.peek_byte(1) == Some(b'"') {
                let line = self.line;
                let literal = decode_cooked_literal(&self.skip_quoted_literal(1, '"')?);
                tokens.push(RustToken {
                    kind: RustTokenKind::Literal(literal),
                    line,
                    raw_identifier: false,
                });
                continue;
            }
            if character == '\'' && self.char_literal_end(self.index).is_some() {
                let line = self.line;
                let literal = self.skip_quoted_literal(0, '\'')?;
                tokens.push(RustToken {
                    kind: RustTokenKind::Literal(literal),
                    line,
                    raw_identifier: false,
                });
                continue;
            }
            if character == 'b'
                && self.peek_byte(1) == Some(b'\'')
                && self.char_literal_end(self.index + 1).is_some()
            {
                let line = self.line;
                let literal = self.skip_quoted_literal(1, '\'')?;
                tokens.push(RustToken {
                    kind: RustTokenKind::Literal(literal),
                    line,
                    raw_identifier: false,
                });
                continue;
            }

            let line = self.line;
            if matches!(character, '(' | '[' | '{') {
                let delimiter = character;
                self.bump_char();
                let expected = match delimiter {
                    '(' => ')',
                    '[' => ']',
                    '{' => '}',
                    _ => unreachable!(),
                };
                let nested = self.lex_group(Some(expected))?;
                tokens.push(RustToken {
                    kind: RustTokenKind::Group {
                        delimiter,
                        tokens: nested,
                    },
                    line,
                    raw_identifier: false,
                });
                continue;
            }
            if matches!(character, ')' | ']' | '}') {
                return Err(format!(
                    "unexpected closing delimiter `{character}` at line {}",
                    self.line
                ));
            }
            if character == 'r'
                && self.peek_byte(1) == Some(b'#')
                && self.source[self.index + 2..]
                    .chars()
                    .next()
                    .is_some_and(is_identifier_start)
            {
                self.index += 2;
                let start = self.index;
                self.bump_char();
                while self.peek_char().is_some_and(is_identifier_continue) {
                    self.bump_char();
                }
                tokens.push(RustToken {
                    kind: RustTokenKind::Ident(self.source[start..self.index].to_owned()),
                    line,
                    raw_identifier: true,
                });
                continue;
            }
            if is_identifier_start(character) {
                let start = self.index;
                self.bump_char();
                while self.peek_char().is_some_and(is_identifier_continue) {
                    self.bump_char();
                }
                tokens.push(RustToken {
                    kind: RustTokenKind::Ident(self.source[start..self.index].to_owned()),
                    line,
                    raw_identifier: false,
                });
                continue;
            }
            self.bump_char();
            tokens.push(RustToken {
                kind: RustTokenKind::Punct(character),
                line,
                raw_identifier: false,
            });
        }
        match closing {
            Some(expected) => Err(format!("unterminated `{expected}` group")),
            None => Ok(tokens),
        }
    }

    fn starts_with(&self, pattern: &str) -> bool {
        self.source[self.index..].starts_with(pattern)
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.index..].chars().next()
    }

    fn peek_byte(&self, offset: usize) -> Option<u8> {
        self.source.as_bytes().get(self.index + offset).copied()
    }

    fn bump_char(&mut self) -> Option<char> {
        let character = self.peek_char()?;
        self.index += character.len_utf8();
        if character == '\n' {
            self.line += 1;
        }
        Some(character)
    }

    fn skip_line_comment(&mut self) {
        let start = self.index + 2;
        self.index += 2;
        while self.index < self.source.len() && self.source.as_bytes()[self.index] != b'\n' {
            self.index += 1;
        }
        let comment = &self.source[start..self.index];
        if matches!(
            comment.trim(),
            "opi-phase17-acceptance" | "opi-runtime-acceptance"
        ) {
            self.acceptance_marker = true;
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), String> {
        let mut depth = 0_usize;
        while self.index < self.source.len() {
            if self.starts_with("/*") {
                depth += 1;
                self.index += 2;
            } else if self.starts_with("*/") {
                depth -= 1;
                self.index += 2;
                if depth == 0 {
                    return Ok(());
                }
            } else {
                self.bump_char();
            }
        }
        Err("unterminated block comment".to_owned())
    }

    fn raw_literal_prefix(&self) -> Option<(usize, usize)> {
        let bytes = self.source.as_bytes();
        let mut cursor = self.index;
        if bytes.get(cursor) == Some(&b'b') || bytes.get(cursor) == Some(&b'c') {
            if bytes.get(cursor + 1) != Some(&b'r') {
                return None;
            }
            cursor += 2;
        } else if bytes.get(cursor) == Some(&b'r') {
            cursor += 1;
        } else {
            return None;
        }
        let hash_start = cursor;
        while bytes.get(cursor) == Some(&b'#') {
            cursor += 1;
        }
        (bytes.get(cursor) == Some(&b'"')).then_some((cursor + 1 - self.index, cursor - hash_start))
    }

    fn skip_raw_literal(&mut self, prefix_len: usize, hashes: usize) -> Result<String, String> {
        self.index += prefix_len;
        let content_start = self.index;
        let bytes = self.source.as_bytes();
        while self.index < bytes.len() {
            if bytes[self.index] == b'\n' {
                self.line += 1;
            }
            if bytes[self.index] == b'"'
                && bytes
                    .get(self.index + 1..self.index + 1 + hashes)
                    .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
            {
                let literal = self.source[content_start..self.index].to_owned();
                self.index += 1 + hashes;
                return Ok(literal);
            }
            self.index += 1;
        }
        Err("unterminated raw literal".to_owned())
    }

    fn char_literal_end(&self, quote_index: usize) -> Option<usize> {
        let bytes = self.source.as_bytes();
        let mut cursor = quote_index + 1;
        if bytes.get(cursor) == Some(&b'\\') {
            cursor += 1;
            match bytes.get(cursor).copied()? {
                b'x' => cursor += 3,
                b'u' if bytes.get(cursor + 1) == Some(&b'{') => {
                    cursor += 2;
                    while bytes.get(cursor).is_some_and(u8::is_ascii_hexdigit) {
                        cursor += 1;
                    }
                    if bytes.get(cursor) != Some(&b'}') {
                        return None;
                    }
                    cursor += 1;
                }
                _ => cursor += 1,
            }
        } else {
            let character = self.source[cursor..].chars().next()?;
            if matches!(character, '\'' | '\n' | '\r') {
                return None;
            }
            cursor += character.len_utf8();
        }
        (bytes.get(cursor) == Some(&b'\'')).then_some(cursor)
    }

    fn skip_quoted_literal(&mut self, prefix_len: usize, quote: char) -> Result<String, String> {
        for _ in 0..prefix_len {
            self.bump_char();
        }
        if self.bump_char() != Some(quote) {
            return Err(format!("expected opening `{quote}` literal delimiter"));
        }
        let content_start = self.index;
        let mut escaped = false;
        while self.index < self.source.len() {
            let character_start = self.index;
            let character = self.bump_char().expect("index is in bounds");
            if character == quote && !escaped {
                return Ok(self.source[content_start..character_start].to_owned());
            }
            escaped = character == '\\' && !escaped;
            if character != '\\' {
                escaped = false;
            }
        }
        Err(format!("unterminated `{quote}` literal"))
    }
}

fn decode_cooked_literal(source: &str) -> String {
    let mut decoded = String::with_capacity(source.len());
    let mut characters = source.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        let Some(escape) = characters.next() else {
            decoded.push('\\');
            break;
        };
        match escape {
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            '0' => decoded.push('\0'),
            '\\' => decoded.push('\\'),
            '\'' => decoded.push('\''),
            '"' => decoded.push('"'),
            'x' => {
                let digits = characters.by_ref().take(2).collect::<String>();
                if let Ok(value) = u8::from_str_radix(&digits, 16) {
                    decoded.push(char::from(value));
                }
            }
            'u' if characters.next_if_eq(&'{').is_some() => {
                let digits = characters
                    .by_ref()
                    .take_while(|character| *character != '}')
                    .filter(|character| *character != '_')
                    .collect::<String>();
                if let Ok(value) = u32::from_str_radix(&digits, 16)
                    && let Some(character) = char::from_u32(value)
                {
                    decoded.push(character);
                }
            }
            '\n' => {
                while characters
                    .peek()
                    .is_some_and(|character| character.is_whitespace())
                {
                    characters.next();
                }
            }
            other => decoded.push(other),
        }
    }
    decoded
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    is_identifier_start(character) || character.is_numeric()
}

fn lex_rust(source: &str) -> LexedRust {
    RustLexer::new(source)
        .lex()
        .unwrap_or_else(|error| panic!("lex Rust source: {error}"))
}

fn token_is_keyword(token: &RustToken, keyword: &str) -> bool {
    !token.raw_identifier && matches!(&token.kind, RustTokenKind::Ident(ident) if ident == keyword)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TokenAtom {
    Ident(String),
    Punct(char),
}

fn token_atom(token: &RustToken) -> Option<TokenAtom> {
    match &token.kind {
        RustTokenKind::Ident(ident) => Some(TokenAtom::Ident(ident.clone())),
        RustTokenKind::Punct(punct) => Some(TokenAtom::Punct(*punct)),
        RustTokenKind::Literal(_) | RustTokenKind::Group { .. } => None,
    }
}

fn pattern_atoms(pattern: &str) -> Vec<TokenAtom> {
    let lexed = lex_rust(pattern);
    lexed
        .tokens
        .iter()
        .map(|token| {
            token_atom(token)
                .unwrap_or_else(|| panic!("token pattern must not contain a group: {pattern}"))
        })
        .collect()
}

fn occurrence_lines(tokens: &[RustToken], pattern: &[TokenAtom], lines: &mut Vec<usize>) {
    if !pattern.is_empty() && tokens.len() >= pattern.len() {
        for start in 0..=tokens.len() - pattern.len() {
            if pattern.iter().enumerate().all(|(offset, expected)| {
                token_atom(&tokens[start + offset]).as_ref() == Some(expected)
            }) {
                lines.push(tokens[start].line);
            }
        }
    }
    for token in tokens {
        if let RustTokenKind::Group { tokens, .. } = &token.kind {
            occurrence_lines(tokens, pattern, lines);
        }
    }
}

fn group_contains_ident(tokens: &[RustToken], expected: &str) -> bool {
    tokens.iter().any(|token| match &token.kind {
        RustTokenKind::Ident(ident) => ident == expected,
        RustTokenKind::Group { tokens, .. } => group_contains_ident(tokens, expected),
        RustTokenKind::Literal(_) | RustTokenKind::Punct(_) => false,
    })
}

fn enum_body_declares_variant(tokens: &[RustToken], variant: &str) -> bool {
    comma_separated(tokens).into_iter().any(|declaration| {
        declaration.iter().find_map(|token| match &token.kind {
            RustTokenKind::Ident(ident) => Some(ident.as_str()),
            _ => None,
        }) == Some(variant)
    })
}

fn body_declares_associated_const(tokens: &[RustToken], name: &str) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        token_is_keyword(token, "const")
            && tokens[index + 1..]
                .iter()
                .find_map(|candidate| match &candidate.kind {
                    RustTokenKind::Ident(ident) => Some(ident.as_str()),
                    RustTokenKind::Punct(';') | RustTokenKind::Punct('=') => Some(""),
                    _ => None,
                })
                == Some(name)
    })
}

fn inherent_impl_declares_associated_const(
    tokens: &[RustToken],
    impl_index: usize,
    target: &str,
    name: &str,
) -> bool {
    let Some(body_offset) = tokens[impl_index + 1..].iter().position(|token| {
        matches!(
            token.kind,
            RustTokenKind::Group { delimiter: '{', .. } | RustTokenKind::Punct(';')
        )
    }) else {
        return false;
    };
    let body_index = impl_index + 1 + body_offset;
    let header = &tokens[impl_index + 1..body_index];
    if header.iter().any(|token| token_is_keyword(token, "for")) {
        return false;
    }
    let mut angle_depth = 0_usize;
    let mut inherent_target = None;
    for token in header
        .iter()
        .take_while(|token| !token_is_keyword(token, "where"))
    {
        match &token.kind {
            RustTokenKind::Punct('<') => angle_depth += 1,
            RustTokenKind::Punct('>') => angle_depth = angle_depth.saturating_sub(1),
            RustTokenKind::Ident(ident) if angle_depth == 0 => {
                inherent_target = Some(ident.as_str());
            }
            _ => {}
        }
    }
    if inherent_target != Some(target) {
        return false;
    }
    matches!(
        &tokens[body_index].kind,
        RustTokenKind::Group {
            delimiter: '{',
            tokens: body,
        } if body_declares_associated_const(body, name)
    )
}

fn removed_variant_special_lines(
    tokens: &[RustToken],
    enum_name: &str,
    variant: &str,
    lines: &mut Vec<usize>,
) {
    for (index, token) in tokens.iter().enumerate() {
        if matches!(&token.kind, RustTokenKind::Ident(ident) if ident == enum_name)
            && matches!(
                tokens.get(index + 1).map(|token| &token.kind),
                Some(RustTokenKind::Punct(':'))
            )
            && matches!(
                tokens.get(index + 2).map(|token| &token.kind),
                Some(RustTokenKind::Punct(':'))
            )
            && let Some(RustToken {
                kind:
                    RustTokenKind::Group {
                        tokens: grouped, ..
                    },
                ..
            }) = tokens.get(index + 3)
            && group_contains_ident(grouped, variant)
        {
            lines.push(token.line);
        }
        if token_is_keyword(token, "enum")
            && matches!(
                tokens.get(index + 1).map(|token| &token.kind),
                Some(RustTokenKind::Ident(ident)) if ident == enum_name
            )
            && let Some(body) = enum_declaration_body(&tokens[index + 2..])
            && enum_body_declares_variant(body, variant)
        {
            lines.push(token.line);
        }
        if token_is_keyword(token, "impl")
            && inherent_impl_declares_associated_const(tokens, index, enum_name, variant)
        {
            lines.push(token.line);
        }
        if let RustTokenKind::Group { tokens, .. } = &token.kind {
            removed_variant_special_lines(tokens, enum_name, variant, lines);
        }
    }
}

fn enum_declaration_body(tokens: &[RustToken]) -> Option<&[RustToken]> {
    let mut angle_depth = 0_usize;
    for token in tokens {
        match &token.kind {
            RustTokenKind::Punct('<') => angle_depth += 1,
            RustTokenKind::Punct('>') => angle_depth = angle_depth.saturating_sub(1),
            RustTokenKind::Punct(';') if angle_depth == 0 => return None,
            RustTokenKind::Group {
                delimiter: '{',
                tokens: body,
            } if angle_depth == 0 => return Some(body),
            _ => {}
        }
    }
    None
}

fn source_token_occurrence_lines(source: &str, pattern: &str) -> Vec<usize> {
    let lexed = lex_rust(source);
    let atoms = pattern_atoms(pattern);
    let mut lines = Vec::new();
    occurrence_lines(&lexed.tokens, &atoms, &mut lines);
    if let [
        TokenAtom::Ident(enum_name),
        TokenAtom::Punct(':'),
        TokenAtom::Punct(':'),
        TokenAtom::Ident(variant),
    ] = atoms.as_slice()
    {
        removed_variant_special_lines(&lexed.tokens, enum_name, variant, &mut lines);
    }
    lines
}

pub(super) fn source_token_occurrences(source: &str, pattern: &str) -> usize {
    source_token_occurrence_lines(source, pattern).len()
}

fn relative_source_path(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(super) fn token_sites(path: &Path, source: &str, token: &str) -> Vec<String> {
    let relative = relative_source_path(path);
    source_token_occurrence_lines(source, token)
        .into_iter()
        .map(|line_number| {
            let line = source.lines().nth(line_number - 1).unwrap_or_default();
            format!("{relative}: {}", line.trim())
        })
        .collect()
}

pub(super) fn acceptance_manifest_difference(
    expected: &[&str],
    actual: &[&str],
) -> Result<(), String> {
    let expected_set = expected.iter().copied().collect::<BTreeSet<_>>();
    let actual_set = actual.iter().copied().collect::<BTreeSet<_>>();
    if expected_set.len() != expected.len() {
        return Err("acceptance manifest contains duplicate entries".to_owned());
    }
    if actual_set.len() != actual.len() {
        return Err("discovered acceptance sources contain duplicate entries".to_owned());
    }
    let missing = expected_set
        .difference(&actual_set)
        .copied()
        .collect::<Vec<_>>();
    let unexpected = actual_set
        .difference(&expected_set)
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() && unexpected.is_empty() {
        Ok(())
    } else {
        Err(format!("missing={missing:?}, unexpected={unexpected:?}"))
    }
}

fn attribute_path_last_ident(tokens: &[RustToken]) -> Option<&str> {
    let mut last_ident = None;
    for token in tokens {
        match &token.kind {
            RustTokenKind::Ident(ident) => last_ident = Some(ident.as_str()),
            RustTokenKind::Punct(':') => {}
            RustTokenKind::Literal(_) | RustTokenKind::Group { .. } | RustTokenKind::Punct(_) => {
                break;
            }
        }
    }
    last_ident
}

fn attribute_path_ends_in_test(tokens: &[RustToken]) -> bool {
    attribute_path_last_ident(tokens) == Some("test")
}

fn attribute_or_cfg_attr_matches(
    attribute: &[RustToken],
    predicate: fn(&[RustToken]) -> bool,
) -> bool {
    predicate(attribute)
        || (attribute_path_last_ident(attribute) == Some("cfg_attr")
            && attribute
                .iter()
                .find_map(|token| match &token.kind {
                    RustTokenKind::Group {
                        delimiter: '(',
                        tokens,
                    } => Some(tokens),
                    _ => None,
                })
                .is_some_and(|arguments| {
                    comma_separated(arguments)
                        .into_iter()
                        .skip(1)
                        .any(|applied| attribute_or_cfg_attr_matches(applied, predicate))
                }))
}

fn test_function_names(tokens: &[RustToken], names: &mut Vec<String>) {
    let mut index = 0;
    while index < tokens.len() {
        let RustTokenKind::Punct('#') = tokens[index].kind else {
            if let RustTokenKind::Group { tokens, .. } = &tokens[index].kind {
                test_function_names(tokens, names);
            }
            index += 1;
            continue;
        };
        let Some(next) = tokens.get(index + 1) else {
            break;
        };
        let RustTokenKind::Group {
            delimiter: '[',
            tokens: attribute,
        } = &next.kind
        else {
            index += 1;
            continue;
        };
        if !attribute_or_cfg_attr_matches(attribute, attribute_path_ends_in_test) {
            index += 2;
            continue;
        }
        let mut cursor = index + 2;
        while cursor < tokens.len() {
            match &tokens[cursor].kind {
                RustTokenKind::Ident(ident) if ident == "fn" => {
                    if let Some(RustToken {
                        kind: RustTokenKind::Ident(name),
                        ..
                    }) = tokens.get(cursor + 1)
                    {
                        names.push(name.clone());
                    }
                    break;
                }
                RustTokenKind::Group { delimiter: '{', .. } | RustTokenKind::Punct(';') => break,
                _ => cursor += 1,
            }
        }
        index += 2;
    }
}

fn phase17_module_has_tests(tokens: &[RustToken]) -> bool {
    for window in tokens.windows(3) {
        let [
            RustToken {
                kind: RustTokenKind::Ident(module),
                ..
            },
            RustToken {
                kind: RustTokenKind::Ident(name),
                ..
            },
            RustToken {
                kind:
                    RustTokenKind::Group {
                        delimiter: '{',
                        tokens: body,
                    },
                ..
            },
        ] = window
        else {
            continue;
        };
        if module == "mod" && name == "phase17" {
            let mut tests = Vec::new();
            test_function_names(body, &mut tests);
            if !tests.is_empty() {
                return true;
            }
        }
    }
    tokens.iter().any(|token| match &token.kind {
        RustTokenKind::Group { tokens, .. } => phase17_module_has_tests(tokens),
        _ => false,
    })
}

fn source_is_runtime_acceptance(path: &Path, source: &str) -> bool {
    let lexed = RustLexer::new(source)
        .lex()
        .unwrap_or_else(|error| panic!("lex Rust source {}: {error}", path.display()));
    let mut tests = Vec::new();
    test_function_names(&lexed.tokens, &mut tests);
    let named_source = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("phase17"));
    named_source
        || tests.iter().any(|name| name.starts_with("phase17_"))
        || phase17_module_has_tests(&lexed.tokens)
        || lexed.acceptance_marker
}

pub(super) fn discover_acceptance_sources_under(
    root: &Path,
    source_roots: &[PathBuf],
) -> Vec<String> {
    let mut files = Vec::new();
    for source_root in source_roots {
        collect_rust_files(source_root, &mut files);
    }
    let mut discovered = files
        .into_iter()
        .filter(|path| {
            let source = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("read source {}: {error}", path.display()));
            source_is_runtime_acceptance(path, &source)
        })
        .map(|path| {
            path.strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    discovered.sort();
    discovered
}

fn comma_separated(tokens: &[RustToken]) -> Vec<&[RustToken]> {
    let mut fields = Vec::new();
    let mut start = 0;
    for (index, token) in tokens.iter().enumerate() {
        if matches!(token.kind, RustTokenKind::Punct(',')) {
            fields.push(&tokens[start..index]);
            start = index + 1;
        }
    }
    fields.push(&tokens[start..]);
    fields
}

fn attribute_can_gate_test(attribute: &[RustToken]) -> bool {
    attribute_or_cfg_attr_matches(attribute, |candidate| {
        matches!(
            attribute_path_last_ident(candidate),
            Some("ignore" | "cfg" | "test")
        )
    })
}

fn cfg_is_test_only(tokens: &[RustToken]) -> bool {
    let Some(RustToken {
        kind:
            RustTokenKind::Group {
                delimiter: '(',
                tokens: condition,
            },
        ..
    }) = tokens
        .iter()
        .find(|token| matches!(token.kind, RustTokenKind::Group { .. }))
    else {
        return false;
    };
    matches!(
        condition.as_slice(),
        [RustToken {
            kind: RustTokenKind::Ident(ident),
            ..
        }] if ident == "test"
    )
}

fn collect_attribute_violations(tokens: &[RustToken], violations: &mut Vec<String>) {
    let mut index = 0;
    while index < tokens.len() {
        if matches!(tokens[index].kind, RustTokenKind::Punct('#')) {
            let group_index = index
                + usize::from(matches!(
                    tokens.get(index + 1).map(|token| &token.kind),
                    Some(RustTokenKind::Punct('!'))
                ))
                + 1;
            if let Some(RustToken {
                kind:
                    RustTokenKind::Group {
                        delimiter: '[',
                        tokens: attribute,
                    },
                line,
                ..
            }) = tokens.get(group_index)
            {
                match attribute_path_last_ident(attribute) {
                    Some("ignore") => {
                        violations.push(format!("test-disable attribute `ignore` at line {line}"))
                    }
                    Some("cfg_attr") => {
                        if attribute_can_gate_test(attribute) {
                            violations
                                .push(format!("conditional test gating `cfg_attr` at line {line}"));
                        }
                    }
                    Some("cfg") if !cfg_is_test_only(attribute) => violations.push(format!(
                        "conditional-compilation attribute `cfg` at line {line}"
                    )),
                    _ => {}
                }
            }
        }
        if let RustTokenKind::Group { tokens, .. } = &tokens[index].kind {
            collect_attribute_violations(tokens, violations);
        }
        index += 1;
    }
}

pub(super) fn acceptance_source_violations(_relative: &str, source: &str) -> Vec<String> {
    let mut violations = Vec::new();
    collect_attribute_violations(&lex_rust(source).tokens, &mut violations);
    violations
}

fn call_name_before_group(tokens: &[RustToken], group_index: usize) -> Option<&str> {
    tokens[..group_index]
        .iter()
        .rev()
        .find_map(|token| match &token.kind {
            RustTokenKind::Ident(ident) => Some(ident.as_str()),
            RustTokenKind::Punct(':') | RustTokenKind::Punct('!') => None,
            _ => Some(""),
        })
        .filter(|name| !name.is_empty())
}

fn is_observation_call(name: &str) -> bool {
    matches!(
        name,
        "assert"
            | "assert_eq"
            | "assert_ne"
            | "debug_assert"
            | "matches"
            | "panic"
            | "contains"
            | "starts_with"
            | "ends_with"
            | "redact"
            | "safe_excerpt"
    )
}

fn binding_name_before_literal(tokens: &[RustToken], literal_index: usize) -> Option<&str> {
    let statement_start = tokens[..literal_index]
        .iter()
        .rposition(|token| matches!(token.kind, RustTokenKind::Punct(';')))
        .map_or(0, |index| index + 1);
    let statement = &tokens[statement_start..literal_index];
    let binding_keyword = statement.iter().position(|token| {
        matches!(
            &token.kind,
            RustTokenKind::Ident(ident) if matches!(ident.as_str(), "let" | "const" | "static")
        )
    })?;
    statement[binding_keyword + 1..]
        .iter()
        .find_map(|token| match &token.kind {
            RustTokenKind::Ident(ident) if ident != "mut" => Some(ident.as_str()),
            _ => None,
        })
}

struct SimpleBinding<'a> {
    expression: &'a [RustToken],
    declaration_position: usize,
    mutable: bool,
}

fn simple_binding<'a>(
    tokens: &'a [RustToken],
    before: usize,
    expected: &str,
) -> Option<SimpleBinding<'a>> {
    let mut statement_start = 0;
    let mut binding = None;
    for statement_end in 0..before {
        if !matches!(tokens[statement_end].kind, RustTokenKind::Punct(';')) {
            continue;
        }
        let current_statement_start = statement_start;
        let statement = &tokens[current_statement_start..statement_end];
        statement_start = statement_end + 1;
        let Some(binding_keyword) = statement.iter().position(|token| {
            ["let", "const", "static"]
                .iter()
                .any(|keyword| token_is_keyword(token, keyword))
        }) else {
            continue;
        };
        let Some((name_index, name)) = statement[binding_keyword + 1..]
            .iter()
            .enumerate()
            .find_map(|(offset, token)| match &token.kind {
                RustTokenKind::Ident(ident) if !token_is_keyword(token, "mut") => {
                    Some((binding_keyword + 1 + offset, ident.as_str()))
                }
                _ => None,
            })
        else {
            continue;
        };
        if name != expected {
            continue;
        }
        let Some(equals_offset) = statement[name_index + 1..]
            .iter()
            .position(|token| matches!(token.kind, RustTokenKind::Punct('=')))
        else {
            continue;
        };
        binding = Some(SimpleBinding {
            expression: &statement[name_index + 2 + equals_offset..],
            declaration_position: current_statement_start + name_index,
            mutable: statement[binding_keyword + 1..name_index]
                .iter()
                .any(|token| token_is_keyword(token, "mut")),
        });
    }
    binding
}

fn binding_is_reassigned(
    tokens: &[RustToken],
    declaration_position: usize,
    before: usize,
    expected: &str,
) -> bool {
    (declaration_position + 1..before).any(|index| {
        matches!(&tokens[index].kind, RustTokenKind::Ident(ident) if ident == expected)
            && matches!(
                tokens.get(index + 1).map(|token| &token.kind),
                Some(RustTokenKind::Punct('='))
            )
            && !matches!(
                tokens.get(index + 2).map(|token| &token.kind),
                Some(RustTokenKind::Punct('='))
            )
            && !matches!(
                tokens.get(index.wrapping_sub(1)).map(|token| &token.kind),
                Some(RustTokenKind::Punct('.') | RustTokenKind::Punct(':'))
            )
    })
}

fn executable_call_uses_ident(tokens: &[RustToken], expected: &str) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        let RustTokenKind::Group { tokens: body, .. } = &token.kind else {
            return false;
        };
        let call_name = call_name_before_group(tokens, index);
        let direct_use = call_name.is_some_and(|name| !is_observation_call(name))
            && group_contains_ident(body, expected);
        direct_use || executable_call_uses_ident(body, expected)
    })
}

fn url_host(literal: &str) -> Option<String> {
    let literal = literal.trim();
    let scheme_len = if literal
        .get(..8)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
    {
        8
    } else if literal
        .get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
    {
        7
    } else {
        return None;
    };
    let authority = literal[scheme_len..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default();
    if authority.starts_with('[') {
        return authority
            .split_once(']')
            .map(|(host, _)| host.trim_start_matches('[').to_ascii_lowercase())
            .filter(|host| !host.is_empty());
    }
    let host = authority
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

fn is_observation_fixture_host(host: &str) -> bool {
    if host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    if host
        .parse::<std::net::IpAddr>()
        .is_ok_and(|address| address.is_loopback())
    {
        return true;
    }
    ["test", "example", "invalid"]
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
        || ["example.com", "example.net", "example.org"]
            .iter()
            .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
}

fn constant_expression(tokens: &[RustToken]) -> Option<String> {
    match tokens {
        [
            RustToken {
                kind: RustTokenKind::Literal(literal),
                ..
            },
        ] => Some(literal.clone()),
        [
            RustToken {
                kind:
                    RustTokenKind::Group {
                        delimiter: '(',
                        tokens,
                    },
                ..
            },
        ] => constant_expression(tokens),
        [
            RustToken {
                kind: RustTokenKind::Ident(name),
                ..
            },
            RustToken {
                kind: RustTokenKind::Punct('!'),
                ..
            },
            RustToken {
                kind:
                    RustTokenKind::Group {
                        delimiter: '(',
                        tokens,
                    },
                ..
            },
        ] => constant_macro_literal(name, tokens),
        _ => None,
    }
}

fn constant_macro_arguments(tokens: &[RustToken]) -> Option<Vec<String>> {
    comma_separated(tokens)
        .into_iter()
        .map(constant_expression)
        .collect()
}

fn render_literal_format(
    template: &str,
    positional: &[String],
    named: &[(String, String)],
) -> Option<String> {
    let mut rendered = String::with_capacity(template.len());
    let mut cursor = 0;
    let mut next_implicit = 0;
    while cursor < template.len() {
        let remainder = &template[cursor..];
        if remainder.starts_with("{{") {
            rendered.push('{');
            cursor += 2;
            continue;
        }
        if remainder.starts_with("}}") {
            rendered.push('}');
            cursor += 2;
            continue;
        }
        if remainder.starts_with('{') {
            let close = remainder.find('}')?;
            let (field, format_spec) = format_field(&remainder[1..close]);
            if format_spec.is_some_and(|spec| !spec.is_empty()) {
                return None;
            }
            let value = if field.is_empty() {
                let value = positional.get(next_implicit)?.as_str();
                next_implicit += 1;
                value
            } else if let Ok(index) = field.parse::<usize>() {
                positional.get(index)?.as_str()
            } else {
                named
                    .iter()
                    .find_map(|(name, value)| (name == field).then_some(value.as_str()))?
            };
            rendered.push_str(value);
            cursor += close + 1;
            continue;
        }
        if remainder.starts_with('}') {
            return None;
        }
        let character = remainder.chars().next().expect("remainder is non-empty");
        rendered.push(character);
        cursor += character.len_utf8();
    }
    Some(rendered)
}

fn format_field(field: &str) -> (&str, Option<&str>) {
    field
        .split_once(':')
        .map_or((field, None), |(field, spec)| (field, Some(spec)))
}

fn constant_macro_literal(name: &str, tokens: &[RustToken]) -> Option<String> {
    match name {
        "concat" => Some(constant_macro_arguments(tokens)?.concat()),
        "format" => {
            let mut arguments = comma_separated(tokens).into_iter();
            let template = constant_expression(arguments.next()?)?;
            let mut positional = Vec::new();
            let mut named = Vec::new();
            for argument in arguments {
                match argument {
                    [
                        RustToken {
                            kind: RustTokenKind::Ident(name),
                            ..
                        },
                        RustToken {
                            kind: RustTokenKind::Punct('='),
                            ..
                        },
                        value @ ..,
                    ] => named.push((name.clone(), constant_expression(value)?)),
                    _ => positional.push(constant_expression(argument)?),
                }
            }
            render_literal_format(&template, &positional, &named)
        }
        _ => None,
    }
}

fn url_authority_bounds(url: &str) -> Option<(usize, usize)> {
    let trimmed = url.trim_start();
    let start = url.len() - trimmed.len();
    let scheme_len = if trimmed
        .get(..8)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
    {
        8
    } else if trimmed
        .get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
    {
        7
    } else {
        return None;
    };
    let authority_start = start + scheme_len;
    let authority_end = url[authority_start..]
        .find(['/', '?', '#'])
        .map_or(url.len(), |offset| authority_start + offset);
    Some((authority_start, authority_end))
}

fn authority_port_start(url: &str, authority: (usize, usize)) -> Option<usize> {
    let (authority_start, authority_end) = authority;
    let authority = &url[authority_start..authority_end];
    let userinfo_end = authority.rfind('@').map_or(0, |index| index + 1);
    let host_port = &authority[userinfo_end..];
    let colon = if host_port.starts_with('[') {
        let closing_bracket = host_port.find(']')?;
        host_port[closing_bracket + 1..]
            .starts_with(':')
            .then_some(closing_bracket + 1)
    } else {
        host_port.find(':')
    }?;
    Some(authority_start + userinfo_end + colon + 1)
}

fn is_loopback_host(host: &str) -> bool {
    host == "localhost"
        || host.ends_with(".localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn expression_is_proven_port(tokens: &[RustToken]) -> bool {
    if let [
        RustToken {
            kind:
                RustTokenKind::Group {
                    delimiter: '(',
                    tokens,
                },
            ..
        },
    ] = tokens
    {
        return expression_is_proven_port(tokens);
    }
    if !tokens.is_empty()
        && tokens.iter().all(|token| {
            matches!(token.kind, RustTokenKind::Punct(character) if character.is_ascii_digit() || character == '_')
        })
    {
        return true;
    }
    if tokens.len() < 4 {
        return false;
    }
    matches!(
        tokens.get(tokens.len() - 3).map(|token| &token.kind),
        Some(RustTokenKind::Punct('.'))
    ) && matches!(
        tokens.get(tokens.len() - 2).map(|token| &token.kind),
        Some(RustTokenKind::Ident(name)) if name == "port"
    ) && matches!(
        tokens.last().map(|token| &token.kind),
        Some(RustTokenKind::Group {
            delimiter: '(',
            tokens,
        }) if tokens.is_empty()
    )
}

#[derive(Debug)]
struct UnresolvedFormatField<'a> {
    start: usize,
    end: usize,
    expression: Option<&'a [RustToken]>,
}

fn format_argument<'a>(
    field: &str,
    next_implicit: &mut usize,
    positional: &[&'a [RustToken]],
    named: &[(&str, &'a [RustToken])],
) -> Option<&'a [RustToken]> {
    if field.is_empty() {
        let argument = positional.get(*next_implicit).copied();
        *next_implicit += 1;
        argument
    } else if let Ok(index) = field.parse::<usize>() {
        positional.get(index).copied()
    } else {
        named
            .iter()
            .find_map(|(name, value)| (*name == field).then_some(*value))
    }
}

fn partially_render_format<'a>(
    template: &str,
    positional: &[&'a [RustToken]],
    named: &[(&str, &'a [RustToken])],
) -> Option<(String, Vec<UnresolvedFormatField<'a>>)> {
    let mut rendered = String::with_capacity(template.len());
    let mut unresolved = Vec::new();
    let mut cursor = 0;
    let mut next_implicit = 0;
    while cursor < template.len() {
        let remainder = &template[cursor..];
        if remainder.starts_with("{{") {
            rendered.push('{');
            cursor += 2;
            continue;
        }
        if remainder.starts_with("}}") {
            rendered.push('}');
            cursor += 2;
            continue;
        }
        if remainder.starts_with('{') {
            let close = remainder.find('}')?;
            let (field, format_spec) = format_field(&remainder[1..close]);
            let expression = format_argument(field, &mut next_implicit, positional, named);
            if format_spec.is_none_or(str::is_empty)
                && let Some(value) = expression.and_then(constant_expression)
            {
                rendered.push_str(&value);
            } else {
                let start = rendered.len();
                rendered.push('0');
                unresolved.push(UnresolvedFormatField {
                    start,
                    end: rendered.len(),
                    expression,
                });
            }
            cursor += close + 1;
            continue;
        }
        if remainder.starts_with('}') {
            return None;
        }
        let character = remainder.chars().next().expect("remainder is non-empty");
        rendered.push(character);
        cursor += character.len_utf8();
    }
    Some((rendered, unresolved))
}

fn unresolved_format_proves_fixture_host(tokens: &[RustToken]) -> bool {
    let mut arguments = comma_separated(tokens).into_iter();
    let Some(template) = arguments.next().and_then(constant_expression) else {
        return false;
    };
    let mut positional = Vec::new();
    let mut named = Vec::new();
    for argument in arguments {
        match argument {
            [
                RustToken {
                    kind: RustTokenKind::Ident(name),
                    ..
                },
                RustToken {
                    kind: RustTokenKind::Punct('='),
                    ..
                },
                value @ ..,
            ] => named.push((name.as_str(), value)),
            _ => positional.push(argument),
        }
    }
    let Some((rendered, unresolved)) = partially_render_format(&template, &positional, &named)
    else {
        return false;
    };
    let Some(authority) = url_authority_bounds(&rendered) else {
        return false;
    };
    let Some(host) = url_host(&rendered) else {
        return false;
    };
    if !is_loopback_host(&host) {
        return false;
    }
    let port_start = authority_port_start(&rendered, authority);
    unresolved.iter().all(|field| {
        if field.start >= authority.1 {
            return true;
        }
        is_loopback_host(&host)
            && port_start
                .is_some_and(|port_start| field.start >= port_start && field.end <= authority.1)
            && field.expression.is_some_and(expression_is_proven_port)
    })
}

fn unresolved_concat_proves_fixture_host(tokens: &[RustToken]) -> bool {
    let arguments = comma_separated(tokens);
    let mut prefix = String::new();
    for (index, argument) in arguments.iter().enumerate() {
        if let Some(fragment) = constant_expression(argument) {
            prefix.push_str(&fragment);
            continue;
        }
        let Some(authority) = url_authority_bounds(&prefix) else {
            return false;
        };
        let Some(host) = url_host(&prefix) else {
            return false;
        };
        if !is_loopback_host(&host) {
            return false;
        }
        if authority.1 < prefix.len() {
            return true;
        }
        return index + 1 == arguments.len()
            && prefix.ends_with(':')
            && is_loopback_host(&host)
            && authority_port_start(&prefix, authority) == Some(prefix.len())
            && expression_is_proven_port(argument);
    }
    false
}

fn unresolved_macro_proves_fixture_host(name: &str, tokens: &[RustToken]) -> bool {
    match name {
        "format" => unresolved_format_proves_fixture_host(tokens),
        "concat" => unresolved_concat_proves_fixture_host(tokens),
        _ => false,
    }
}

#[derive(Clone, Default)]
struct FixtureScope {
    server_parameters: BTreeSet<String>,
    url_parameters: BTreeSet<String>,
}

fn expression_constructs_mock_server(tokens: &[RustToken]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        matches!(token.kind, RustTokenKind::Group { delimiter: '(', .. })
            && call_name_before_group(tokens, index) == Some("start")
            && qualified_type_before_call(tokens, index) == Some("MockServer")
    })
}

fn receiver_is_proven_mock_server(
    receiver: &str,
    scope: &[RustToken],
    before: usize,
    fixture_scope: &FixtureScope,
) -> bool {
    fixture_scope.server_parameters.contains(receiver)
        || simple_binding(scope, before, receiver).is_some_and(|binding| {
            !binding.mutable
                && !binding_is_reassigned(scope, binding.declaration_position, before, receiver)
                && expression_constructs_mock_server(binding.expression)
        })
}

struct FunctionParameter<'a> {
    name: &'a str,
    parameter_type: &'a [RustToken],
}

struct FunctionDeclaration<'a> {
    name: &'a str,
    parameters: Vec<FunctionParameter<'a>>,
}

fn function_declaration_before_body(
    tokens: &[RustToken],
    body_index: usize,
) -> Option<FunctionDeclaration<'_>> {
    if !matches!(
        tokens.get(body_index).map(|token| &token.kind),
        Some(RustTokenKind::Group { delimiter: '{', .. })
    ) {
        return None;
    }
    let function_index = tokens[..body_index]
        .iter()
        .rposition(|token| token_is_keyword(token, "fn"))?;
    let RustTokenKind::Ident(name) = &tokens.get(function_index + 1)?.kind else {
        return None;
    };
    let parameter_index = (function_index + 2..body_index).find(|index| {
        matches!(
            tokens[*index].kind,
            RustTokenKind::Group { delimiter: '(', .. }
        )
    })?;
    let actual_body_index = (parameter_index + 1..=body_index).find(|index| {
        matches!(
            tokens[*index].kind,
            RustTokenKind::Group { delimiter: '{', .. }
        )
    })?;
    if actual_body_index != body_index {
        return None;
    }
    let RustTokenKind::Group {
        tokens: parameters, ..
    } = &tokens[parameter_index].kind
    else {
        unreachable!("parameter group was matched above")
    };
    let parameters = comma_separated(parameters)
        .into_iter()
        .filter_map(|parameter| {
            let colon = parameter
                .iter()
                .position(|token| matches!(token.kind, RustTokenKind::Punct(':')))?;
            let name = parameter[..colon]
                .iter()
                .rev()
                .find_map(|token| match &token.kind {
                    RustTokenKind::Ident(name)
                        if !matches!(name.as_str(), "mut" | "ref" | "self") =>
                    {
                        Some(name.as_str())
                    }
                    _ => None,
                })?;
            Some(FunctionParameter {
                name,
                parameter_type: &parameter[colon + 1..],
            })
        })
        .collect();
    Some(FunctionDeclaration { name, parameters })
}

fn collect_function_argument_proofs(
    tokens: &[RustToken],
    function_name: &str,
    argument_index: usize,
    seen: &mut bool,
) -> bool {
    for (index, token) in tokens.iter().enumerate() {
        let RustTokenKind::Group { tokens: body, .. } = &token.kind else {
            continue;
        };
        let declaration_parameters = index >= 2
            && token_is_keyword(&tokens[index - 2], "fn")
            && matches!(
                &tokens[index - 1].kind,
                RustTokenKind::Ident(name) if name == function_name
            );
        if !declaration_parameters && call_name_before_group(tokens, index) == Some(function_name) {
            *seen = true;
            let Some(argument) = comma_separated(body).get(argument_index).copied() else {
                return false;
            };
            if !expression_proves_fixture_url(argument, tokens, index, 0, &FixtureScope::default())
            {
                return false;
            }
        }
        if !collect_function_argument_proofs(body, function_name, argument_index, seen) {
            return false;
        }
    }
    true
}

fn function_fixture_scope(
    tokens: &[RustToken],
    body_index: usize,
    root: &[RustToken],
) -> Option<FixtureScope> {
    let declaration = function_declaration_before_body(tokens, body_index)?;
    let mut fixture_scope = FixtureScope::default();
    for (index, parameter) in declaration.parameters.into_iter().enumerate() {
        if parameter
            .parameter_type
            .iter()
            .any(|token| matches!(&token.kind, RustTokenKind::Ident(kind) if kind == "MockServer"))
        {
            fixture_scope
                .server_parameters
                .insert(parameter.name.to_owned());
        }
        if parameter.name.ends_with("_uri") || parameter.name.ends_with("_url") {
            let mut seen = false;
            if collect_function_argument_proofs(root, declaration.name, index, &mut seen) && seen {
                fixture_scope
                    .url_parameters
                    .insert(parameter.name.to_owned());
            }
        }
    }
    Some(fixture_scope)
}

fn expression_proves_fixture_url(
    expression: &[RustToken],
    scope: &[RustToken],
    before: usize,
    depth: usize,
    fixture_scope: &FixtureScope,
) -> bool {
    if depth > 4 {
        return false;
    }
    if let Some(literal) = constant_expression(expression) {
        return url_host(&literal).is_some_and(|host| is_loopback_host(&host));
    }
    if let [
        RustToken {
            kind: RustTokenKind::Ident(name),
            ..
        },
        RustToken {
            kind: RustTokenKind::Punct('!'),
            ..
        },
        RustToken {
            kind: RustTokenKind::Group { tokens, .. },
            ..
        },
    ] = expression
        && unresolved_macro_proves_fixture_host(name, tokens)
    {
        return true;
    }
    match expression {
        [
            RustToken {
                kind: RustTokenKind::Ident(name),
                ..
            },
        ] => simple_binding(scope, before, name).map_or_else(
            || fixture_scope.url_parameters.contains(name),
            |binding| {
                !binding.mutable
                    && !binding_is_reassigned(scope, binding.declaration_position, before, name)
                    && expression_proves_fixture_url(
                        binding.expression,
                        scope,
                        binding.declaration_position,
                        depth + 1,
                        fixture_scope,
                    )
            },
        ),
        [
            RustToken {
                kind:
                    RustTokenKind::Group {
                        delimiter: '(',
                        tokens,
                    },
                ..
            },
        ] => expression_proves_fixture_url(tokens, scope, before, depth + 1, fixture_scope),
        [
            RustToken {
                kind: RustTokenKind::Ident(wrapper),
                ..
            },
            RustToken {
                kind:
                    RustTokenKind::Group {
                        delimiter: '(',
                        tokens,
                    },
                ..
            },
        ] if wrapper == "Some" => {
            expression_proves_fixture_url(tokens, scope, before, depth + 1, fixture_scope)
        }
        [
            RustToken {
                kind: RustTokenKind::Ident(receiver),
                ..
            },
            RustToken {
                kind: RustTokenKind::Punct('.'),
                ..
            },
            RustToken {
                kind: RustTokenKind::Ident(method),
                ..
            },
            RustToken {
                kind:
                    RustTokenKind::Group {
                        delimiter: '(',
                        tokens,
                    },
                ..
            },
        ] if tokens.is_empty()
            && method == "uri"
            && receiver_is_proven_mock_server(receiver, scope, before, fixture_scope) =>
        {
            true
        }
        [
            receiver @ ..,
            RustToken {
                kind: RustTokenKind::Punct('.'),
                ..
            },
            RustToken {
                kind: RustTokenKind::Ident(conversion),
                ..
            },
            RustToken {
                kind:
                    RustTokenKind::Group {
                        delimiter: '(',
                        tokens,
                    },
                ..
            },
        ] if tokens.is_empty()
            && matches!(conversion.as_str(), "into" | "to_owned" | "to_string") =>
        {
            expression_proves_fixture_url(receiver, scope, before, depth + 1, fixture_scope)
        }
        _ => false,
    }
}

fn qualified_type_before_call(tokens: &[RustToken], group_index: usize) -> Option<&str> {
    match tokens.get(group_index.checked_sub(4)?)?.kind {
        RustTokenKind::Ident(ref qualifier)
            if matches!(
                tokens.get(group_index - 3).map(|token| &token.kind),
                Some(RustTokenKind::Punct(':'))
            ) && matches!(
                tokens.get(group_index - 2).map(|token| &token.kind),
                Some(RustTokenKind::Punct(':'))
            ) =>
        {
            Some(qualifier.as_str())
        }
        _ => None,
    }
}

fn member_receiver_before_call(tokens: &[RustToken], group_index: usize) -> Option<&str> {
    if !matches!(
        tokens.get(group_index.checked_sub(2)?)?.kind,
        RustTokenKind::Punct('.')
    ) {
        return None;
    }
    match &tokens.get(group_index.checked_sub(3)?)?.kind {
        RustTokenKind::Ident(receiver) => Some(receiver.as_str()),
        _ => None,
    }
}

fn is_http_client_owner(owner: &str) -> bool {
    let owner = owner.to_ascii_lowercase();
    owner.contains("client") || owner.contains("http") || owner.contains("reqwest")
}

#[derive(Clone, Copy)]
struct ProviderUrlArgument {
    index: Option<usize>,
}

fn is_non_network_test_provider(owner: &str) -> bool {
    matches!(
        owner,
        "BlockingCleanupProvider"
            | "BarrierRefreshProvider"
            | "CompleteThenHangProvider"
            | "ControlledEmitCleanupProvider"
            | "ControlledProvider"
            | "CountingProvider"
            | "ErrorRefreshProvider"
            | "HangingThenCompleteProvider"
            | "HeldRequestProvider"
            | "InvalidMetadataProvider"
            | "MockProvider"
            | "MutableRefreshProvider"
            | "PendingProvider"
            | "ProbeProvider"
            | "RecordingProvider"
            | "RefreshProvider"
            | "RouteReportingProvider"
            | "SecondTurnGatedDeltaProvider"
            | "StreamProvider"
            | "TestProvider"
    )
}

fn is_provider_constructor_name(name: &str) -> bool {
    matches!(name, "builder" | "default" | "for_route" | "new")
        || name.starts_with("from_")
        || name.starts_with("new_")
        || name.starts_with("with_")
}

fn provider_constructor_url_argument(
    tokens: &[RustToken],
    group_index: usize,
    call_name: Option<&str>,
) -> Option<ProviderUrlArgument> {
    let name = call_name?;
    if matches!(name, "mistral_provider" | "openrouter_provider") {
        let qualifier = qualified_type_before_call(tokens, group_index);
        if qualifier
            .is_none_or(|qualifier| qualifier.chars().next().is_some_and(char::is_lowercase))
        {
            return Some(ProviderUrlArgument { index: Some(0) });
        }
        return None;
    }
    let owner = qualified_type_before_call(tokens, group_index)?;
    if is_non_network_test_provider(owner) {
        return None;
    }
    let index = match (owner, name) {
        ("AnthropicProvider", "new" | "with_client") => Some(0),
        ("AnthropicProvider", "for_route") => Some(2),
        (
            "OpenAiChatProvider",
            "new" | "new_with_compat" | "with_client" | "with_auth" | "for_route",
        ) => Some(0),
        ("OpenAiChatProvider", "new_for_profile") => Some(0),
        (
            "OpenAiResponsesProvider",
            "new" | "with_client" | "new_with_config" | "with_auth" | "with_auth_extra"
            | "for_route",
        ) => Some(0),
        ("GeminiProvider", "new" | "with_client") => Some(0),
        ("BedrockProvider" | "OpenAiCodexResponsesProvider", "new") => Some(0),
        ("HttpCatalogSource", "new") => Some(0),
        ("AzureOpenAIProvider", "new" | "from_config") => Some(0),
        ("VertexProvider", "new") => Some(2),
        ("VertexProvider", "from_config") => Some(3),
        _ if owner.ends_with("Provider") && is_provider_constructor_name(name) => None,
        _ => return None,
    };
    Some(ProviderUrlArgument { index })
}

fn receiver_chain_roots_in_http_client(tokens: &[RustToken]) -> bool {
    if let [
        RustToken {
            kind:
                RustTokenKind::Group {
                    delimiter: '(',
                    tokens,
                },
            ..
        },
    ] = tokens
    {
        return receiver_chain_roots_in_http_client(tokens);
    }
    if let Some(RustToken {
        kind: RustTokenKind::Ident(owner),
        ..
    }) = tokens.last()
    {
        return is_http_client_owner(owner);
    }
    let Some(group_index) = tokens.len().checked_sub(1) else {
        return false;
    };
    let RustTokenKind::Group { .. } = &tokens[group_index].kind else {
        return false;
    };
    let call_name = call_name_before_group(tokens, group_index);
    if call_name.is_some_and(|name| {
        matches!(
            name,
            "new" | "builder" | "default" | "for_route" | "with_auth" | "with_client"
        )
    }) && qualified_type_before_call(tokens, group_index).is_some_and(is_http_client_owner)
    {
        return true;
    }
    matches!(
        tokens
            .get(group_index.wrapping_sub(2))
            .map(|token| &token.kind),
        Some(RustTokenKind::Punct('.'))
    ) && receiver_chain_roots_in_http_client(&tokens[..group_index - 2])
}

fn is_fail_closed_url_consumer_call(
    tokens: &[RustToken],
    group_index: usize,
    call_name: Option<&str>,
) -> bool {
    if provider_constructor_url_argument(tokens, group_index, call_name).is_some() {
        return true;
    }
    let Some(name) = call_name else {
        return false;
    };
    if name == "connect" {
        return true;
    }
    if !matches!(
        name,
        "delete" | "get" | "patch" | "post" | "put" | "request"
    ) {
        return false;
    }
    member_receiver_before_call(tokens, group_index)
        .or_else(|| qualified_type_before_call(tokens, group_index))
        .is_some_and(is_http_client_owner)
        || matches!(
            tokens
                .get(group_index.wrapping_sub(2))
                .map(|token| &token.kind),
            Some(RustTokenKind::Punct('.'))
        ) && receiver_chain_roots_in_http_client(&tokens[..group_index - 2])
}

fn url_consumer_argument_proves_fixture(
    tokens: &[RustToken],
    group_index: usize,
    call_name: Option<&str>,
    body: &[RustToken],
    scope: &[RustToken],
    before: usize,
    fixture_scope: &FixtureScope,
) -> bool {
    let arguments = comma_separated(body);
    let provider_argument = provider_constructor_url_argument(tokens, group_index, call_name);
    let argument = if let Some(argument) = provider_argument {
        argument
            .index
            .and_then(|index| arguments.get(index).copied())
    } else if call_name == Some("request") {
        arguments.last().copied()
    } else {
        arguments.first().copied()
    };
    argument.is_some_and(|argument| {
        expression_proves_fixture_url(argument, scope, before, 0, fixture_scope)
    })
}

fn is_url_consumer_call(tokens: &[RustToken], group_index: usize, call_name: Option<&str>) -> bool {
    if provider_constructor_url_argument(tokens, group_index, call_name).is_some() {
        return true;
    }
    let Some(name) = call_name else {
        return false;
    };
    if matches!(
        name,
        "connect" | "delete" | "get" | "patch" | "post" | "put" | "request"
    ) {
        return true;
    }
    if !matches!(
        name,
        "for_route" | "new" | "stream_prepared" | "with_auth" | "with_client"
    ) {
        return false;
    }
    qualified_type_before_call(tokens, group_index).is_some_and(|qualifier| {
        let qualifier = qualifier.to_ascii_lowercase();
        qualifier.contains("provider") || qualifier.contains("client")
    })
}

fn collect_hermetic_violations(
    tokens: &[RustToken],
    root: &[RustToken],
    observation_context: bool,
    url_consumer_context: bool,
    fixture_scope: &FixtureScope,
    violations: &mut Vec<String>,
) {
    for (index, token) in tokens.iter().enumerate() {
        match &token.kind {
            RustTokenKind::Literal(literal) => {
                let fixture_binding =
                    binding_name_before_literal(tokens, index).is_some_and(|name| {
                        let normalized = name.to_ascii_lowercase();
                        (normalized.contains("fixture")
                            || normalized.contains("redaction")
                            || normalized.contains("expected")
                            || normalized.contains("canary"))
                            && !executable_call_uses_ident(root, name)
                    });
                if !observation_context
                    && !fixture_binding
                    && let Some(host) = url_host(literal)
                    && !is_observation_fixture_host(&host)
                {
                    violations.push(format!(
                        "non-fixture HTTP endpoint `{host}` at line {}",
                        token.line
                    ));
                }
            }
            RustTokenKind::Group { tokens: body, .. } => {
                let call_name = call_name_before_group(tokens, index);
                let is_url_consumer = is_url_consumer_call(tokens, index, call_name);
                let is_fail_closed_url_consumer =
                    is_fail_closed_url_consumer_call(tokens, index, call_name);
                let nested_observation = if call_name.is_some_and(is_observation_call) {
                    true
                } else if is_url_consumer {
                    false
                } else {
                    observation_context
                };
                let constant_macro = call_name.and_then(|name| constant_macro_literal(name, body));
                if !nested_observation
                    && is_fail_closed_url_consumer
                    && !url_consumer_argument_proves_fixture(
                        tokens,
                        index,
                        call_name,
                        body,
                        tokens,
                        index,
                        fixture_scope,
                    )
                {
                    violations.push(format!(
                        "unresolved HTTP consumer argument at line {}",
                        token.line
                    ));
                }
                if !nested_observation
                    && let Some(literal) = &constant_macro
                    && let Some(host) = url_host(literal)
                    && !is_observation_fixture_host(&host)
                {
                    violations.push(format!(
                        "non-fixture HTTP endpoint `{host}` at line {}",
                        token.line
                    ));
                }
                if !nested_observation
                    && url_consumer_context
                    && constant_macro.is_none()
                    && call_name.is_some_and(|name| {
                        matches!(name, "concat" | "format")
                            && !unresolved_macro_proves_fixture_host(name, body)
                    })
                {
                    violations.push(format!(
                        "unresolved HTTP endpoint construction at line {}",
                        token.line
                    ));
                }
                if constant_macro.is_some() {
                    continue;
                }
                let nested_fixture_scope = function_fixture_scope(tokens, index, root)
                    .unwrap_or_else(|| fixture_scope.clone());
                collect_hermetic_violations(
                    body,
                    root,
                    nested_observation,
                    url_consumer_context || is_url_consumer,
                    &nested_fixture_scope,
                    violations,
                );
            }
            RustTokenKind::Ident(_) | RustTokenKind::Punct(_) => {}
        }
    }
}

pub(super) fn hermetic_source_violations(source: &str) -> Vec<String> {
    let lexed = lex_rust(source);
    let mut violations = Vec::new();
    collect_hermetic_violations(
        &lexed.tokens,
        &lexed.tokens,
        false,
        false,
        &FixtureScope::default(),
        &mut violations,
    );
    violations
}

fn render_tokens(tokens: &[RustToken]) -> String {
    let mut rendered = String::new();
    let mut previous_was_word = false;
    for token in tokens {
        match &token.kind {
            RustTokenKind::Ident(ident) => {
                if previous_was_word {
                    rendered.push(' ');
                }
                if token.raw_identifier {
                    rendered.push_str("r#");
                }
                rendered.push_str(ident);
                previous_was_word = true;
            }
            RustTokenKind::Literal(literal) => {
                if previous_was_word {
                    rendered.push(' ');
                }
                rendered.push_str(&format!("{literal:?}"));
                previous_was_word = true;
            }
            RustTokenKind::Punct(punct) => {
                rendered.push(*punct);
                previous_was_word = false;
            }
            RustTokenKind::Group { delimiter, tokens } => {
                let closing = match delimiter {
                    '(' => ')',
                    '[' => ']',
                    '{' => '}',
                    _ => unreachable!(),
                };
                rendered.push(*delimiter);
                rendered.push_str(&render_tokens(tokens));
                rendered.push(closing);
                previous_was_word = false;
            }
        }
    }
    rendered
}

fn evidence_sink_aliases(token_sets: &[&[RustToken]]) -> BTreeSet<String> {
    fn collect_use_tree(tokens: &[RustToken], edges: &mut Vec<(String, String)>) {
        for (index, token) in tokens.iter().enumerate() {
            if token_is_keyword(token, "as")
                && let Some(source) = tokens[..index].iter().rev().find_map(|candidate| {
                    if let RustTokenKind::Ident(ident) = &candidate.kind {
                        Some(ident.clone())
                    } else {
                        None
                    }
                })
                && let Some(alias) = tokens[index + 1..].iter().find_map(|candidate| {
                    if let RustTokenKind::Ident(ident) = &candidate.kind {
                        Some(ident.clone())
                    } else {
                        None
                    }
                })
            {
                edges.push((source, alias));
            }
            if let RustTokenKind::Group { tokens, .. } = &token.kind {
                collect_use_tree(tokens, edges);
            }
        }
    }

    fn collect(tokens: &[RustToken], edges: &mut Vec<(String, String)>) {
        let mut index = 0;
        while index < tokens.len() {
            if token_is_keyword(&tokens[index], "use") {
                let end = tokens[index + 1..]
                    .iter()
                    .position(|token| matches!(token.kind, RustTokenKind::Punct(';')))
                    .map_or(tokens.len(), |offset| index + 1 + offset);
                collect_use_tree(&tokens[index + 1..end], edges);
                index = end;
            } else if let RustTokenKind::Group { tokens, .. } = &tokens[index].kind {
                collect(tokens, edges);
            }
            index += 1;
        }
    }

    let mut aliases = BTreeSet::from(["EvidenceSink".to_owned()]);
    let mut edges = Vec::new();
    for tokens in token_sets {
        collect(tokens, &mut edges);
    }
    loop {
        let mut changed = false;
        for (source, alias) in &edges {
            if aliases.contains(source) {
                changed |= aliases.insert(alias.clone());
            }
        }
        if !changed {
            break;
        }
    }
    aliases
}

fn collect_evidence_sink_impl_targets(
    tokens: &[RustToken],
    trait_names: &BTreeSet<String>,
    targets: &mut Vec<String>,
) {
    let mut index = 0;
    while index < tokens.len() {
        if token_is_keyword(&tokens[index], "impl") {
            let header_end = tokens[index + 1..]
                .iter()
                .position(|token| {
                    matches!(
                        token.kind,
                        RustTokenKind::Group { delimiter: '{', .. } | RustTokenKind::Punct(';')
                    )
                })
                .map_or(tokens.len(), |offset| index + 1 + offset);
            let mut angle_depth = 0_usize;
            let for_index =
                (index + 1..header_end).find(|candidate| match &tokens[*candidate].kind {
                    RustTokenKind::Punct('<') => {
                        angle_depth += 1;
                        false
                    }
                    RustTokenKind::Punct('>') => {
                        angle_depth = angle_depth.saturating_sub(1);
                        false
                    }
                    RustTokenKind::Ident(_) => {
                        token_is_keyword(&tokens[*candidate], "for") && angle_depth == 0
                    }
                    _ => false,
                });
            if let Some(for_index) = for_index {
                let mut angle_depth = 0_usize;
                let implemented_trait =
                    tokens[index + 1..for_index]
                        .iter()
                        .rev()
                        .find_map(|token| match &token.kind {
                            RustTokenKind::Punct('>') => {
                                angle_depth += 1;
                                None
                            }
                            RustTokenKind::Punct('<') => {
                                angle_depth = angle_depth.saturating_sub(1);
                                None
                            }
                            RustTokenKind::Ident(ident) if angle_depth == 0 => Some(ident.as_str()),
                            _ => None,
                        });
                if !implemented_trait.is_some_and(|name| trait_names.contains(name)) {
                    index += 1;
                    continue;
                }
                let target_end = tokens[for_index + 1..header_end]
                    .iter()
                    .position(|token| token_is_keyword(token, "where"))
                    .map_or(header_end, |offset| for_index + 1 + offset);
                let target = render_tokens(&tokens[for_index + 1..target_end]);
                if !target.is_empty() {
                    targets.push(target);
                }
            }
        }
        if let RustTokenKind::Group { tokens, .. } = &tokens[index].kind {
            collect_evidence_sink_impl_targets(tokens, trait_names, targets);
        }
        index += 1;
    }
}

pub(super) fn evidence_sink_impl_targets(source: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let tokens = lex_rust(source).tokens;
    let trait_names = evidence_sink_aliases(&[&tokens]);
    collect_evidence_sink_impl_targets(&tokens, &trait_names, &mut targets);
    targets
}

pub(super) fn evidence_sink_impl_targets_across_sources(sources: &[&str]) -> Vec<String> {
    let token_sets = sources
        .iter()
        .map(|source| lex_rust(source).tokens)
        .collect::<Vec<_>>();
    let token_refs = token_sets.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let trait_names = evidence_sink_aliases(&token_refs);
    let mut targets = Vec::new();
    for tokens in &token_sets {
        collect_evidence_sink_impl_targets(tokens, &trait_names, &mut targets);
    }
    targets
}

// ===========================================================================
// P17-MIG-006 — removed interfaces stay removed: no symbol, alias, or shim in
// production source.
