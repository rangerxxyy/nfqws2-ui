export type PanelRoute = 'conf' | 'lists' | 'check' | 'trace' | 'autotune' | 'configCheck' | 'monitor' | 'profiles' | 'community' | 'snapshots' | 'changes' | 'logs' | 'appearance' | 'about';

const ROUTE_GROUPS: Record<PanelRoute, string> = {
  conf: 'configuration',
  profiles: 'configuration',
  check: 'site-check',
  trace: 'site-check',
  configCheck: 'site-check',
  autotune: 'tuning',
  monitor: 'monitoring',
  community: 'site-check',
  lists: 'lists',
  snapshots: 'system',
  changes: 'system',
  logs: 'system',
  appearance: 'system',
  about: 'system',
};

export function navigationGroupForRoute(route: string | undefined): string | undefined {
  return route && route in ROUTE_GROUPS ? ROUTE_GROUPS[route as PanelRoute] : undefined;
}
