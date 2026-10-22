import { describe, expect, it } from 'vitest';
import { resourceUsagePercent, usageTone } from './metrics';

describe('resource metrics presentation', () => {
  it('calculates a bounded percentage and keeps missing totals unknown', () => {
    expect(resourceUsagePercent(750, 1000)).toBe(75);
    expect(resourceUsagePercent(1200, 1000)).toBe(100);
    expect(resourceUsagePercent(10, 0)).toBeNull();
    expect(resourceUsagePercent(null, 1000)).toBeNull();
  });

  it('uses neutral, warning, and critical thresholds like XKeen UI', () => {
    expect(usageTone(69.9)).toBe('neutral');
    expect(usageTone(70)).toBe('warning');
    expect(usageTone(89.9)).toBe('warning');
    expect(usageTone(90)).toBe('critical');
    expect(usageTone(null)).toBe('neutral');
  });
});
