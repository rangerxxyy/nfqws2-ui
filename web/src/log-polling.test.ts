import { describe, expect, it } from 'vitest';
import { shouldPollLogs } from './log-polling';

describe('log auto-refresh polling', () => {
  it('polls only while the logs view is visible, auto-refresh is enabled and no request is pending', () => {
    expect(shouldPollLogs(true, true, false)).toBe(true);
    expect(shouldPollLogs(false, true, false)).toBe(false);
    expect(shouldPollLogs(true, false, false)).toBe(false);
    expect(shouldPollLogs(true, true, true)).toBe(false);
  });
});
