import { describe, expect, it } from 'vitest';
import { canShowToast, lifecycleMessage } from './notifications';

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
