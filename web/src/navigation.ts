export type PanelRoute = 'conf' | 'lists' | 'check' | 'trace' | 'autotune' | 'configCheck' | 'dpiLab' | 'monitor' | 'profiles' | 'community' | 'snapshots' | 'changes' | 'logs' | 'appearance' | 'about';

type AvailablePanelRoute = Exclude<PanelRoute, 'dpiLab'>;

const ROUTE_GROUPS: Record<AvailablePanelRoute, string> = {
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

export function isPanelRouteAvailable(route: string | undefined): route is AvailablePanelRoute {
  return !!route && route in ROUTE_GROUPS;
}

export function navigationGroupForRoute(route: string | undefined): string | undefined {
  return isPanelRouteAvailable(route) ? ROUTE_GROUPS[route] : undefined;
}
