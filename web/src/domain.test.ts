import { describe, expect, it } from 'vitest';
import { formatBytes, isMutatingActionAllowed, normalizeStatus } from './domain';

describe('UI domain formatting', () => {
  it('keeps unknown status unknown instead of painting it green', () => {
    expect(normalizeStatus('unknown')).toBe('unknown');
    expect(normalizeStatus('running')).toBe('running');
    expect(normalizeStatus('unexpected')).toBe('unknown');
  });

  it('formats bytes without confusing total with available memory', () => {
    expect(formatBytes(512 * 1024 * 1024)).toBe('512 МБ');
    expect(formatBytes(null)).toBe('—');
  });

  it('allows only the service action enum', () => {
    expect(isMutatingActionAllowed('restart')).toBe(true);
    expect(isMutatingActionAllowed('reload; reboot')).toBe(false);
  });
});
