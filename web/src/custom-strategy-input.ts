const MAX_CUSTOM_STRATEGIES = 10;

/** Extracts Lua desync steps from pasted nfqws2 arguments without running a shell. */
export function parseCustomStrategies(input: string): string[][] {
  const normalized = input.replace(/\\[ \t]*\r?\n/g, ' ');
  const strategies: string[][] = [];
  for (const line of normalized.split(/\r?\n/)) {
    if (/^\s*#/.test(line)) continue;
    const tokens = line
      .replace(/(--lua-desync=)(["'`])([^"'`]*?)\2/g, '$1$3')
      .replace(/["'`\\]/g, ' ')
      .replace(/^\s*[A-Z_]+=/, '')
      .split(/\s+/)
      .filter(Boolean);
    let steps: string[] = [];
    for (const token of tokens) {
      if (token === '--new') {
        if (steps.length) strategies.push(steps);
        steps = [];
      } else if (token.startsWith('--lua-desync=')) {
        steps.push(token);
      } else if (/^[a-z_][a-z0-9_]*:\S/.test(token)) {
        steps.push(`--lua-desync=${token}`);
      }
    }
    if (steps.length) strategies.push(steps);
  }
  if (strategies.length > MAX_CUSTOM_STRATEGIES) {
    throw new Error(`Можно добавить не более ${MAX_CUSTOM_STRATEGIES} собственных стратегий за один запуск.`);
  }
  return strategies;
}
