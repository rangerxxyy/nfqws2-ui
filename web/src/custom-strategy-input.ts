const MAX_CUSTOM_STRATEGIES = 10;

/** Extracts Lua desync steps from pasted nfqws2 arguments without running a shell. */
export function parseCustomStrategies(input: string): string[][] {
  const sections = input.split(/(?:^|\s)--new(?=\s|$)/);
  const strategies = sections.flatMap((section) => {
    const steps: string[] = [];
    const argument = /--lua-desync=(?:"([^"]*)"|'([^']*)'|([^\s"']+))/g;
    for (const match of section.matchAll(argument)) {
      const value = match[1] ?? match[2] ?? match[3] ?? '';
      if (value) steps.push(`--lua-desync=${value}`);
    }
    return steps.length ? [steps] : [];
  });
  if (strategies.length > MAX_CUSTOM_STRATEGIES) {
    throw new Error(`Можно добавить не более ${MAX_CUSTOM_STRATEGIES} собственных стратегий за один запуск.`);
  }
  return strategies;
}
