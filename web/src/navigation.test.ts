import { describe, expect, it } from 'vitest';
import { navigationGroupForRoute } from './navigation';

describe('navigationGroupForRoute', () => {
  it('places the strategy constructor under Configuration', () => {
    expect(navigationGroupForRoute('profiles')).toBe('configuration');
    expect(navigationGroupForRoute('conf')).toBe('configuration');
  });

  it('keeps system tools under System', () => {
    expect(navigationGroupForRoute('snapshots')).toBe('system');
    expect(navigationGroupForRoute('packageLeftovers')).toBeUndefined();
    expect(navigationGroupForRoute('supportReport')).toBeUndefined();
    expect(navigationGroupForRoute('diagnostic16k')).toBeUndefined();
  });

  it('places DPI Lab under diagnostics in both navigation layouts', () => {
    expect(navigationGroupForRoute('dpiLab')).toBe('site-check');
  });
});
