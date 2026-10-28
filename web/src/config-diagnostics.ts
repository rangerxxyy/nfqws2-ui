import type { NfqwsLuaFunctionInfo, NfqwsLuaStandardGroup } from './api';
import { diagnoseProfileLuaArguments } from './profile-editor';

export type ConfigLuaArgumentEntry = { argument: string; line: number };
export type ConfigLuaWarning = { argument: string; line: number; message: string };

function beforeConfigArgumentComment(line: string): string {
  let escaped = false;
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    if (character === '#' && (index === 0 || /\s/.test(line[index - 1] ?? '')) && !escaped) return line.slice(0, index);
    if (character === '\\') escaped = !escaped;
    else escaped = false;
  }
  return line;
}

/** Extract active --lua-desync tokens from config lines without evaluating shell assignments. */
export function configLuaArgumentEntries(content: string): ConfigLuaArgumentEntry[] {
  const entries: ConfigLuaArgumentEntry[] = [];
  const lines = content.split(/\r\n|\r|\n/);
  lines.forEach((line, index) => {
    const activeLine = beforeConfigArgumentComment(line);
    const expression = /--lua-desync=(?:"([^"]*)"|'([^']*)'|([^\s"'#]+))/g;
    for (const match of activeLine.matchAll(expression)) {
      const value = match[1] ?? match[2] ?? match[3];
      if (value) entries.push({ argument: `--lua-desync=${value}`, line: index + 1 });
    }
  });
  return entries;
}

/** Check only literal Lua arguments against installed metadata; unresolved shell values stay unknown. */
export function diagnoseConfigLuaArguments(
  content: string,
  catalog: readonly NfqwsLuaFunctionInfo[],
  groups: readonly NfqwsLuaStandardGroup[] = [],
): ConfigLuaWarning[] {
  return configLuaArgumentEntries(content).flatMap(({ argument, line }) =>
    diagnoseProfileLuaArguments([argument], catalog, groups).map(({ message }) => ({ argument, line, message })),
  );
}
