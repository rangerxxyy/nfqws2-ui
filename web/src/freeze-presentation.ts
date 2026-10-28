import type { FreezeInConfigStatus } from './api';

export type FreezeRowFacts = { freezeDetected: boolean; foundStrategy: { reusedFrom?: string | null } | null; directOk: boolean; currentOk: boolean; directClassification?: string; currentClassification?: string; inConf?: FreezeInConfigStatus };
export type FreezeRowTone = 'error' | 'success' | 'warning' | 'neutral';

export function freezeInConfigNote(status?: FreezeInConfigStatus): string | null {
  if (!status) return null;
  if (status.state === 'working') {
    const profile = status.route ?? status.profiles[0];
    return profile ? `Уже работает: профиль #${profile.index} «${profile.name}» принимает этот адрес с найденной стратегией.` : 'Уже работает: найденная стратегия используется текущим маршрутом.';
  }
  if (status.state === 'elsewhere') {
    const found = status.profiles.map(profile => `#${profile.index} «${profile.name}»`).join(', ');
    const route = status.route ? `#${status.route.index} «${status.route.name}»` : 'неизвестный профиль';
    const why = status.why ? `: ${status.why}` : '';
    return `Найденный шаг есть в профиле${status.profiles.length === 1 ? '' : 'ях'} ${found || 'другой конфигурации'}, но сейчас адрес принимает профиль ${route}${why}.`;
  }
  if (status.state === 'unknown') return `Маршрут не подтверждён${status.why ? `: ${status.why}.` : '.'}`;
  return null;
}

export function freezeMeasurementDetails(direct: string, current: string): string {
  return `Напрямую: ${direct}\nЧерез nfqws2: ${current}`;
}

export function freezeRowPresentation(facts: FreezeRowFacts): { tone: FreezeRowTone; label: string } {
  if (facts.foundStrategy && facts.inConf?.state === 'working') return { tone: 'success', label: 'Стратегия уже работает' };
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
  inConfState?: FreezeInConfigStatus['state'];
}): boolean {
  return ['running', 'complete', 'partial', 'cancelled'].includes(input.state)
    && input.targetKind === 'user_host'
    && input.foundStrategy !== null
    && input.inConfState !== 'working'
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
