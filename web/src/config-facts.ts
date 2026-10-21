const ARGUMENT_VARIABLES = [
  'NFQWS_BASE_ARGS',
  'NFQWS_ARGS',
  'NFQWS_ARGS_QUIC',
  'NFQWS_ARGS_UDP',
  'NFQWS_ARGS_IPSET',
  'NFQWS_ARGS_CUSTOM',
] as const;

export type ConfigSectionFact = {
  variable: (typeof ARGUMENT_VARIABLES)[number];
  strategies: number;
  profiles: number;
  filters: number;
};

export type NfqwsConfigFacts = { selectedMode: string | null; sections: ConfigSectionFact[] };

function assignmentValue(content: string, key: string): string | null {
  const assignment = new RegExp(`^\\s*${key}\\s*=`, 'm').exec(content);
  if (!assignment || assignment.index === undefined) return null;
  let cursor = assignment.index + assignment[0].length;
  while (cursor < content.length && /\s/.test(content[cursor]!)) cursor += 1;
  const quote = content[cursor] === '"' || content[cursor] === "'" ? content[cursor] : null;
  if (quote) cursor += 1;
  let value = '';
  let escaped = false;
  for (; cursor < content.length; cursor += 1) {
    const character = content[cursor]!;
    if (quote && !escaped && character === quote) break;
    if (!quote && (character === '\n' || character === '\r' || (character === '#' && (cursor === 0 || /\s/.test(content[cursor - 1]!))))) break;
    value += character;
    if (character === '\\' && quote !== "'") escaped = !escaped;
    else escaped = false;
  }
  return value.trim();
}

export function summarizeNfqwsConfig(content: string): NfqwsConfigFacts {
  const mode = assignmentValue(content, 'NFQWS_EXTRA_ARGS');
  const sections = ARGUMENT_VARIABLES.flatMap((variable) => {
    const value = assignmentValue(content, variable);
    if (!value || value.startsWith('$') || value.startsWith('${')) return [];
    const activeLines = value.split(/\r?\n/).filter((line) => !line.trimStart().startsWith('#'));
    const activeText = activeLines.join('\n');
    const count = (pattern: RegExp) => [...activeText.matchAll(pattern)].length;
    return [{
      variable,
      strategies: count(/(?:^|\s)--lua-desync=/g),
      profiles: count(/(?:^|\s)--new(?:=[^\s]+)?(?=\s|$)/g),
      filters: count(/(?:^|\s)--filter-(?:tcp|udp|l7)=/g),
    }];
  });
  return { selectedMode: mode || null, sections };
}
