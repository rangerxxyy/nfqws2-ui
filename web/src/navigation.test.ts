import { describe, expect, it } from 'vitest';
import { navigationGroupForRoute } from './navigation';

describe('navigationGroupForRoute', () => {
  it('places the strategy constructor under Configuration', () => {
    expect(navigationGroupForRoute('profiles')).toBe('configuration');
    expect(navigationGroupForRoute('conf')).toBe('configuration');
  });

  it('keeps system tools under System', () => {
    expect(navigationGroupForRoute('snapshots')).toBe('system');
    expect(navigationGroupForRoute('packageLeftovers')).toBe('system');
  });
});
