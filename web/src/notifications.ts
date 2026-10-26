export type ToastTone = 'success' | 'error';
export type LifecycleEvent = 'start' | 'stop' | 'restart' | 'panel-update' | 'core-update';
export const TOAST_DEDUPLICATION_WINDOW_MS = 5500;

export function toastCardsToDismiss<T extends { removing: boolean }>(cards: readonly T[], maxVisible = 4): T[] {
  const activeCards = cards.filter((card) => !card.removing);
  const overflowCount = Math.max(0, activeCards.length - Math.max(0, maxVisible) + 1);
  return activeCards.slice(0, overflowCount);
}

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

export function canShowToast(message: string, tone: ToastTone, now: number, seen: Map<string, number>, windowMs = TOAST_DEDUPLICATION_WINDOW_MS): boolean {
  const key = `${tone}\u0000${message}`;
  const previous = seen.get(key);
  if (previous !== undefined && now - previous < windowMs) return false;
  seen.set(key, now);
  for (const [candidate, timestamp] of seen) if (now - timestamp > 10_000) seen.delete(candidate);
  return true;
}
