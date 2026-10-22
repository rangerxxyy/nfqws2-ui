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
        let encoded = if raw.starts_with('"') && raw.ends_with('"') {
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
        };
        Ok(self.replace(assignment, &encoded))
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
