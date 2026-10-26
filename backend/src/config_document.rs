//! Lossless editing of assignment values; never evaluates shell expressions.

use std::ops::Range;

#[derive(Debug)]
struct Assignment<'a> {
    name: &'a str,
    value: Range<usize>,
}

/// A conservative shell-assignment document. Unsupported executable fragments
/// must stay in the raw editor; they are never sourced or partially rewritten.
#[derive(Debug)]
pub struct ConfigDocument<'a> {
    source: &'a str,
    assignments: Vec<Assignment<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub message: String,
    pub line: Option<usize>,
}

/// Performs conservative checks derived from the nfqws2-keenetic init script.
/// This does not expand shell variables or validate options against a binary.
pub fn validate_nfqws2_config(source: &str) -> Result<Vec<ConfigDiagnostic>, String> {
    let document = ConfigDocument::parse(source)?;
    let delimiter_forbidden = [
        "NFQWS_BASE_ARGS",
        "NFQWS_ARGS",
        "NFQWS_ARGS_QUIC",
        "NFQWS_ARGS_UDP",
        "NFQWS_EXTRA_ARGS",
        "NFQWS_ARGS_IPSET",
    ];
    let argument_variables = [
        "NFQWS_BASE_ARGS",
        "NFQWS_ARGS",
        "NFQWS_ARGS_QUIC",
        "NFQWS_ARGS_UDP",
        "NFQWS_EXTRA_ARGS",
        "NFQWS_ARGS_IPSET",
        "NFQWS_ARGS_CUSTOM",
    ];
    let mut diagnostics = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for assignment in &document.assignments {
        if !argument_variables.contains(&assignment.name) {
            continue;
        }
        let raw_value = &source[assignment.value.clone()];
        let line = source[..assignment.value.start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        if !seen.insert(assignment.name) {
            diagnostics.push(ConfigDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "duplicate_assignment",
                message: format!(
                    "Переменная {} задана несколько раз; фактическое значение shell здесь не вычисляется",
                    assignment.name
                ),
                line: Some(line),
            });
        }
        if delimiter_forbidden.contains(&assignment.name) && raw_value.contains("--new") {
            let delimiter_line = raw_value
                .find("--new")
                .map(|offset| {
                    source[..assignment.value.start + offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1
                })
                .unwrap_or(line);
            diagnostics.push(ConfigDiagnostic {
                severity: DiagnosticSeverity::Error,
                code: "profile_delimiter_location",
                message: format!(
                    "--new запрещён в {}; штатный init-скрипт допускает разделители профилей только в NFQWS_ARGS_CUSTOM",
                    assignment.name
                ),
                line: Some(delimiter_line),
            });
        }
        if raw_value.contains('$') {
            diagnostics.push(ConfigDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "dynamic_shell_value",
                message: format!(
                    "Значение {} содержит shell-переменную; она не раскрывается и не проверяется этим endpoint",
                    assignment.name
                ),
                line: Some(line),
            });
        }
    }
    Ok(diagnostics)
}

impl<'a> ConfigDocument<'a> {
    pub fn parse(source: &'a str) -> Result<Self, String> {
        if source.contains('\0') {
            return Err("Конфигурация содержит NUL-байт".into());
        }
        let bytes = source.as_bytes();
        let mut cursor = 0;
        let mut assignments = Vec::new();
        let failure = |offset: usize| {
            let line = bytes[..offset]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count()
                + 1;
            format!(
                "Строка {line}: неподдерживаемый или незавершённый shell-синтаксис; используйте raw-редактор"
            )
        };
        while cursor < bytes.len() {
            while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
                cursor += 1;
            }
            if cursor == bytes.len() {
                break;
            }
            if bytes[cursor] == b'#' {
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
                continue;
            }
            let statement = cursor;
            if source[cursor..].starts_with("export")
                && bytes
                    .get(cursor + 6)
                    .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                cursor += 6;
                while bytes
                    .get(cursor)
                    .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
                {
                    cursor += 1;
                }
            }
            let name_start = cursor;
            if !bytes
                .get(cursor)
                .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
            {
                return Err(failure(statement));
            }
            cursor += 1;
            while bytes
                .get(cursor)
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            {
                cursor += 1;
            }
            let name = &source[name_start..cursor];
            if bytes.get(cursor) != Some(&b'=') {
                return Err(failure(statement));
            }
            cursor += 1;
            let value_start = cursor;
            let mut quote = None;
            while let Some(&byte) = bytes.get(cursor) {
                if byte == b'\\' && quote != Some(b'\'') {
                    if cursor + 1 == bytes.len() {
                        return Err(failure(statement));
                    }
                    cursor += 2;
                    continue;
                }
                if quote != Some(b'\'')
                    && (byte == b'`' || (byte == b'$' && bytes.get(cursor + 1) == Some(&b'(')))
                {
                    return Err(failure(statement));
                }
                if let Some(open) = quote {
                    if byte == open {
                        quote = None;
                    }
                } else if byte == b'\'' || byte == b'"' {
                    quote = Some(byte);
                } else if byte.is_ascii_whitespace() {
                    break;
                } else if b";|&<>()".contains(&byte) {
                    return Err(failure(statement));
                }
                cursor += 1;
            }
            if quote.is_some() {
                return Err(failure(statement));
            }
            assignments.push(Assignment {
                name,
                value: value_start..cursor,
            });
            while bytes
                .get(cursor)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r'))
            {
                cursor += 1;
            }
            if bytes.get(cursor) == Some(&b'#') {
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
            }
            if cursor < bytes.len() {
                if bytes[cursor] != b'\n' {
                    return Err(failure(statement));
                }
                cursor += 1;
            }
        }
        Ok(Self {
            source,
            assignments,
        })
    }

    fn unique(&self, name: &str) -> Result<&Assignment<'a>, String> {
        let matches = self
            .assignments
            .iter()
            .filter(|item| item.name == name)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [assignment] => Ok(assignment),
            [] => Err(format!(
                "Переменная {name} отсутствует; добавьте её в raw-редакторе"
            )),
            _ => Err(format!(
                "Переменная {name} задана несколько раз; сначала устраните неоднозначность в raw-редакторе"
            )),
        }
    }

    pub fn contains_assignment(&self, name: &str) -> bool {
        self.assignments
            .iter()
            .any(|assignment| assignment.name == name)
    }

    fn replace(&self, assignment: &Assignment<'_>, value: &str) -> String {
        let mut result = self.source.to_owned();
        result.replace_range(assignment.value.clone(), value);
        result
    }

    pub fn set_literal(&self, name: &str, value: &str) -> Result<String, String> {
        if value.contains('\0') {
            return Err("Значение содержит NUL-байт".into());
        }
        let assignment = self.unique(name)?;
        let raw = &self.source[assignment.value.clone()];
        let encoded = Self::encode_literal(raw, value);
        Ok(self.replace(assignment, &encoded))
    }

    /// Replace all assignments only when the caller has already resolved the
    /// supported static/self-append forms. Repeating the final literal value
    /// preserves shell's last-assignment-wins behavior and surrounding text.
    pub fn set_literal_all(&self, name: &str, value: &str) -> Result<String, String> {
        if value.contains('\0') {
            return Err("Значение содержит NUL-байт".into());
        }
        let mut matches = self
            .assignments
            .iter()
            .filter(|assignment| assignment.name == name)
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return self.set_literal_or_append(name, value);
        }
        let mut result = self.source.to_owned();
        for assignment in matches.drain(..).rev() {
            let raw = &self.source[assignment.value.clone()];
            result.replace_range(assignment.value.clone(), &Self::encode_literal(raw, value));
        }
        Ok(result)
    }

    fn encode_literal(raw: &str, value: &str) -> String {
        if raw.starts_with('"') && raw.ends_with('"') {
            let mut escaped = String::from("\"");
            for character in value.chars() {
                if matches!(character, '\\' | '"' | '$' | '`') {
                    escaped.push('\\');
                }
                escaped.push(character);
            }
            escaped.push('"');
            escaped
        } else if !raw.starts_with('\'')
            && !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_./:-,".contains(&byte))
        {
            value.to_owned()
        } else {
            format!("'{}'", value.replace('\'', "'\\''"))
        }
    }

    /// Set a unique literal assignment or append it when an optional nfqws2
    /// argument group is absent from an otherwise valid configuration.
    pub fn set_literal_or_append(&self, name: &str, value: &str) -> Result<String, String> {
        if value.contains('\0') {
            return Err("Значение содержит NUL-байт".into());
        }
        let matches = self
            .assignments
            .iter()
            .filter(|item| item.name == name)
            .count();
        match matches {
            1 => self.set_literal(name, value),
            count if count > 1 => Err(format!(
                "Переменная {name} задана несколько раз; сначала устраните неоднозначность в raw-редакторе"
            )),
            _ => {
                let mut encoded = String::from("\"");
                for character in value.chars() {
                    if matches!(character, '\\' | '"' | '$' | '`') {
                        encoded.push('\\');
                    }
                    encoded.push(character);
                }
                encoded.push('"');
                let separator = if self.source.is_empty() || self.source.ends_with('\n') {
                    ""
                } else {
                    "\n"
                };
                Ok(format!("{}{separator}{name}={encoded}\n", self.source))
            }
        }
    }

    /// Read a shell assignment only when its full value is a single literal.
    /// Expansions, concatenated quote fragments and escaped syntax are refused;
    /// this parser never evaluates shell code.
    pub fn literal_value(&self, name: &str) -> Result<String, String> {
        let assignment = self.unique(name)?;
        let raw = &self.source[assignment.value.clone()];
        Self::literal_assignment_value(raw, name)
    }

    fn literal_assignment_value(raw: &str, name: &str) -> Result<String, String> {
        if raw.starts_with('"') || raw.starts_with('\'') {
            let quote = raw.as_bytes()[0];
            if raw.len() < 2 || raw.as_bytes().last() != Some(&quote) {
                return Err(format!("Значение {name} не является одной строкой"));
            }
            let value = &raw[1..raw.len() - 1];
            if value.bytes().any(|byte| {
                byte == quote || byte == b'\\' || (quote == b'"' && matches!(byte, b'$' | b'`'))
            }) {
                return Err(format!(
                    "Значение {name} содержит shell-синтаксис и не может безопасно редактироваться"
                ));
            }
            return Ok(value.to_owned());
        }
        if raw.is_empty()
            || !raw
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_./:=,+-".contains(&byte))
        {
            return Err(format!(
                "Значение {name} не является безопасным shell-литералом"
            ));
        }
        Ok(raw.to_owned())
    }

    /// Read a literal NFQWS_EXTRA_ARGS value or expand one exact `$MODE_*`
    /// assignment from the same document. Repeated assignments are accepted
    /// only as static replacements or a single self-append of the previous
    /// value, as used by Zemidala's custom profile editor.
    pub fn literal_or_mode_value(&self, name: &str) -> Result<String, String> {
        let assignments = self
            .assignments
            .iter()
            .filter(|assignment| assignment.name == name)
            .collect::<Vec<_>>();
        if assignments.is_empty() {
            return Err(format!("Переменная {name} отсутствует"));
        }

        let mut value = String::new();
        for assignment in assignments {
            let raw = &self.source[assignment.value.clone()];
            if let Some(suffix) = Self::self_append_suffix(raw, name)? {
                if !suffix.is_empty() {
                    if !value.is_empty() {
                        value.push(' ');
                    }
                    value.push_str(suffix.trim_start());
                }
                continue;
            }

            if name == "NFQWS_EXTRA_ARGS"
                && let Some(variable) = Self::mode_reference(raw)
            {
                if !["MODE_LIST", "MODE_AUTO", "MODE_ALL"].contains(&variable) {
                    return Err(format!("Недопустимая статическая ссылка в {name}"));
                }
                value = self
                    .expand_mode_value(variable, &mut Vec::new(), &mut 0)
                    .map_err(|message| {
                        format!("{name}: не удалось безопасно раскрыть {variable}: {message}")
                    })?;
                continue;
            }

            value = Self::literal_assignment_value(raw, name)?;
        }
        Ok(value)
    }

    fn mode_reference(raw: &str) -> Option<&str> {
        let value = raw.strip_prefix('"')?.strip_suffix('"')?;
        if value.starts_with("${") {
            value.strip_prefix("${")?.strip_suffix('}')
        } else {
            value.strip_prefix('$')
        }
    }

    fn self_append_suffix<'b>(raw: &'b str, name: &str) -> Result<Option<&'b str>, String> {
        let Some(value) = raw
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
        else {
            return Ok(None);
        };
        let short = format!("${name}");
        let braced = format!("${{{name}}}");
        let suffix = value
            .strip_prefix(&braced)
            .or_else(|| value.strip_prefix(&short));
        let Some(suffix) = suffix else {
            return Ok(None);
        };
        if !suffix.is_empty() && !suffix.chars().next().is_some_and(char::is_whitespace) {
            return Ok(None);
        }
        if suffix
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b'$' | b'\\' | b'`' | b'"'))
        {
            return Err(format!(
                "Значение {name} содержит неподдерживаемый shell-синтаксис в self-append"
            ));
        }
        Ok(Some(suffix.trim_start()))
    }

    fn expand_mode_value(
        &self,
        name: &str,
        active: &mut Vec<String>,
        expansion_count: &mut usize,
    ) -> Result<String, String> {
        const MAX_MODE_EXPANSIONS: usize = 32;
        if active.iter().any(|current| current == name) {
            return Err(format!("обнаружен цикл в переменной {name}"));
        }
        active.push(name.to_owned());
        let assignment = self.unique(name)?;
        let raw = &self.source[assignment.value.clone()];
        let value = if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
            let value = &raw[1..raw.len() - 1];
            if value.contains(['\\', '"', '`']) {
                return Err(format!(
                    "переменная {name} содержит неподдерживаемое shell-экранирование"
                ));
            }
            value
        } else {
            let value = self.literal_value(name)?;
            active.pop();
            return Ok(value);
        };

        let mut expanded = String::with_capacity(value.len());
        let mut cursor = 0;
        while cursor < value.len() {
            let rest = &value[cursor..];
            if let Some(rest) = rest.strip_prefix('$') {
                let (mode_name, consumed) = if let Some(rest) = rest.strip_prefix('{') {
                    let Some(end) = rest.find('}') else {
                        return Err(format!("незакрытая ссылка в {name}"));
                    };
                    (&rest[..end], end + 3)
                } else {
                    let length = rest
                        .bytes()
                        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                        .count();
                    (&rest[..length], length + 1)
                };
                if !["MODE_LIST", "MODE_AUTO", "MODE_ALL"].contains(&mode_name) {
                    return Err(format!(
                        "недопустимая shell-переменная ${mode_name} в {name}"
                    ));
                }
                *expansion_count = expansion_count.saturating_add(1);
                if *expansion_count > MAX_MODE_EXPANSIONS {
                    return Err("превышен лимит вложенных ссылок MODE_*".into());
                }
                expanded.push_str(&self.expand_mode_value(mode_name, active, expansion_count)?);
                cursor += consumed;
            } else {
                let character = rest.chars().next().expect("cursor is before end");
                expanded.push(character);
                cursor += character.len_utf8();
            }
        }
        active.pop();
        Ok(expanded)
    }

    /// Append a sequence of plain nfqws2 argument tokens to a literal variable.
    pub fn append_literal_words(&self, name: &str, words: &str) -> Result<String, String> {
        if words.is_empty()
            || words.bytes().any(|byte| {
                byte.is_ascii_control()
                    || matches!(
                        byte,
                        b'\'' | b'"' | b'`' | b'$' | b'\\' | b';' | b'&' | b'|'
                    )
            })
        {
            return Err("Добавляемые параметры содержат недопустимый shell-синтаксис".into());
        }
        let existing = self.literal_value(name)?;
        let appended = words.split_whitespace().collect::<Vec<_>>().join("\n");
        let value = if existing.is_empty() {
            appended
        } else {
            let separator = if appended.starts_with("--new") {
                "\n\n"
            } else {
                "\n"
            };
            format!("{}{}{}", existing.trim_end(), separator, appended)
        };
        self.set_literal(name, &value)
    }

    pub fn set_mode(&self, mode: &str) -> Result<String, String> {
        if !["$MODE_LIST", "$MODE_AUTO", "$MODE_ALL"].contains(&mode) {
            return Err("Неизвестный режим nfqws2".into());
        }
        let definition = self.unique(&mode[1..])?;
        let raw = &self.source[definition.value.clone()];
        if raw.trim_matches(['\'', '"']).trim().is_empty() {
            return Err(format!("Режим {mode} не настроен"));
        }
        Ok(self.replace(self.unique("NFQWS_EXTRA_ARGS")?, &format!("\"{mode}\"")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_multiline_value_and_preserves_all_other_bytes() {
        let text = "# пользовательский конфиг\r\nexport ISP_INTERFACE=\"wan\r\n eth2\" # пояснение\r\nUNKNOWN='keep # $literal'\r\nNFQWS_ARGS=\"--filter-tcp=443\r\n --lua-desync=fake\"\r\n";
        let edited = ConfigDocument::parse(text)
            .unwrap()
            .set_literal("ISP_INTERFACE", "eth3")
            .unwrap();
        assert_eq!(edited, text.replacen("\"wan\r\n eth2\"", "\"eth3\"", 1));
    }

    #[test]
    fn flattens_supported_self_appended_custom_profile_assignments_without_touching_comments() {
        let source = "NFQWS_ARGS_CUSTOM=\"--filter-tcp=443\" # base\nNFQWS_ARGS_CUSTOM=\"$NFQWS_ARGS_CUSTOM --new --filter-tcp=80\" # append\n";
        let document = ConfigDocument::parse(source).unwrap();
        let resolved = document.literal_or_mode_value("NFQWS_ARGS_CUSTOM").unwrap();
        assert_eq!(resolved, "--filter-tcp=443 --new --filter-tcp=80");
        let updated = document
            .set_literal_all(
                "NFQWS_ARGS_CUSTOM",
                "--filter-tcp=443 --new --filter-tcp=443",
            )
            .unwrap();
        assert!(updated.contains("# base"));
        assert!(updated.contains("# append"));
        let parsed = ConfigDocument::parse(&updated).unwrap();
        assert_eq!(
            parsed.literal_or_mode_value("NFQWS_ARGS_CUSTOM").unwrap(),
            "--filter-tcp=443 --new --filter-tcp=443"
        );
    }

    #[test]
    fn changes_exact_variable_without_matching_comments_or_quoted_content() {
        let text =
            "# ISP_INTERFACE=wrong\nUNKNOWN='ISP_INTERFACE=fake'\n ISP_INTERFACE=wan # keep\n";
        let edited = ConfigDocument::parse(text)
            .unwrap()
            .set_literal("ISP_INTERFACE", "eth0")
            .unwrap();
        assert_eq!(
            edited,
            text.replace("ISP_INTERFACE=wan", "ISP_INTERFACE=eth0")
        );
    }

    #[test]
    fn argument_groups_can_be_added_when_missing_and_existing_lines_are_preserved() {
        let source = "KEEP=1\nNFQWS_ARGS=\"--filter-tcp=443\"\n";
        let edited = ConfigDocument::parse(source)
            .unwrap()
            .set_literal_or_append("NFQWS_ARGS_UDP", "--filter-udp=443 --payload=quic_initial")
            .unwrap();
        assert!(edited.starts_with(source));
        assert!(edited.ends_with("NFQWS_ARGS_UDP=\"--filter-udp=443 --payload=quic_initial\"\n"));

        let changed = ConfigDocument::parse(&edited)
            .unwrap()
            .set_literal_or_append("NFQWS_ARGS_UDP", "--filter-udp=443 --new")
            .unwrap();
        assert!(changed.ends_with("NFQWS_ARGS_UDP=\"--filter-udp=443 --new\"\n"));
    }

    #[test]
    fn rejects_duplicate_missing_malformed_and_executable_assignments() {
        assert!(
            ConfigDocument::parse("A=one\nA=two\n")
                .unwrap()
                .set_literal("A", "three")
                .is_err()
        );
        assert!(
            ConfigDocument::parse("A=one\n")
                .unwrap()
                .set_literal("MISSING", "x")
                .is_err()
        );
        for text in [
            "A=\"unterminated",
            "A=one; touch /tmp/never\n",
            "A=$(touch /tmp/never)\n",
            "A=`id`\n",
            "if true; then\nA=one\nfi\n",
            "A=one B=two\n",
            "A=\0\n",
        ] {
            assert!(ConfigDocument::parse(text).is_err(), "accepted {text:?}");
        }
    }

    #[test]
    fn quotes_literal_values_instead_of_turning_them_into_shell_commands() {
        let text = "A=\"old\" # untouched\n";
        let edited = ConfigDocument::parse(text)
            .unwrap()
            .set_literal("A", "$(id) `$HOME` \\\"quoted")
            .unwrap();
        assert_eq!(
            edited,
            "A=\"\\$(id) \\`\\$HOME\\` \\\\\\\"quoted\" # untouched\n"
        );
        assert_eq!(
            ConfigDocument::parse("A='old'\n")
                .unwrap()
                .set_literal("A", "it's fine")
                .unwrap(),
            "A='it'\\''s fine'\n"
        );
    }

    #[test]
    fn mode_selection_preserves_comment_and_uses_intentional_variable_expansion() {
        let text = "MODE_LIST='--hostlist=lists/user.list'\nMODE_ALL=''\nNFQWS_EXTRA_ARGS='$MODE_ALL' # режим\n";
        let document = ConfigDocument::parse(text).unwrap();
        assert_eq!(
            document.set_mode("$MODE_LIST").unwrap(),
            text.replace(
                "NFQWS_EXTRA_ARGS='$MODE_ALL'",
                "NFQWS_EXTRA_ARGS=\"$MODE_LIST\""
            )
        );
        assert!(document.set_mode("$MODE_ALL").is_err());
        assert!(document.set_mode("$MODE_AUTO").is_err());
        assert!(document.set_mode("$(id)").is_err());
        assert!(
            ConfigDocument::parse("MODE_ALL='  '\nNFQWS_EXTRA_ARGS=''\n")
                .unwrap()
                .set_mode("$MODE_ALL")
                .is_err()
        );
    }

    #[test]
    fn foreign_config_expands_nested_zemidala_mode_assignments_safely() {
        let source = concat!(
            "MODE_LIST=\"--hostlist=lists/user.list\"\n",
            "MODE_ALL=\"--hostlist-exclude=lists/exclude.list\"\n",
            "MODE_AUTO=\"$MODE_LIST --hostlist-auto=lists/auto.list $MODE_ALL\"\n",
            "NFQWS_EXTRA_ARGS=\"$MODE_AUTO\"\n",
        );
        let document = ConfigDocument::parse(source).unwrap();
        assert_eq!(
            document.literal_or_mode_value("NFQWS_EXTRA_ARGS").unwrap(),
            "--hostlist=lists/user.list --hostlist-auto=lists/auto.list --hostlist-exclude=lists/exclude.list"
        );

        let braced = ConfigDocument::parse(
            "MODE_LIST='--hostlist=lists/user.list'\nMODE_AUTO=\"${MODE_LIST}\"\nNFQWS_EXTRA_ARGS=\"$MODE_AUTO\"\n",
        )
        .unwrap();
        assert_eq!(
            braced.literal_or_mode_value("NFQWS_EXTRA_ARGS").unwrap(),
            "--hostlist=lists/user.list"
        );

        let cycle = ConfigDocument::parse(
            "MODE_LIST=\"$MODE_AUTO\"\nMODE_AUTO=\"$MODE_LIST\"\nNFQWS_EXTRA_ARGS=\"$MODE_AUTO\"\n",
        )
        .unwrap();
        assert!(cycle.literal_or_mode_value("NFQWS_EXTRA_ARGS").is_err());

        let unsupported = ConfigDocument::parse(
            "MODE_AUTO=\"$HOME --hostlist=lists/user.list\"\nNFQWS_EXTRA_ARGS=\"$MODE_AUTO\"\n",
        )
        .unwrap();
        assert!(
            unsupported
                .literal_or_mode_value("NFQWS_EXTRA_ARGS")
                .is_err()
        );
    }

    #[test]
    fn foreign_config_reads_literal_and_self_appended_argument_assignments() {
        let source = concat!(
            "NFQWS_ARGS_CUSTOM=\"--filter-tcp=443 --lua-desync=fake\"\n",
            "NFQWS_ARGS_CUSTOM=\"$NFQWS_ARGS_CUSTOM --new --filter-tcp=80 --lua-desync=split\"\n",
        );
        let document = ConfigDocument::parse(source).unwrap();
        assert_eq!(
            document.literal_or_mode_value("NFQWS_ARGS_CUSTOM").unwrap(),
            "--filter-tcp=443 --lua-desync=fake --new --filter-tcp=80 --lua-desync=split"
        );

        let braced = ConfigDocument::parse(
            "NFQWS_ARGS=\"--filter-tcp=443\"\nNFQWS_ARGS=\"${NFQWS_ARGS} --lua-desync=fake\"\n",
        )
        .unwrap();
        assert_eq!(
            braced.literal_or_mode_value("NFQWS_ARGS").unwrap(),
            "--filter-tcp=443 --lua-desync=fake"
        );

        let replaced = ConfigDocument::parse(
            "NFQWS_ARGS=\"--filter-tcp=443\"\nNFQWS_ARGS=\"--filter-tcp=80\"\n",
        )
        .unwrap();
        assert_eq!(
            replaced.literal_or_mode_value("NFQWS_ARGS").unwrap(),
            "--filter-tcp=80"
        );

        let unsupported = ConfigDocument::parse(
            "NFQWS_ARGS=\"--filter-tcp=443\"\nNFQWS_ARGS=\"$HOME --filter-tcp=80\"\n",
        )
        .unwrap();
        assert!(unsupported.literal_or_mode_value("NFQWS_ARGS").is_err());
    }

    #[test]
    fn parses_the_upstream_package_fixture_without_changing_unrelated_strategies() {
        let text = include_str!("../tests/fixtures/nfqws2-keenetic.conf");
        let edited = ConfigDocument::parse(text)
            .unwrap()
            .set_literal("ISP_INTERFACE", "eth4")
            .unwrap();
        assert_eq!(
            edited,
            text.replacen("ISP_INTERFACE=\"eth3\"", "ISP_INTERFACE=\"eth4\"", 1)
        );
    }

    #[test]
    fn reads_only_unambiguous_literal_assignment_values() {
        for (source, expected) in [
            (
                "NFQWS_ARGS_CUSTOM=\"--new --filter-tcp=443\"\n",
                "--new --filter-tcp=443",
            ),
            (
                "NFQWS_ARGS_CUSTOM='--new --filter-tcp=443'\n",
                "--new --filter-tcp=443",
            ),
            ("NFQWS_ARGS_CUSTOM=--new\n", "--new"),
        ] {
            let document = ConfigDocument::parse(source).unwrap();
            assert_eq!(
                document.literal_value("NFQWS_ARGS_CUSTOM").unwrap(),
                expected
            );
        }
    }

    #[test]
    fn rejects_dynamic_quoted_and_duplicate_values_when_reading_literal() {
        for source in [
            "NFQWS_ARGS_CUSTOM=\"$MODE\"\n",
            "NFQWS_ARGS_CUSTOM=\"--new\\\"--other\"\n",
            "NFQWS_ARGS_CUSTOM=\"--new\"'--other'\n",
            "NFQWS_ARGS_CUSTOM=\"--new\"\nNFQWS_ARGS_CUSTOM=\"--other\"\n",
        ] {
            let document = ConfigDocument::parse(source).unwrap();
            assert!(
                document.literal_value("NFQWS_ARGS_CUSTOM").is_err(),
                "accepted {source:?}"
            );
        }
    }

    #[test]
    fn appends_words_to_custom_args_and_preserves_comments_and_other_assignments() {
        let source = "OTHER='keep'\nNFQWS_ARGS_CUSTOM=\"--new --filter-tcp=80\" # profiles\n";
        let updated = ConfigDocument::parse(source)
            .unwrap()
            .append_literal_words("NFQWS_ARGS_CUSTOM", "--new --filter-tcp=443")
            .unwrap();
        assert_eq!(
            updated,
            "OTHER='keep'\nNFQWS_ARGS_CUSTOM=\"--new --filter-tcp=80\n\n--new\n--filter-tcp=443\" # profiles\n"
        );
    }

    #[test]
    fn nfqws2_static_validation_rejects_new_in_package_managed_argument_groups() {
        let source = "NFQWS_ARGS=\"--filter-tcp=443 --new\"\nNFQWS_ARGS_CUSTOM=\"--new\"\n";
        let diagnostics = validate_nfqws2_config(source).unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "profile_delimiter_location");
        assert_eq!(diagnostics[0].line, Some(1));
        assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn nfqws2_static_validation_applies_package_delimiter_rule_to_every_managed_group() {
        for variable in [
            "NFQWS_BASE_ARGS",
            "NFQWS_ARGS",
            "NFQWS_ARGS_QUIC",
            "NFQWS_ARGS_UDP",
            "NFQWS_EXTRA_ARGS",
            "NFQWS_ARGS_IPSET",
        ] {
            let source = format!("{variable}=\"--new\"\n");
            let diagnostics = validate_nfqws2_config(&source).unwrap();
            assert!(
                diagnostics.iter().any(|item| {
                    item.code == "profile_delimiter_location"
                        && item.severity == DiagnosticSeverity::Error
                }),
                "package-managed group {variable} must reject --new"
            );
        }
    }

    #[test]
    fn nfqws2_static_validation_reports_the_line_where_forbidden_delimiter_occurs() {
        let diagnostics =
            validate_nfqws2_config("NFQWS_ARGS=\"--filter-tcp=443\n --new\"\n").unwrap();
        assert_eq!(diagnostics[0].line, Some(2));
    }

    #[test]
    fn nfqws2_static_validation_warns_on_unresolved_shell_values_without_expanding_them() {
        let diagnostics = validate_nfqws2_config(
            "MODE_LIST=\"--hostlist=/opt/etc/nfqws2/lists/user.list\"\nNFQWS_EXTRA_ARGS=\"$MODE_LIST\"\n",
        )
        .unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "dynamic_shell_value");
        assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Warning);
    }

    #[test]
    fn nfqws2_static_validation_accepts_upstream_fixture_and_custom_delimiters() {
        let diagnostics =
            validate_nfqws2_config(include_str!("../tests/fixtures/nfqws2-keenetic.conf")).unwrap();
        assert!(
            !diagnostics
                .iter()
                .any(|item| item.severity == DiagnosticSeverity::Error)
        );
        assert!(
            diagnostics
                .iter()
                .any(|item| item.code == "dynamic_shell_value")
        );
        assert!(
            validate_nfqws2_config(
                "NFQWS_ARGS_CUSTOM=\"--filter-tcp=80 --new --filter-tcp=443\"\n"
            )
            .is_ok()
        );
    }

    #[test]
    fn nfqws2_static_validation_rejects_unsafe_shell_syntax() {
        assert!(validate_nfqws2_config("NFQWS_ARGS=$(touch /tmp/should-not-run)\n").is_err());
    }
}
