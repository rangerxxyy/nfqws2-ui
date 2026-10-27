export type ToastTone = 'success' | 'error';
export type LifecycleEvent = 'start' | 'stop' | 'restart' | 'panel-update' | 'core-update';
export const TOAST_DEDUPLICATION_WINDOW_MS = 5500;
export const CORE_UPDATE_CHECK_INTERVAL_MS = 30 * 60 * 1000;

export function shouldCheckCoreUpdate(now: number, lastCheckedAt: number): boolean {
  return now - lastCheckedAt >= CORE_UPDATE_CHECK_INTERVAL_MS;
}

export function coreUpdateNoticePresentation(
  status: { latestTag: string | null; installedTag: string | null },
  dismissedTag: string | null,
) {
  const latestTag = status.latestTag;
  const normalize = (tag: string | null) => tag?.trim().replace(/^v/i, '').split('-')[0] ?? null;
  const updateAvailable = Boolean(latestTag && status.installedTag && normalize(latestTag) !== normalize(status.installedTag));
  const version = updateAvailable ? latestTag : null;
  return { highlight: version !== null, notify: version !== null && version !== dismissedTag, version };
}

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
