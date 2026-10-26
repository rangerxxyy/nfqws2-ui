import type { NfqwsLuaFunctionInfo } from './api';

export type ProfileLuaWarning = { argument: string; message: string };
export type LuaParameterOption = { key: string; value: string | null; display: string };

export function parseLuaParameterOption(definition: string): LuaParameterOption | null {
  const display = definition.trim();
  const token = display.split(/[\s—]/, 1)[0] ?? '';
  const remainder = display.slice(token.length).trimStart();
  if (remainder && !/^[-—:]/.test(remainder)) return null;
  const separator = token.indexOf('=');
  const key = (separator < 0 ? token : token.slice(0, separator)).replace(/\[[^\]]*\]$/, '');
  if (!/^[a-z][a-z0-9_]*$/.test(key)) return null;
  const example = separator < 0 ? null : token.slice(separator + 1);
  const value = example === null ? null : example.startsWith('<') || example.endsWith('>') ? '' : example;
  return { key, value, display };
}

/** Reports only issues that can be proven from the installed Lua catalog. */
export function diagnoseProfileLuaArguments(args: readonly string[], catalog: readonly NfqwsLuaFunctionInfo[]): ProfileLuaWarning[] {
  if (!catalog.length) return [];
  const known = new Map(catalog.map((item) => [item.name, item]));
  const warnings: ProfileLuaWarning[] = [];
  for (const argument of args) {
    if (!argument.startsWith('--lua-desync=')) continue;
    const [name = '', ...parameters] = argument.slice('--lua-desync='.length).split(':');
    const info = known.get(name);
    if (!info) {
      warnings.push({ argument, message: `Функция ${name || '(пустое имя)'} отсутствует в установленных Lua-скриптах` });
      continue;
    }
    const supplied = new Set(parameters.map((value) => value.split('=', 1)[0]));
    for (const required of info.required) {
      if (!supplied.has(required)) warnings.push({ argument, message: `Для ${name} не указан обязательный параметр ${required}` });
    }
  }
  return warnings;
}

/** Reorders ordered payload/range/desync steps while retaining filter argument positions. */
export function reorderProfileSteps(args: readonly string[], stepIndex: number, direction: -1 | 1): string[] {
  const isOrderedStep = (argument: string) => argument.startsWith('--lua-desync=')
    || argument.startsWith('--payload=')
    || argument.startsWith('--in-range=')
    || argument.startsWith('--out-range=');
  const positions = args.flatMap((argument, index) => isOrderedStep(argument) ? [index] : []);
  const targetStepIndex = stepIndex + direction;
  if (stepIndex < 0 || targetStepIndex < 0 || stepIndex >= positions.length || targetStepIndex >= positions.length) return [...args];

  const result = [...args];
  const from = positions[stepIndex];
  const to = positions[targetStepIndex];
  if (from === undefined || to === undefined) return result;
  [result[from], result[to]] = [result[to]!, result[from]!];
  return result;
}

export function replaceDesyncFunction(argument: string, functionName: string): string {
  const prefix = '--lua-desync=';
  if (!argument.startsWith(prefix) || !/^[a-zA-Z0-9_-]+$/.test(functionName)) return argument;
  const [, ...parameters] = argument.slice(prefix.length).split(':');
  return `${prefix}${functionName}${parameters.length ? `:${parameters.join(':')}` : ''}`;
}

export function replaceDesyncParameters(argument: string, parameters: string): string {
  const prefix = '--lua-desync=';
  if (!argument.startsWith(prefix)) return argument;
  const [functionName] = argument.slice(prefix.length).split(':');
  if (!functionName) return argument;
  const value = parameters.trim().replace(/^:+/, '');
  return `${prefix}${functionName}${value ? `:${value}` : ''}`;
}

export function splitDesyncParameters(argument: string): string[] {
  if (!argument.startsWith('--lua-desync=')) return [];
  return argument.slice('--lua-desync='.length).split(':').slice(1).filter(Boolean);
}

export function updateDesyncParameter(argument: string, index: number, parameter: string): string {
  const params = splitDesyncParameters(argument);
  if (!Number.isInteger(index) || index < 0 || index >= params.length || !isOneDesyncParameter(parameter)) return argument;
  params[index] = parameter;
  return replaceDesyncParameters(argument, params.join(':'));
}

export function removeDesyncParameter(argument: string, index: number): string {
  const params = splitDesyncParameters(argument);
  if (!Number.isInteger(index) || index < 0 || index >= params.length) return argument;
  params.splice(index, 1);
  return replaceDesyncParameters(argument, params.join(':'));
}

export function appendDesyncParameter(argument: string, parameter: string): string {
  if (!isOneDesyncParameter(parameter)) return argument;
  const params = splitDesyncParameters(argument);
  params.push(parameter);
  return replaceDesyncParameters(argument, params.join(':'));
}

function isOneDesyncParameter(value: string): boolean {
  return Boolean(value) && !/[\s:\0\r\n'"`$\\;&|]/.test(value);
}

export function replaceProfileName(args: readonly string[], name: string): string[] {
  const value = name.trim();
  if (value && (!/^[\p{L}\p{N}_.-]{1,64}$/u.test(value))) return [...args];
  const result = args.filter((argument) => !argument.startsWith('--name='));
  if (value) result.unshift(`--name=${value}`);
  return result;
}

export function replaceProfileArgument(args: readonly string[], index: number, argument: string): string[] {
  if (!Number.isInteger(index) || index < 0 || index >= args.length) return [...args];
  if (!argument || /\s|[\0\r\n'"`$\\;&|]/.test(argument) || argument === '--new' || argument.startsWith('--new=')) return [...args];
  const result = [...args];
  result[index] = argument;
  return result;
}

export function serializeProfileForSharing(name: string, args: readonly string[], version: string | null): string {
  const versionLabel = version ? ` v${version.replace(/^v/i, '')}` : '';
  return `# nfqws2${versionLabel} · ${name}\n${args.join(' ')}`;
}

export type ProfilePasteResult =
  | { ok: true; profiles: string[][] }
  | { ok: false; reason: 'empty' | 'shell-syntax' | 'unclosed-quote' | 'profile-limit' | 'argument-limit' };

const PROFILE_ASSIGNMENTS = new Set(['NFQWS_ARGS_CUSTOM', 'NFQWS_ARGS', 'NFQWS_ARGS_UDP', 'NFQWS_ARGS_IPSET', 'NFQWS_EXTRA_ARGS']);

export function parseProfilePaste(source: string): ProfilePasteResult {
  if (!source.trim()) return { ok: false, reason: 'empty' };
  if (source.length > 512 * 1024 || source.includes('\0')) return { ok: false, reason: 'argument-limit' };
  const text = source.replace(/\\[ \t]*\r?\n/g, ' ');
  const tokens: string[] = [];
  let token = '';
  let active = false;
  let quote: "'" | '"' | null = null;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index]!;
    if ('$`;&|<>\\'.includes(character)) return { ok: false, reason: 'shell-syntax' };
    if (quote) {
      if (character === quote) quote = null;
      else token += character;
      active = true;
      continue;
    }
    if (character === "'" || character === '"') { quote = character; active = true; continue; }
    if (character === '#' && !active) {
      while (index < text.length && text[index] !== '\n') index += 1;
      continue;
    }
    if (/\s/.test(character)) {
      if (active) { tokens.push(token); token = ''; active = false; }
      continue;
    }
    token += character; active = true;
  }
  if (quote) return { ok: false, reason: 'unclosed-quote' };
  if (active) tokens.push(token);

  const args: string[] = [];
  for (const item of tokens) {
    const assignment = /^([A-Z_][A-Z0-9_]*)=(.*)$/.exec(item);
    if (assignment && PROFILE_ASSIGNMENTS.has(assignment[1]!)) {
      args.push(...(assignment[2] ?? '').split(/\s+/).filter(Boolean));
      continue;
    }
    if (item.startsWith('--')) args.push(item);
    else if (/^[a-z_][a-z0-9_]*:\S/.test(item)) args.push(`--lua-desync=${item}`);
  }
  if (args.length > 8192) return { ok: false, reason: 'argument-limit' };
  const profiles: string[][] = [[]];
  for (const argument of args) {
    if (argument === '--new' || argument.startsWith('--new=')) {
      const name = argument.slice('--new='.length);
      profiles.push(name ? [`--name=${name}`] : []);
    } else profiles[profiles.length - 1]!.push(argument);
  }
  if (profiles[0]?.length === 0 && profiles.length > 1) profiles.shift();
  if (profiles.length > 64) return { ok: false, reason: 'profile-limit' };
  const nonEmpty = profiles.filter((profile) => profile.length > 0);
  return nonEmpty.length ? { ok: true, profiles: nonEmpty } : { ok: false, reason: 'empty' };
}
