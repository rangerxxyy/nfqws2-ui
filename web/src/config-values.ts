/** Read a literal shell assignment for display without evaluating shell code. */
export function configAssignmentValue(content: string, key: string): string {
  const lines = content.split(/\r?\n/);
  let source = '';
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (!line.trimStart().startsWith(`${key}=`)) continue;
    const equals = line.indexOf('=');
    const value = line.slice(equals + 1).replace(/^[\t ]+/, '');
    if (value.startsWith('"') || value.startsWith("'")) {
      const quote = value[0];
      source = value.slice(1);
      while (true) {
        let closing = -1;
        for (let offset = 0; offset < source.length; offset += 1) {
          if (source[offset] !== quote) continue;
          if (quote === '"' && source[offset - 1] === '\\') continue;
          closing = offset;
          break;
        }
        if (closing >= 0) return source.slice(0, closing);
        index += 1;
        if (index >= lines.length) return source;
        source += `\n${lines[index]}`;
      }
    }
    return value.replace(/\s+#.*$/, '').trimEnd();
  }
  return '';
}
