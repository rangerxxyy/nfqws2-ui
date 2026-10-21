export type Locale = 'ru' | 'en' | 'zh' | 'tr';
export type StatusResponse = {
  service: { state: string; detail: string; checkedAt: number | null; version: string | null };
  system: { memoryTotalBytes: number | null; memoryUsedBytes: number | null; cpuPercent: number | null };
  panel: { version: string; port: number };
};
export type SessionResponse = { authenticated: boolean; csrfToken?: string };
export type ConfigResponse = { file: string; content: string; revision: string };

export class ApiError extends Error {
  constructor(readonly status: number, message: string) { super(message); this.name = 'ApiError'; }
}

let csrfToken = '';
export function setCsrfToken(value: string): void { csrfToken = value; }

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.body && !headers.has('Content-Type')) headers.set('Content-Type', 'application/json');
  if (['POST', 'PUT', 'DELETE', 'PATCH'].includes((init.method ?? 'GET').toUpperCase()) && csrfToken) {
    headers.set('X-CSRF-Token', csrfToken);
  }
  const response = await fetch(path, { ...init, headers, credentials: 'same-origin' });
  const payload = await response.json().catch(() => ({})) as { error?: string };
  if (!response.ok) throw new ApiError(response.status, payload.error || `HTTP ${response.status}`);
  return payload as T;
}
