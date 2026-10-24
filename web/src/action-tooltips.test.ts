import { afterEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_ACTION_TOOLTIP_DELAY_MS, scheduleDelayedTooltip } from './action-tooltips';

describe('delayed action tooltip timer', () => {
  afterEach(() => vi.useRealTimers());

  it('waits three seconds and can be cancelled before it appears', () => {
    vi.useFakeTimers();
    const show = vi.fn();
    const cancel = scheduleDelayedTooltip(show);
    expect(DEFAULT_ACTION_TOOLTIP_DELAY_MS).toBe(3000);
    vi.advanceTimersByTime(2999);
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
});
