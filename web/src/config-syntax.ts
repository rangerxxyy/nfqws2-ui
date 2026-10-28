function escapeConfigHtml(value: string): string {
  return value.replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character] ?? character);
}

function highlightConfigTokens(source: string, comments: boolean): string {
  let html = '';
  let index = 0;
  let inLua = false;
  let afterLuaFlag = false;
  let awaitingLuaMode = false;
  let awaitingParamKey = false;
  let awaitingParamValue = false;
  let awaitingFlagValue = false;
  const append = (value: string, token?: string) => {
    const safe = escapeConfigHtml(value);
    html += token ? `<span class="conf-token-${token}">${safe}</span>` : safe;
  };

  while (index < source.length) {
    const rest = source.slice(index);
    if (comments && rest.startsWith('#')) { append(rest, 'comment'); break; }
    if (/\s/.test(rest[0])) {
      awaitingFlagValue = false;
      append(rest[0]); index += 1; continue;
    }
    if (rest[0] === '"' || rest[0] === "'") {
      const quote = rest[0]; let end = 1; let escaped = false;
      while (end < rest.length) { const ch = rest[end]; if (ch === quote && !escaped) break; escaped = ch === '\\' && !escaped; if (ch !== '\\') escaped = false; end += 1; }
      if (end < rest.length) {
        append(quote, 'string');
        if (quote === '"') html += highlightConfigTokens(rest.slice(1, end), false);
        else append(rest.slice(1, end), 'string');
        append(quote, 'string'); index += end + 1; continue;
      }
      append(quote, 'string'); index += 1; continue;
    }
    const variable = rest.match(/^\$\{?[A-Za-z_][A-Za-z0-9_]*\}?/);
    if (variable) { append(variable[0], 'variable'); index += variable[0].length; continue; }
    const flag = rest.match(/^--[A-Za-z0-9_-]+/);
    if (flag) {
      append(flag[0], 'flag');
      if (flag[0] === '--lua-desync') { inLua = true; afterLuaFlag = true; awaitingLuaMode = false; awaitingParamKey = false; awaitingParamValue = false; awaitingFlagValue = false; }
      else awaitingFlagValue = true;
      index += flag[0].length; continue;
    }
    const delimiter = rest.match(/^[=,:]/);
    if (delimiter) {
      append(delimiter[0], 'operator');
      if (delimiter[0] === '=' && afterLuaFlag) { afterLuaFlag = false; awaitingLuaMode = true; }
      else if (delimiter[0] === ':') { awaitingParamKey = true; awaitingParamValue = false; }
      else if (delimiter[0] === '=' && awaitingParamKey) { awaitingParamKey = false; awaitingParamValue = true; }
      else if (delimiter[0] === '=' && awaitingFlagValue) { /* keep the flag's value state through comma-separated values */ }
      index += 1; continue;
    }
    const number = rest.match(/^-?\d+(?:[._+-]\d+)*%?/);
    if (number) {
      const token = awaitingParamValue && inLua || awaitingFlagValue ? 'value' : 'number';
      append(number[0], token);
      if (awaitingParamValue && inLua) awaitingParamValue = false;
      index += number[0].length; continue;
    }
    const word = rest.match(/^[A-Za-z_][A-Za-z0-9_.-]*/);
    if (word) {
      const value = word[0]; const isKey = rest.slice(value.length).startsWith('=');
      if (awaitingParamKey && inLua) append(value, 'key');
      else if (isKey) append(value, 'key');
      else if (awaitingLuaMode) { append(value, 'mode'); awaitingLuaMode = false; }
      else if (awaitingParamValue && inLua || awaitingFlagValue) { append(value, 'value'); awaitingParamValue = false; }
      else if (/^(if|then|else|elif|fi|for|while|do|done|case|esac|in|export|local|return|function)$/i.test(value)) append(value, 'keyword');
      else append(value, 'word');
      index += value.length; continue;
    }
    append(source[index]); index += 1;
  }
  return html;
}

export function highlightConfigLine(line: string): string {
  const trimmed = line.trimStart();
  if (trimmed.startsWith('#')) return escapeConfigHtml(line.slice(0, line.length - trimmed.length)) + highlightConfigTokens(trimmed, true);
  return highlightConfigTokens(line, true) || '&nbsp;';
}
