export type ConfigLuaArgumentEntry = { argument: string; line: number };

/** Extract active --lua-desync tokens from config lines without evaluating shell assignments. */
export function configLuaArgumentEntries(content: string): ConfigLuaArgumentEntry[] {
  const entries: ConfigLuaArgumentEntry[] = [];
  const lines = content.split(/\r\n|\r|\n/);
  lines.forEach((line, index) => {
    if (line.trimStart().startsWith('#')) return;
    const expression = /--lua-desync=(?:"([^"]*)"|'([^']*)'|([^\s"'#]+))/g;
    for (const match of line.matchAll(expression)) {
      const value = match[1] ?? match[2] ?? match[3];
      if (value) entries.push({ argument: `--lua-desync=${value}`, line: index + 1 });
    }
  });
  return entries;
}
