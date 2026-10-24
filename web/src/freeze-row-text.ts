type FreezeRowIdentity = { label: string; host: string; path: string; targetKind: string };
type FreezeRowDetail = {
  foundStrategy: { sni: string; step: string; successes: number; attempts: number } | null;
  triedCandidates: number;
  candidateAttempts: { sni: string; successes: number; attempts: number; reason: string | null }[];
};

export function freezeRowIdentity(result: FreezeRowIdentity): string {
  if (result.targetKind === 'pinned_address') return result.label;
  const endpoint = `${result.host}${result.path === '/' ? '' : result.path}`;
  return result.targetKind === 'user_host' ? endpoint : `${result.label}\n${endpoint}`;
}

export function freezeRowDetails(result: FreezeRowDetail): { summary: string; details: string | null } {
  const attempts = result.candidateAttempts
    .filter((attempt) => attempt.sni !== result.foundStrategy?.sni)
    .slice(-3)
    .map((attempt) => `${attempt.sni} · ${attempt.successes}/${attempt.attempts}${attempt.reason ? ` · ${attempt.reason}` : ''}`)
    .join('\n');
  if (result.foundStrategy) {
    return {
      summary: `SNI ${result.foundStrategy.sni} · ${result.foundStrategy.successes}/${result.foundStrategy.attempts}`,
      details: [`Стратегия: ${result.foundStrategy.step}`, `Проверено имён: ${result.triedCandidates}`, attempts ? `Другие попытки: ${attempts}` : ''].filter(Boolean).join('\n'),
    };
  }
  if (result.triedCandidates > 0) {
    return {
      summary: `Проверено вариантов: ${result.triedCandidates} · рабочий не найден`,
      details: attempts ? `Последние попытки: ${attempts}` : null,
    };
  }
  return { summary: '', details: null };
}
