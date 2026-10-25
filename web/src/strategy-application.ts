/** Replace only strategy steps, retaining non-strategy base arguments. */
export function replaceBaseDesyncSteps(current: string, steps: string[]): string {
  if (steps.length === 0) throw new Error('Параметры кандидата недоступны.');
  if (steps.some((step) => !/^--lua-desync=\S+$/.test(step))) {
    throw new Error('Разрешены только параметры --lua-desync без пробелов.');
  }
  if (/[$`\\;|&\0]/.test(current)) {
    throw new Error('NFQWS_BASE_ARGS содержит shell-подстановки или команды; безопасное применение недоступно.');
  }
  const tokens = current.match(/"(?:\\.|[^"\\])*"|'[^']*'|[^\s]+/g) ?? [];
  const retained = tokens.filter((token) => {
    const value = token.startsWith('"') && token.endsWith('"') || token.startsWith("'") && token.endsWith("'")
      ? token.slice(1, -1)
      : token;
    return !value.startsWith('--lua-desync=');
  });
  return [...retained, ...steps].join(' ');
}
