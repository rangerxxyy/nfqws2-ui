import type { NfqwsLuaFunctionInfo, NfqwsLuaStandardGroup } from './api';
import { diagnoseProfileLuaArguments } from './profile-editor';
import { configAssignmentValue } from './config-values';

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

function loadedLuaScriptNames(content: string): Set<string> | null {
  const baseArgs = configAssignmentValue(content, 'NFQWS_BASE_ARGS');
  if (/\$(?:\(|\{?[A-Za-z_]|`)/.test(baseArgs)) return null;
  const scripts = new Set<string>();
  const expression = /(?:^|\s)--lua-init=(?:"@([^"]+)"|'@([^']+)'|@([^\s"'#]+))/g;
  for (const match of baseArgs.matchAll(expression)) {
    const path = match[1] ?? match[2] ?? match[3];
    const name = path?.split(/[\\/]/).pop();
    if (name) scripts.add(name);
  }
  return scripts;
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
  const loadedScripts = loadedLuaScriptNames(content);
  return configLuaArgumentEntries(content).flatMap(({ argument, line }) => {
    const diagnostics = diagnoseProfileLuaArguments([argument], catalog, groups)
      .map(({ message }) => ({ argument, line, message }));
    const functionName = argument.slice('--lua-desync='.length).split(':', 1)[0] ?? '';
    const functionInfo = catalog.find(({ name }) => name === functionName);
    if (!functionInfo || !loadedScripts || loadedScripts.has(functionInfo.file.split(/[\\/]/).pop() ?? functionInfo.file)) return diagnostics;
    diagnostics.push({ argument, line, message: `Файл ${functionInfo.file} для функции ${functionInfo.name} не подключён через --lua-init` });
    return diagnostics;
  });
}
