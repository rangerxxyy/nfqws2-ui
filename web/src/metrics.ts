export type UsageTone = 'neutral' | 'warning' | 'critical';

export function resourceUsagePercent(used: number | null, total: number | null): number | null {
  if (used === null || total === null || !Number.isFinite(used) || !Number.isFinite(total) || total <= 0) return null;
  return Math.min(100, Math.max(0, used * 100 / total));
}

export function usageTone(percent: number | null): UsageTone {
  if (percent === null || !Number.isFinite(percent)) return 'neutral';
  if (percent >= 90) return 'critical';
  if (percent >= 70) return 'warning';
  return 'neutral';
}
