export type FreezeRowFacts = { freezeDetected: boolean; foundStrategy: { reusedFrom?: string | null } | null; directOk: boolean; currentOk: boolean; directClassification?: string; currentClassification?: string };
export type FreezeRowTone = 'error' | 'success' | 'warning' | 'neutral';

export function freezeMeasurementDetails(direct: string, current: string): string {
  return `Напрямую: ${direct}\nЧерез nfqws2: ${current}`;
}

export function freezeRowPresentation(facts: FreezeRowFacts): { tone: FreezeRowTone; label: string } {
  if (facts.foundStrategy?.reusedFrom) return { tone: 'warning', label: 'Совпадение в той же AS' };
  if (facts.foundStrategy) return { tone: 'success', label: 'Стратегия найдена' };
  if (facts.directClassification === 'PossibleUploadStall' || facts.currentClassification === 'PossibleUploadStall') return { tone: 'warning', label: 'POST не завершился' };
  if (facts.freezeDetected) return { tone: 'error', label: 'Возможный обрыв' };
  if (facts.directOk && facts.currentOk) return { tone: 'success', label: 'Ответ получен' };
  return { tone: 'neutral', label: 'Результат не подтверждён' };
}

export function freezeCanApplyToHost(input: {
  state: string;
  targetKind: string;
  freezeDetected: boolean;
  foundStrategy: { attempts: number; successes: number; reusedFrom?: string | null } | null;
}): boolean {
  return ['running', 'complete', 'partial', 'cancelled'].includes(input.state)
    && input.targetKind === 'user_host'
    && input.foundStrategy !== null
    && input.foundStrategy.reusedFrom == null
    && input.foundStrategy.attempts >= 3
    && input.foundStrategy.successes >= 3;
}

export function freezeCanOfferProfileJoin(input: {
  targetKind: string;
  asnVerified: boolean;
  freezeDetected: boolean;
  foundStrategy: { attempts: number; successes: number; reusedFrom?: string | null } | null;
}): boolean {
  return input.targetKind === 'user_host'
    && input.asnVerified
    && input.freezeDetected
    && input.foundStrategy !== null
    && input.foundStrategy.reusedFrom == null
    && input.foundStrategy.attempts >= 3
    && input.foundStrategy.successes >= 3;
}
