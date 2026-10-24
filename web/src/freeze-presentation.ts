export type FreezeRowFacts = { freezeDetected: boolean; foundStrategy: { reusedFrom?: string | null } | null; directOk: boolean; currentOk: boolean };
export type FreezeRowTone = 'error' | 'success' | 'warning' | 'neutral';

export function freezeRowPresentation(facts: FreezeRowFacts): { tone: FreezeRowTone; label: string } {
  if (facts.foundStrategy?.reusedFrom) return { tone: 'warning', label: 'Совпадение в той же AS' };
  if (facts.foundStrategy) return { tone: 'success', label: 'Стратегия найдена' };
  if (facts.freezeDetected) return { tone: 'error', label: 'Обрыв подтверждён' };
  if (facts.directOk && facts.currentOk) return { tone: 'success', label: 'Обрыв не обнаружен' };
  return { tone: 'neutral', label: 'Результат не подтверждён' };
}

export function freezeCanApplyToHost(input: {
  state: string;
  targetKind: string;
  freezeDetected: boolean;
  foundStrategy: { attempts: number; successes: number; reusedFrom?: string | null } | null;
}): boolean {
  return input.state === 'complete'
    && input.targetKind === 'user_host'
    && input.freezeDetected
    && input.foundStrategy !== null
    && input.foundStrategy.reusedFrom == null
    && input.foundStrategy.attempts >= 3
    && input.foundStrategy.successes >= 3;
}
