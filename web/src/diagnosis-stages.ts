import type { SiteDiagnosticStep } from './api';

export const SITE_DIAGNOSIS_STAGES = [
  { id: 'dns', label: 'Имя сайта · DNS' },
  { id: 'tcp', label: 'Соединение · TCP 443' },
  { id: 'direct', label: 'Запрос напрямую' },
  { id: 'sni', label: 'Проверка имени · SNI' },
  { id: 'http', label: 'Открытый HTTP · порт 80' },
  { id: 'current', label: 'Как открывается сейчас' },
  { id: 'quic', label: 'QUIC · HTTP/3' },
] as const;

export function isSkippedDiagnosisStep(step: SiteDiagnosticStep | null | undefined): boolean {
  return step?.status === 'unavailable' && step.detail.trimStart().startsWith('Не нужно:');
}

export function diagnosisQueueMarker(state: 'complete' | 'active' | 'pending', resultKind?: 'success' | 'failure' | 'unknown'): string {
  if (state === 'active') return '…';
  if (state === 'pending') return '·';
  return resultKind === 'failure' ? '×' : resultKind === 'unknown' ? '!' : '✓';
}

export function diagnosisProgressState(stageId: string, activeStage: string, completedStages: ReadonlySet<string>): 'complete' | 'active' | 'pending' {
  if (completedStages.has(stageId)) return 'complete';
  return stageId === activeStage ? 'active' : 'pending';
}
