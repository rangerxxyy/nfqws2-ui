import { describe, expect, it } from 'vitest';
import { isPanelRouteAvailable, navigationGroupForRoute } from './navigation';

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

  it('keeps DPI Lab unavailable until it is ready to be reopened', () => {
    expect(isPanelRouteAvailable('dpiLab')).toBe(false);
    expect(navigationGroupForRoute('dpiLab')).toBeUndefined();
    expect(isPanelRouteAvailable('check')).toBe(true);
  });
});
