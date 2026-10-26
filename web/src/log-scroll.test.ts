import { describe, expect, it } from 'vitest';
import { shouldFollowLogTail } from './log-scroll';

describe('shouldFollowLogTail', () => {
  it('starts at the end when the journal has not been rendered yet', () => {
    expect(shouldFollowLogTail(false, 0, 0, 0)).toBe(true);
  });

  it('follows new lines when the reader was already near the end', () => {
    expect(shouldFollowLogTail(true, 760, 240, 1000)).toBe(true);
  });

  it('keeps the reader position when they scrolled up to inspect older lines', () => {
    expect(shouldFollowLogTail(true, 300, 240, 1000)).toBe(false);
  });
});
