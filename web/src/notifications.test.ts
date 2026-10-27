import { describe, expect, it } from 'vitest';
import { canShowToast, coreUpdateNoticePresentation, CORE_UPDATE_CHECK_INTERVAL_MS, lifecycleMessage, shouldCheckCoreUpdate, toastCardsToDismiss } from './notifications';

describe('toast stack overflow', () => {
  it('removes only the oldest active toast cards and respects the visible-card limit', () => {
    const cards = [{ id: 'oldest', removing: false }, { id: 'progress', removing: false }, { id: 'newer', removing: false }, { id: 'fading', removing: true }];
    expect(toastCardsToDismiss(cards, 3)).toEqual([{ id: 'oldest', removing: false }]);
    expect(toastCardsToDismiss(cards.slice(1), 3)).toEqual([]);
  });
});

describe('lifecycle notifications', () => {
  it('uses clear names for service and updater events', () => {
    expect(lifecycleMessage('stop')).toBe('nfqws2 остановлен');
    expect(lifecycleMessage('restart')).toBe('nfqws2 перезапущен');
    expect(lifecycleMessage('panel-update', 'v0.6.0')).toBe('nfqws2-UI обновлен до v0.6.0');
    expect(lifecycleMessage('core-update', 'v2.0.0')).toBe('nfqws2 обновлен до v2.0.0');
  });

  it('suppresses the same toast briefly without suppressing a different event', () => {
    const seen = new Map<string, number>();
    expect(canShowToast('nfqws2 остановлен', 'success', 1000, seen, 1200)).toBe(true);
    expect(canShowToast('nfqws2 остановлен', 'success', 1200, seen, 1200)).toBe(false);
    expect(canShowToast('nfqws2 перезапущен', 'success', 1200, seen, 1200)).toBe(true);
    expect(canShowToast('nfqws2 остановлен', 'success', 2500, seen, 1200)).toBe(true);
  });

  it('keeps an identical event suppressed for the full visible toast lifetime', () => {
    const seen = new Map<string, number>();
    expect(canShowToast('nfqws2 остановлен', 'success', 1000, seen)).toBe(true);
    expect(canShowToast('nfqws2 остановлен', 'success', 6300, seen)).toBe(false);
    expect(canShowToast('nfqws2 остановлен', 'success', 6500, seen)).toBe(true);
  });
});

describe('nfqws2 core update notice', () => {
  it('throttles background release checks to avoid repeated router requests on tab focus', () => {
    expect(shouldCheckCoreUpdate(10_000, 0)).toBe(false);
    expect(shouldCheckCoreUpdate(CORE_UPDATE_CHECK_INTERVAL_MS, 0)).toBe(true);
    expect(shouldCheckCoreUpdate(CORE_UPDATE_CHECK_INTERVAL_MS + 1, 1)).toBe(true);
  });

  it('highlights and notifies when the newest release differs from installed core', () => {
    expect(coreUpdateNoticePresentation({ latestTag: 'v1.3.2', installedTag: '1.3.1', updateAvailable: true }, null))
      .toEqual({ highlight: true, notify: true, version: 'v1.3.2' });
  });

  it('does not show an update when the newest release is already installed', () => {
    expect(coreUpdateNoticePresentation({ latestTag: 'v1.3.2', installedTag: '1.3.2-r1', updateAvailable: false }, null))
      .toEqual({ highlight: false, notify: false, version: null });
  });

  it('does not override a negative update decision from the backend', () => {
    expect(coreUpdateNoticePresentation({ latestTag: 'v1.3.3', installedTag: '1.3.2', updateAvailable: false }, null))
      .toEqual({ highlight: false, notify: false, version: null });
  });

  it('respects the backend update decision for package revision tags', () => {
    expect(coreUpdateNoticePresentation({ latestTag: 'v1.3.2-r2', installedTag: '1.3.2-r1', updateAvailable: true }, null))
      .toEqual({ highlight: true, notify: true, version: 'v1.3.2-r2' });
  });

  it('allows a manually checked update notice to be shown again after dismissal', () => {
    expect(coreUpdateNoticePresentation({ latestTag: 'v1.3.2', installedTag: '1.3.1', updateAvailable: true }, null).notify).toBe(true);
    expect(coreUpdateNoticePresentation({ latestTag: 'v1.3.2', installedTag: '1.3.1', updateAvailable: true }, 'v1.3.2').notify).toBe(false);
  });
});
