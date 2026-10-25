export const SITE_DIAGNOSIS_STAGES = [
  { id: 'dns', label: 'Имя сайта · DNS' },
  { id: 'tcp', label: 'Соединение · TCP 443' },
  { id: 'direct', label: 'Запрос напрямую' },
  { id: 'sni', label: 'Проверка имени · SNI' },
  { id: 'http', label: 'Открытый HTTP · порт 80' },
  { id: 'current', label: 'Текущий маршрут nfqws2' },
  { id: 'quic', label: 'QUIC · HTTP/3' },
] as const;

export function diagnosisQueueMarker(state: 'complete' | 'active' | 'pending', resultKind?: 'success' | 'failure' | 'unknown'): string {
  if (state === 'active') return '…';
  if (state === 'pending') return '·';
  return resultKind === 'failure' ? '×' : resultKind === 'unknown' ? '!' : '✓';
}

export function diagnosisQueueState(index: number, activeIndex: number, resultsReceived = true): 'complete' | 'active' | 'pending' {
  if (!resultsReceived) return index === activeIndex ? 'active' : 'pending';
  if (index < activeIndex) return 'complete';
  return index === activeIndex ? 'active' : 'pending';
}

export function nextDiagnosisQueueIndex(currentIndex: number, stageCount: number): number | null {
  if (currentIndex < 0 || stageCount < 1 || currentIndex >= stageCount - 1) return null;
  return currentIndex + 1;
}
