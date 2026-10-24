import { afterEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_ACTION_TOOLTIP_DELAY_MS, fallbackTooltipText, scheduleDelayedTooltip } from './action-tooltips';

describe('delayed action tooltip timer', () => {
  afterEach(() => vi.useRealTimers());

  it('waits two seconds and can be cancelled before it appears', () => {
    vi.useFakeTimers();
    const show = vi.fn();
    const cancel = scheduleDelayedTooltip(show);
    expect(DEFAULT_ACTION_TOOLTIP_DELAY_MS).toBe(2000);
    vi.advanceTimersByTime(1999);
    expect(show).not.toHaveBeenCalled();
    cancel();
    vi.advanceTimersByTime(1);
    expect(show).not.toHaveBeenCalled();
  });

  it('shows the tooltip once the full delay has elapsed', () => {
    vi.useFakeTimers();
    const show = vi.fn();
    scheduleDelayedTooltip(show);
    vi.advanceTimersByTime(3000);
    expect(show).toHaveBeenCalledOnce();
  });

  it('describes navigation and action controls even without a native title', () => {
    const nav = { dataset: { actionClick: 'switchMainTab', args: '["monitor"]' }, getAttribute: () => null, textContent: '' } as unknown as HTMLButtonElement;
    expect(fallbackTooltipText(nav)).toContain('ручную проверку');
    const action = { dataset: { actionClick: 'runMonitoring' }, getAttribute: () => null, textContent: '' } as unknown as HTMLButtonElement;
    expect(fallbackTooltipText(action)).toContain('Проверить доступность');
  });
});
