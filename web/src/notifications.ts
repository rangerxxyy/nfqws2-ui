export type ToastTone = 'success' | 'error';
export type LifecycleEvent = 'start' | 'stop' | 'restart' | 'panel-update' | 'core-update';

export function lifecycleMessage(event: LifecycleEvent, version?: string): string {
  const base = {
    start: 'nfqws2 запущен',
    stop: 'nfqws2 остановлен',
    restart: 'nfqws2 перезапущен',
    'panel-update': 'nfqws2-UI обновлен',
    'core-update': 'nfqws2 обновлен',
  }[event];
  return version?.trim() ? `${base} до ${version.trim()}` : base;
}

export function toastPresentation(tone: ToastTone) {
  return tone === 'error'
    ? { title: 'Ошибка', icon: '!', role: 'alert' as const }
    : { title: 'Готово', icon: '✓', role: 'status' as const };
}

export function canShowToast(message: string, tone: ToastTone, now: number, seen: Map<string, number>, windowMs = 1200): boolean {
  const key = `${tone}\u0000${message}`;
  const previous = seen.get(key);
  if (previous !== undefined && now - previous < windowMs) return false;
  seen.set(key, now);
  for (const [candidate, timestamp] of seen) if (now - timestamp > 10_000) seen.delete(candidate);
  return true;
}
