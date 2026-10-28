export const SITE_DIAGNOSIS_STAGES = [
  { id: 'dns', label: 'Имя сайта · DNS' },
  { id: 'tcp', label: 'Соединение · TCP 443' },
  { id: 'current', label: 'Текущий маршрут nfqws2' },
  { id: 'direct', label: 'Запрос напрямую' },
  { id: 'sni', label: 'Проверка имени · SNI' },
  { id: 'http', label: 'Открытый HTTP · порт 80' },
  { id: 'quic', label: 'QUIC · HTTP/3' },
] as const;

export function diagnosisQueueMarker(state: 'complete' | 'active' | 'pending', resultKind?: 'success' | 'failure' | 'unknown'): string {
  if (state === 'active') return '…';
  if (state === 'pending') return '·';
  return resultKind === 'failure' ? '×' : resultKind === 'unknown' ? '!' : '✓';
}

export function diagnosisProgressState(stageId: string, activeStage: string, completedStages: ReadonlySet<string>): 'complete' | 'active' | 'pending' {
  if (completedStages.has(stageId)) return 'complete';
  return stageId === activeStage ? 'active' : 'pending';
}
