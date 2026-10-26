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
  // The current backend returns one complete report, not per-stage progress.
  // Keep the queue neutral until those real results arrive.
  if (!resultsReceived) return 'pending';
  if (index < activeIndex) return 'complete';
  return index === activeIndex ? 'active' : 'pending';
}
