/// Reflows only simple, double-quoted `NFQWS_ARGS*` values. Tokens and their
/// order are preserved; shell expansion, escapes, nested quotes, and malformed
/// values are rejected instead of guessed at.
pub fn format_nfqws_config(source: &str) -> Result<String, String> {
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    while cursor < source.len() {
        let line_end = source[cursor..]
            .find('\n')
            .map(|offset| cursor + offset + 1)
            .unwrap_or(source.len());
        let line = &source[cursor..line_end];
        let line_without_ending = line.trim_end_matches(['\r', '\n']);
        let trimmed = line_without_ending.trim_start_matches([' ', '\t']);
        let assignment = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some(equal) = assignment.find('=') else {
            output.push_str(line);
            cursor = line_end;
            continue;
        };
        let raw_name = &assignment[..equal];
        let name = raw_name.trim();
        if !(name == "NFQWS_ARGS" || name.starts_with("NFQWS_ARGS_")) {
            output.push_str(line);
            cursor = line_end;
            continue;
        }
        if raw_name != name {
            return Err(format!(
                "Переменная {name}: пробелы в имени переменной не поддерживаются"
            ));
        }

        let after_equal = &assignment[equal + 1..];
        let value_prefix_len =
            after_equal.len() - after_equal.trim_start_matches([' ', '\t']).len();
        let value = &after_equal[value_prefix_len..];
        if !value.starts_with('"') {
            output.push_str(line);
            cursor = line_end;
            continue;
        }

        let absolute_equal = cursor + line_without_ending.find('=').unwrap();
        let opening_quote = absolute_equal + 1 + value_prefix_len;
        let content_start = opening_quote + 1;
        let Some(close_relative) = source[content_start..].find('"') else {
            return Err(format!("Переменная {name}: незакрытая двойная кавычка"));
        };
        let close_quote = content_start + close_relative;
        let trailing = source[close_quote + 1..]
            .split_once('\n')
            .map_or(&source[close_quote + 1..], |(tail, _)| tail)
            .trim_end_matches('\r')
            .trim_start_matches([' ', '\t']);
        if !trailing.is_empty() && !trailing.starts_with('#') {
            return Err(format!(
                "Переменная {name}: после закрывающей кавычки найдено дополнительное shell-выражение"
            ));
        }
        let content = &source[content_start..close_quote];
        if content.lines().skip(1).any(looks_like_shell_assignment) {
            return Err(format!(
                "Переменная {name}: следующая переменная встретилась до закрывающей кавычки"
            ));
        }
        if content
            .chars()
            .any(|character| matches!(character, '\\' | '$' | '`' | '\'' | '"'))
        {
            return Err(format!(
                "Переменная {name}: форматирование пропущено из-за экранирования, подстановки или вложенных кавычек"
            ));
        }
        let tokens = content.split_whitespace().collect::<Vec<_>>();
        if tokens.iter().any(|token| token.starts_with('#')) {
            return Err(format!(
                "Переменная {name}: комментарий внутри значения нельзя безопасно переформатировать"
            ));
        }
        let indent = "    ";
        let mut formatted = String::new();
        for token in tokens {
            if !formatted.is_empty() {
                formatted.push_str(newline);
                if token == "--new" {
                    formatted.push_str(newline);
                }
                formatted.push_str(indent);
            }
            formatted.push_str(token);
        }
        output.push_str(&source[cursor..content_start]);
        output.push_str(&formatted);
        output.push('"');
        cursor = close_quote + 1;
    }
    Ok(if source.is_empty() {
        String::new()
    } else {
        output
    })
}

fn looks_like_shell_assignment(line: &str) -> bool {
    let trimmed = line.trim_start_matches([' ', '\t']);
    let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let Some((name, _)) = trimmed.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        && name
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_uppercase() || *byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_nfqws_args_one_argument_per_line_and_preserves_other_text() {
        let source = "# header\r\nNFQWS_ARGS=\"--filter-tcp=443   --lua-desync=fake\r\n --new --filter-tcp=80\" # keep\r\nISP_INTERFACE=wan\r\n";
        let formatted = format_nfqws_config(source).unwrap();
        assert_eq!(
            formatted,
            "# header\r\nNFQWS_ARGS=\"--filter-tcp=443\r\n    --lua-desync=fake\r\n\r\n    --new\r\n    --filter-tcp=80\" # keep\r\nISP_INTERFACE=wan\r\n"
        );
        assert_eq!(format_nfqws_config(&formatted).unwrap(), formatted);
    }

    #[test]
    fn refuses_values_with_ambiguous_quotes_or_shell_expansions() {
        assert!(format_nfqws_config("NFQWS_ARGS=\"--filter-tcp=443 \"$EXTRA\"\"\n").is_err());
        assert!(format_nfqws_config("NFQWS_ARGS=\"--filter-tcp=443\n").is_err());
        assert!(
            format_nfqws_config("NFQWS_ARGS=\"--filter-tcp=443\nISP_INTERFACE=\"wan\"\n").is_err()
        );
        assert!(
            format_nfqws_config("NFQWS_ARGS=\"--filter-tcp=443\nISP_INTERFACE=\"#wan\"\n").is_err()
        );
    }
}
