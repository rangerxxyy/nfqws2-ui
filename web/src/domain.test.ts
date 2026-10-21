import { describe, expect, it } from 'vitest';
import { actionToApplyConfig, formatBytes, isMutatingActionAllowed, normalizeStatus } from './domain';

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

  it('applies a saved config with the safe action for the confirmed service state', () => {
    expect(actionToApplyConfig('running')).toBe('restart');
    expect(actionToApplyConfig('stopped')).toBe('start');
    expect(actionToApplyConfig('unknown')).toBeNull();
    expect(actionToApplyConfig('error')).toBeNull();
  });
});
