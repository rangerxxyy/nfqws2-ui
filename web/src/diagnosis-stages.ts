export const SITE_DIAGNOSIS_STAGES = [
  { id: 'dns', label: 'Имя сайта · DNS' },
  { id: 'tcp', label: 'Соединение · TCP 443' },
  { id: 'direct', label: 'Запрос напрямую' },
  { id: 'sni', label: 'Проверка имени · SNI' },
  { id: 'http', label: 'Открытый HTTP · порт 80' },
  { id: 'current', label: 'Текущий маршрут nfqws2' },
  { id: 'quic', label: 'QUIC · HTTP/3' },
] as const;

export function diagnosisQueueState(index: number, activeIndex: number): 'complete' | 'active' | 'pending' {
  if (index < activeIndex) return 'complete';
  return index === activeIndex ? 'active' : 'pending';
}
