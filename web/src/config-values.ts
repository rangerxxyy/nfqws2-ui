/** Read a literal shell assignment for display without evaluating shell code. */
export function configEditorLayout(value: string, minimumRows: number): {
  lineCount: number;
  visibleRows: number;
  lineNumbers: string;
} {
  const lineCount = Math.max(1, value.split(/\r\n|\r|\n/).length);
  return {
    lineCount,
    visibleRows: Math.min(10, Math.max(minimumRows, lineCount)),
    lineNumbers: Array.from({ length: lineCount }, (_, index) => String(index + 1)).join('\n'),
  };
}

export function configEditorViewportHeight(visibleRows: number): string {
  const rows = Math.min(10, Math.max(1, Math.floor(visibleRows)));
  return `calc(${rows} * 1.25rem + 1rem)`;
}

export function sameConfigText(left: string, right: string): boolean {
  return left.replace(/\r\n?/g, '\n') === right.replace(/\r\n?/g, '\n');
}

export function configDraftDisposition(
  draft: string,
  baselineContent: string,
  currentContent: string,
  baselineRevision: string,
  currentRevision: string,
): 'unchanged' | 'changed' | 'stale' {
  if (sameConfigText(draft, currentContent)) return 'unchanged';
  if (baselineRevision !== currentRevision || !sameConfigText(baselineContent, currentContent)) return 'stale';
  return 'changed';
}

export function configOutlineEntries(value: string): Array<{ name: string; line: number; offset: number }> {
  return [...value.matchAll(/^([A-Z_][A-Z0-9_]*)=/gm)].map((match) => ({
    name: match[1] ?? '',
    line: value.slice(0, match.index ?? 0).split(/\r\n|\r|\n/).length,
    offset: match.index ?? 0,
  }));
}

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
