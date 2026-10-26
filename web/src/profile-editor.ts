import type { NfqwsArgumentProfile, NfqwsCliCatalog, NfqwsListSummary, NfqwsLuaFunctionInfo, NfqwsLuaStandardGroup, NfqwsProfileComponent } from './api';

export type ProfileArgumentChoice = { group: string; value: string; label: string };
export type RuntimeProfileConstructorTarget = { source: string; args: string[]; linkedComponents: NfqwsProfileComponent[]; managed: boolean };

/** Shapes only the built-in constructor navigation; stored config sources and custom names stay intact. */
export function prepareVisibleRuntimeProfiles(profiles: readonly NfqwsArgumentProfile[]): NfqwsArgumentProfile[] {
  return profiles
    .filter((profile) => profile.source !== 'NFQWS_ARGS_IPSET')
    .map((profile, index) => ({
      ...profile,
      name: profile.source === 'NFQWS_ARGS_QUIC_IPSET' ? 'QUIC' : profile.name,
      index: index + 1,
    }));
}

/** Returns a new custom-profile draft with one argument added to exactly one --new-delimited profile. */
export function appendArgumentToCustomProfile(profiles: readonly string[][], profileIndex: number, argument: string): string[][] | null {
  if (!Number.isInteger(profileIndex) || profileIndex < 0 || profileIndex >= profiles.length || !argument.trim()) return null;
  return profiles.map((profile, index) => index === profileIndex ? [...profile, argument] : [...profile]);
}

/** Picks the actual config variable edited by a profile's constructor. Managed profile summaries can contain several shared variables. */
export function getRuntimeProfileConstructorTarget(profile: NfqwsArgumentProfile): RuntimeProfileConstructorTarget | null {
  if (profile.editable) {
    return { source: profile.source, args: [...profile.args], linkedComponents: [], managed: false };
  }
  const primary = profile.components?.[0];
  if (!primary?.editable) return null;
  return { source: primary.source, args: [...primary.args], linkedComponents: (profile.components ?? []).slice(1), managed: true };
}

/** Resolves list arguments to the managed profile's linked scope variable, or to UDP's own variable. */
export function getRuntimeProfileListTarget(profile: NfqwsArgumentProfile, argument: string): { source: string; editable: boolean } | null {
  if (profile.editable) return null;
  const source = argument.startsWith('--ipset')
    ? 'NFQWS_ARGS_IPSET'
    : /^(--hostlist|--autohostlist)/.test(argument)
      ? 'NFQWS_EXTRA_ARGS'
      : null;
  if (!source) return null;
  const component = profile.components?.find((item) => item.source === source);
  if (component) return { source: component.source, editable: component.editable };
  const primary = profile.components?.[0];
  return primary ? { source: primary.source, editable: primary.editable } : null;
}

export function getRuntimeProfileSharedComponentNote(source: string): string | null {
  if (source === 'NFQWS_ARGS_IPSET') return 'Общая переменная: изменения затронут профили «QUIC по IP» и «HTTP/HTTPS по IP».';
  if (source === 'NFQWS_EXTRA_ARGS') return 'Общая переменная: изменения затронут профили «QUIC по сайтам» и «HTTP/HTTPS по сайтам».';
  return null;
}

export function getRuntimeProfileComponentArgs(args: readonly string[]): { lists: string[]; other: string[] } {
  const isList = (argument: string) => argument.startsWith('--ipset') || argument.startsWith('--hostlist') || argument.startsWith('--autohostlist');
  return { lists: args.filter(isList), other: args.filter((argument) => !isList(argument)) };
}

/** Filters the runtime-backed profile argument catalog by visible label, syntax, or group. */
export function filterProfileArgumentChoices(
  choices: readonly ProfileArgumentChoice[],
  query: string,
): ProfileArgumentChoice[] {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return [...choices];
  return choices.filter(({ group, label, value }) =>
    `${group} ${label} ${value}`.toLocaleLowerCase().includes(needle));
}

/** Returns the next option index for listbox keyboard navigation, or null for an unrelated key. */
export function nextProfileChoiceIndex(currentIndex: number, key: string, optionCount: number): number | null {
  if (optionCount <= 0) return null;
  const current = Number.isInteger(currentIndex) && currentIndex >= 0 && currentIndex < optionCount ? currentIndex : -1;
  if (key === 'Home') return 0;
  if (key === 'End') return optionCount - 1;
  if (key === 'ArrowDown') return current < 0 ? 0 : (current + 1) % optionCount;
  if (key === 'ArrowUp') return current < 0 ? optionCount - 1 : (current - 1 + optionCount) % optionCount;
  return null;
}

export function summarizeProfileSelection(args: readonly string[]): string {
  const parts: string[] = [];
  for (const argument of args) {
    if (argument.startsWith('--filter-tcp=')) parts.push(`TCP ${argument.slice('--filter-tcp='.length)}`);
    else if (argument.startsWith('--filter-udp=')) parts.push(`UDP ${argument.slice('--filter-udp='.length)}`);
    else if (argument.startsWith('--filter-l7=') || argument.startsWith('--filter-7=')) parts.push(argument.slice(argument.indexOf('=') + 1).toUpperCase());
  }
  if (!parts.length) parts.push('весь трафик');
  const hasHost = args.some((arg) => (arg.startsWith('--hostlist') || arg.startsWith('--autohostlist')) && !arg.startsWith('--hostlist-exclude'));
  const hasIp = args.some((arg) => arg.startsWith('--ipset') && !arg.startsWith('--ipset-exclude'));
  const hasExclude = args.some((arg) => arg.startsWith('--hostlist-exclude') || arg.startsWith('--ipset-exclude'));
  if (hasHost || hasIp) parts.push([hasHost ? 'домены' : '', hasIp ? 'IP' : ''].filter(Boolean).join(' + '));
  else if (hasExclude) parts.push('все, кроме исключений');
  else parts.push('без списков');
  return parts.join(' · ');
}

type PortRange = readonly [number, number];

function profilePortRanges(args: readonly string[], protocol: 'tcp' | 'udp'): PortRange[] {
  const values = args.filter((argument) => argument.startsWith(`--filter-${protocol}=`)).map((argument) => argument.slice(`--filter-${protocol}=`.length));
  const hasAnyTransportFilter = args.some((argument) => argument.startsWith('--filter-tcp=') || argument.startsWith('--filter-udp='));
  if (!values.length) return hasAnyTransportFilter ? [] : [[1, 65535]];
  const ranges: PortRange[] = [];
  for (const value of values.flatMap((item) => item.split(','))) {
    if (value === '*') { ranges.push([1, 65535]); continue; }
    const match = /^(\d+)(?:-(\d+))?$/.exec(value);
    if (!match) return [[1, 65535]];
    const first = Number(match[1]); const last = Number(match[2] ?? match[1]);
    if (first < 1 || last > 65535 || first > last) return [[1, 65535]];
    ranges.push([first, last]);
  }
  return ranges;
}

function profilePortsOverlap(first: readonly string[], second: readonly string[], protocol: 'tcp' | 'udp'): boolean {
  return profilePortRanges(first, protocol).some(([start, end]) => profilePortRanges(second, protocol)
    .some(([otherStart, otherEnd]) => start <= otherEnd && otherStart <= end));
}

function profileProtocolsOverlap(first: readonly string[], second: readonly string[]): boolean {
  const firstFilter = first.find((argument) => argument.startsWith('--filter-l7=') || argument.startsWith('--filter-7='));
  const secondFilter = second.find((argument) => argument.startsWith('--filter-l7=') || argument.startsWith('--filter-7='));
  if (!firstFilter || !secondFilter) return true;
  const firstProtocols = firstFilter.slice(firstFilter.indexOf('=') + 1).split(',');
  return secondFilter.slice(secondFilter.indexOf('=') + 1).split(',').some((protocol) => firstProtocols.includes(protocol));
}

/** Mirrors the reference's conservative first-match warning for unscoped earlier profiles. */
export function profileOrderWarnings(profiles: readonly (readonly string[])[]): string[][] {
  return profiles.map((args, index) => {
    for (let previous = 0; previous < index; previous += 1) {
      const earlier = profiles[previous] ?? [];
      const earlierHasListScope = earlier.some((argument) => argument.startsWith('--hostlist=')
        || argument.startsWith('--hostlist-domains=') || argument.startsWith('--hostlist-auto=')
        || argument.startsWith('--ipset=') || argument.startsWith('--ipset-ip='));
      if (earlierHasListScope || !profileProtocolsOverlap(earlier, args)) continue;
      if (profilePortsOverlap(earlier, args, 'tcp') || profilePortsOverlap(earlier, args, 'udp')) {
        return [`Трафик может быть обработан профилем #${previous + 1} раньше этого профиля.`];
      }
    }
    return [];
  });
}

/** Returns safe starter values for the full profile rule: match conditions, scope and ordered actions. */
export function profileArgumentChoices(args: readonly string[], lists: readonly NfqwsListSummary[], cli?: NfqwsCliCatalog): ProfileArgumentChoice[] {
  const definitions: ProfileArgumentChoice[] = [
    { group: 'Когда срабатывает · транспорт', value: '--filter-tcp=80', label: 'TCP-порты · 80' },
    { group: 'Когда срабатывает · транспорт', value: '--filter-tcp=443', label: 'TCP-порты · 443' },
    { group: 'Когда срабатывает · транспорт', value: '--filter-tcp=8080', label: 'TCP-порты · 8080' },
    { group: 'Когда срабатывает · транспорт', value: '--filter-udp=443', label: 'UDP-порты · 443' },
    { group: 'Когда срабатывает · протокол', value: '--filter-l7=tls', label: 'TLS' },
    { group: 'Когда срабатывает · протокол', value: '--filter-l7=http', label: 'HTTP' },
    { group: 'Когда срабатывает · протокол', value: '--filter-l7=quic', label: 'QUIC' },
    { group: 'Для кого · домены', value: '--hostlist-domains=example.com', label: 'Домен · example.com' },
    { group: 'Для кого · домены', value: '--hostlist-exclude-domains=example.com', label: 'Исключить домен · example.com' },
    { group: 'Для кого · IP', value: '--ipset-ip=192.0.2.1', label: 'IP-адрес · 192.0.2.1' },
    { group: 'Для кого · IP', value: '--ipset-exclude-ip=192.0.2.1', label: 'Исключить IP · 192.0.2.1' },
    { group: 'Что делает · пакеты', value: '--payload=tls_client_hello', label: 'TLS ClientHello' },
    { group: 'Что делает · пакеты', value: '--payload=quic_initial', label: 'QUIC Initial' },
    { group: 'Что делает · диапазон', value: '--out-range=1', label: 'Диапазон исходящих пакетов' },
    { group: 'Что делает · диапазон', value: '--in-range=1', label: 'Диапазон входящих пакетов' },
    { group: 'Что делает · стратегия', value: '--lua-desync=fake', label: 'Lua-шаг · fake' },
  ];
  for (const list of lists) {
    const path = list.path;
    const label = list.id;
    if (/ipset/i.test(label) || /ipset|ipset4/i.test(path)) {
      definitions.push({ group: 'Для кого · IP', value: `--ipset=${path}`, label: `IP-список · ${label}` });
      definitions.push({ group: 'Для кого · IP', value: `--ipset-exclude=${path}`, label: `Исключить IP-список · ${label}` });
    } else {
      definitions.push({ group: 'Для кого · домены', value: `--hostlist=${path}`, label: `Список доменов · ${label}` });
      definitions.push({ group: 'Для кого · домены', value: `--hostlist-exclude=${path}`, label: `Исключить список · ${label}` });
      if (/auto/i.test(label) || /auto/i.test(path)) definitions.push({ group: 'Для кого · домены', value: `--hostlist-auto=${path}`, label: `Автосписок доменов · ${label}` });
    }
  }
  if (cli) {
    for (const protocol of cli.l7) definitions.push({ group: 'Когда срабатывает · протокол', value: `--filter-l7=${protocol}`, label: `Протокол · ${protocol}` });
    for (const payload of cli.payloads) definitions.push({ group: 'Что делает · пакеты', value: `--payload=${payload}`, label: `Тип пакета · ${payload}` });
    for (const option of cli.options) {
      // --new separates profiles and is rejected inside a profile body; --name
      // already has a dedicated editor field and must not be duplicated here.
      if (option.global || ['new', 'name', 'filter-l7', 'payload'].includes(option.name)) continue;
      const value = option.takesValue ? `--${option.name}=` : `--${option.name}`;
      definitions.push({ group: 'Параметры nfqws2', value, label: `${option.syntax} · ${getRussianNfqwsOptionDescription(option.name)}` });
    }
  }
  return definitions.filter(({ value }, index) => {
    // Repeated Lua steps are meaningful (for example, different desync stages);
    // only hide an identical starter choice for the other argument kinds.
    const firstChoiceForValue = definitions.findIndex((choice) => choice.value === value) === index;
    return firstChoiceForValue && (value.startsWith('--lua-desync=') || !args.includes(value));
  });
}

const PROFILE_FILTER_PREFIX = /^--filter-(?:tcp|udp|l7|7)=/;

/** Returns comma-separated match values for TCP/UDP/L7 filters. */
export function profileFilterValues(argument: string): string[] {
  if (!PROFILE_FILTER_PREFIX.test(argument)) return [];
  return argument.slice(argument.indexOf('=') + 1).split(',').map((value) => value.trim()).filter(Boolean);
}

function replaceProfileFilterValues(argument: string, values: readonly string[]): string | null {
  if (!PROFILE_FILTER_PREFIX.test(argument) || values.some((value) => !value || /[,\s\0\r\n'"`$\\;&|]/.test(value))) return null;
  return `${argument.slice(0, argument.indexOf('=') + 1)}${values.join(',')}`;
}

export function updateProfileFilterValue(argument: string, index: number, value: string): string | null {
  const values = profileFilterValues(argument);
  if (!Number.isInteger(index) || index < 0 || index >= values.length) return null;
  values[index] = value.trim();
  return replaceProfileFilterValues(argument, values);
}

export function removeProfileFilterValue(argument: string, index: number): string | null {
  const values = profileFilterValues(argument);
  if (!Number.isInteger(index) || index < 0 || index >= values.length) return null;
  values.splice(index, 1);
  return values.length ? replaceProfileFilterValues(argument, values) : null;
}

export function appendProfileFilterValue(argument: string, value: string): string | null {
  const values = profileFilterValues(argument);
  if (!values.length || !value.trim()) return null;
  values.push(value.trim());
  return replaceProfileFilterValues(argument, values);
}

export type ProfileLuaWarning = { argument: string; message: string };
export type LuaParameterOption = { key: string; value: string | null; display: string };
export type LuaProfileHelp = {
  docs: string[];
  arguments: string[];
  fileArguments: string[];
  standardGroups: Array<{ name: string; arguments: string[] }>;
  required: string[];
};

// Some shipped Lua files document only a subset of options supported by the
// nfqws2 runtime. These known options remain function-scoped in the editor.
const compatibilityLuaArguments: Record<string, Readonly<Record<string, string | null>>> = {
  fake: { repeats: '2' },
  fakedsplit: { altorder: '1', nofake1: null, nofake2: null, nofake3: null, nofake4: null, oob: null, oob_data: '0x00' },
  fakeddisorder: { pos: 'midsld', altorder: '1', nofake1: null, nofake2: null, nofake3: null, nofake4: null, oob: null, oob_data: '0x00' },
  hostfakesplit: { altorder: '1', nofake1: null, nofake2: null, oob: null, oob_data: '0x00' },
};

export function getLuaProfileHelp(
  info: NfqwsLuaFunctionInfo | undefined,
  groups: readonly NfqwsLuaStandardGroup[],
): LuaProfileHelp | null {
  if (!info) return null;
  const standardGroups = info.standardGroups.flatMap((name) => {
    const group = groups.find((item) => item.name === name);
    return group ? [{ name, arguments: [...group.arguments] }] : [];
  });
  return {
    docs: [...info.docs],
    arguments: [...info.arguments],
    fileArguments: [...(info.fileArguments ?? [])],
    standardGroups,
    required: [...info.required],
  };
}

export function getLuaProfileParameterOptions(
  info: NfqwsLuaFunctionInfo | undefined,
  groups: readonly NfqwsLuaStandardGroup[],
  selectedFunctionName = info?.name,
): string[] {
  if (!info && !selectedFunctionName) return [];
  const functionName = info?.name ?? selectedFunctionName!;
  const compatible = compatibilityLuaArguments[functionName] ?? {};
  const definitions = [
    ...(info?.arguments ?? []).filter((definition) => {
      const option = parseLuaParameterOption(definition);
      return !(option && Object.hasOwn(compatible, option.key) && definition.includes('обнаружен в коде функции'));
    }),
    ...groups.filter((group) => info?.standardGroups.includes(group.name)).flatMap((group) => group.arguments),
    ...(info?.fileArguments ?? []),
    'strategy=1 — номер стратегии для circular',
    ...Object.entries(compatible).map(([key, value]) => `${value === null ? key : `${key}=${value}`} — поддерживается nfqws2`),
  ];
  const seen = new Set<string>();
  return definitions.filter((definition) => {
    const option = parseLuaParameterOption(definition);
    if (!option || seen.has(option.key)) return false;
    seen.add(option.key);
    return true;
  });
}

export function parseLuaParameterOption(definition: string): LuaParameterOption | null {
  const display = definition.trim();
  const token = display.split(/[\s—]/, 1)[0] ?? '';
  const remainder = display.slice(token.length).trimStart();
  if (remainder && !/^[-—:]/.test(remainder)) return null;
  // Lua help uses key[=value] for options whose value is optional. Keep the
  // key usable as a flag by default (for example, tcp_md5[=hex] -> tcp_md5).
  const optionalValue = /^([a-z][a-z0-9_]*)\[=([^\]]+)\]$/.exec(token);
  if (optionalValue) return { key: optionalValue[1]!, value: null, display };
  const separator = token.indexOf('=');
  const key = (separator < 0 ? token : token.slice(0, separator)).replace(/\[[^\]]*\]$/, '');
  if (!/^[a-z][a-z0-9_]*$/.test(key)) return null;
  const example = separator < 0 ? null : token.slice(separator + 1);
  const value = example === null ? null : example.startsWith('<') || example.endsWith('>') ? '' : example;
  return { key, value, display };
}

const luaFunctionDescriptionsRu: Readonly<Record<string, string>> = {
  circular: 'При ошибках последовательно переключает номера стратегий и повторяет запрос.',
  fake: 'Отправляет поддельный пакет перед исходным, чтобы изменить видимый для провайдера запрос.',
  fakedsplit: 'Разделяет поддельный пакет на части и меняет их порядок или содержимое.',
  fakeddisorder: 'Меняет порядок частей поддельного пакета перед отправкой.',
  hostfakesplit: 'Применяет подмену только к указанному имени сайта и поддельным пакетам.',
  hostfake: 'Подставляет указанное имя сайта в поддельный пакет.',
  multisplit: 'Разделяет запрос на несколько частей в указанных позициях.',
  multidisorder: 'Разделяет запрос на части и отправляет их в заданном порядке.',
  tcpseg: 'Разбивает TCP-запрос на сегменты в заданных позициях.',
  udplen: 'Меняет длину UDP-пакета, добавляя или убирая данные.',
  badseq: 'Изменяет номер последовательности TCP-пакета.',
  disorder: 'Меняет порядок отправки частей исходного запроса.',
  split: 'Разделяет исходный запрос на части в выбранных позициях.',
  tamper: 'Изменяет выбранные поля запроса перед отправкой.',
};

const luaArgumentDescriptionsRu: Readonly<Record<string, string>> = {
  altorder: 'чередовать порядок отправки частей',
  badsum: 'подменить контрольную сумму поддельного пакета',
  blob: 'использовать указанный файл данных вместо встроенного шаблона',
  disorder_after: 'отправить часть после указанной позиции',
  fails: 'число неудачных запросов до переключения стратегии',
  host: 'шаблон доменного имени для поддельного запроса',
  ip_autottl: 'автоматически подобрать TTL для поддельного IP-пакета',
  midhost: 'указать имя сайта для подмены в середине запроса',
  nld: 'количество повторов стратегии без ответа',
  pos: 'позиция или список позиций изменения/разделения пакета',
  nofake1: 'не отправлять поддельный пакет первого типа',
  nofake2: 'не отправлять поддельный пакет второго типа',
  nofake3: 'не отправлять поддельный пакет третьего типа',
  nofake4: 'не отправлять поддельный пакет четвёртого типа',
  oob: 'передать байт вне основного потока TCP',
  oob_data: 'значение байта, передаваемого вне основного потока TCP',
  pattern: 'шаблон содержимого поддельного пакета',
  repeats: 'число повторов указанного действия, обычно отправки фейка',
  retrans: 'число повторных передач до переключения стратегии',
  seqovl: 'перекрыть часть номера последовательности',
  seqovl_pattern: 'шаблон данных для перекрытия',
  strategy: 'номер стратегии для механизма circular',
  tcp_ack: 'смещение номера подтверждения TCP',
  tcp_md5: 'добавить заголовок TCP MD5; значение необязательно',
  tcp_ts_up: 'изменить временную метку TCP',
  time: 'интервал в секундах для сброса счётчика ошибок',
};

/** Keeps source Lua comments out of the Russian UI while retaining argument names verbatim. */
export function getRussianLuaFunctionDescription(name: string): string {
  if (name === 'circular') return 'Переключает стратегии по кругу после заданного числа ошибок.';
  return luaFunctionDescriptionsRu[name] ?? `Функция Lua «${name}» из установленных скриптов nfqws2.`;
}

/** Provides a Russian help line even when the installed Lua file documents a key only in English. */
export function getRussianLuaArgumentDescription(definition: string): string {
  const parsed = parseLuaParameterOption(definition);
  if (!parsed) return 'Аргумент Lua из справки установленного скрипта.';
  return `${parsed.key} — ${luaArgumentDescriptionsRu[parsed.key] ?? 'параметр шага Lua; формат значения зависит от выбранной функции.'}`;
}

const nfqwsOptionDescriptionsRu: Readonly<Record<string, string>> = {
  'filter-tcp': 'ограничить профиль TCP-портами',
  'filter-udp': 'ограничить профиль UDP-портами',
  'filter-l7': 'ограничить профиль сетевыми протоколами',
  'filter-7': 'ограничить профиль сетевыми протоколами',
  'hostlist': 'применять профиль к доменам из списка',
  'hostlist-exclude': 'исключить домены из списка',
  'hostlist-domains': 'применять профиль к указанным доменам',
  'hostlist-exclude-domains': 'исключить указанные домены',
  'ipset': 'применять профиль к адресам из IP-списка',
  'ipset-exclude': 'исключить адреса из IP-списка',
  'ipset-ip': 'применять профиль к указанному IP-адресу',
  'ipset-exclude-ip': 'исключить указанный IP-адрес',
  'autohostlist': 'добавлять доступные домены в автосписок',
  'hostlist-auto': 'добавлять доступные домены в автосписок',
  'payload': 'выбрать тип пакетов для Lua-обработки',
  'lua-desync': 'добавить Lua-функцию в последовательность обработки',
  'in-range': 'ограничить обработку входящими пакетами',
  'out-range': 'ограничить обработку исходящими пакетами',
  'name': 'задать отображаемое имя профиля',
  'new': 'начать следующий профиль в конфигурации',
  'skip': 'пропустить обработку этого профиля',
};

/** Replaces English nfqws2 help text with Russian context while keeping exact CLI syntax. */
export function getRussianNfqwsOptionDescription(name: string): string {
  return nfqwsOptionDescriptionsRu[name] ?? 'параметр nfqws2; точный формат значения задаёт справка установленной версии.';
}

/** Reports only issues that can be proven from the installed Lua catalog. */
export function diagnoseProfileLuaArguments(
  args: readonly string[],
  catalog: readonly NfqwsLuaFunctionInfo[],
  groups: readonly NfqwsLuaStandardGroup[] = [],
): ProfileLuaWarning[] {
  if (!catalog.length) return [];
  // Older bundled Lua help comments omit arguments that are accepted by the
  // nfqws2 desync implementations. Keep this compatibility list narrow and
  // function-scoped so real misspellings still produce a warning.
  const known = new Map(catalog.map((item) => [item.name, item]));
  const knownSomewhere = new Set(['strategy', 'final', 'cond', 'cond_neg']);
  for (const item of catalog) {
    for (const definition of [...item.arguments, ...(item.fileArguments ?? [])]) {
      const parsed = parseLuaParameterOption(definition);
      if (parsed) knownSomewhere.add(parsed.key);
    }
  }
  for (const group of groups) {
    for (const definition of group.arguments) {
      const parsed = parseLuaParameterOption(definition);
      if (parsed) knownSomewhere.add(parsed.key);
    }
  }
  for (const options of Object.values(compatibilityLuaArguments)) for (const key of Object.keys(options)) knownSomewhere.add(key);
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
    const allowed = new Set(['strategy', 'final', 'cond', 'cond_neg', ...(info.fileArguments ?? [])
      .map(parseLuaParameterOption).filter((item) => item !== null).map((item) => item.key)]);
    for (const key of Object.keys(compatibilityLuaArguments[name] ?? {})) allowed.add(key);
    for (const definition of info.arguments) {
      const parsed = parseLuaParameterOption(definition);
      if (parsed) allowed.add(parsed.key);
    }
    for (const groupName of info.standardGroups) {
      const group = groups.find((item) => item.name === groupName);
      for (const definition of group?.arguments ?? []) {
        const parsed = parseLuaParameterOption(definition);
        if (parsed) allowed.add(parsed.key);
      }
    }
    const hasParameterInventory = info.arguments.length > 0
      || (info.fileArguments?.length ?? 0) > 0
      || info.standardGroups.some((groupName) => groups.some((group) => group.name === groupName));
    if (!hasParameterInventory) continue;
    for (const parameter of parameters) {
      const key = parameter.split('=', 1)[0] ?? '';
      if (!key || allowed.has(key)) continue;
      warnings.push({
        argument,
        message: knownSomewhere.has(key)
          ? `Параметр «${key}» не описан для функции ${name}`
          : `Неизвестный параметр «${key}»`,
      });
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

export function moveProfileStepTo(args: readonly string[], fromIndex: number, toIndex: number): string[] {
  const isOrderedStep = (argument: string) => argument.startsWith('--lua-desync=')
    || argument.startsWith('--payload=') || argument.startsWith('--in-range=') || argument.startsWith('--out-range=');
  const positions = args.flatMap((argument, index) => isOrderedStep(argument) ? [index] : []);
  if (![fromIndex, toIndex].every(Number.isInteger) || fromIndex < 0 || toIndex < 0 || fromIndex >= positions.length || toIndex >= positions.length) return [...args];
  const result = [...args];
  const direction = fromIndex < toIndex ? 1 : -1;
  for (let index = fromIndex; index !== toIndex; index += direction) {
    const positionsNow = result.flatMap((argument, position) => isOrderedStep(argument) ? [position] : []);
    const from = positionsNow[index]; const to = positionsNow[index + direction];
    if (from === undefined || to === undefined) return [...args];
    [result[from], result[to]] = [result[to]!, result[from]!];
  }
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
  if (value && (!/^[\p{L}\p{N}_.-]{1,32}$/u.test(value))) return [...args];
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
