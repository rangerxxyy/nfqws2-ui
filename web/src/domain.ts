export type ServiceState = 'running' | 'stopped' | 'error' | 'unknown';
export type ServiceAction = 'start' | 'stop' | 'restart';

export type ConfigEditorState = { draft: string; editing: boolean };
export type ConfigEditorAction =
  | { type: 'loaded'; content: string }
  | { type: 'edit' }
  | { type: 'change'; value: string }
  | { type: 'cancel'; content: string }
  | { type: 'saved'; content: string }
  | { type: 'reset' };

export function configEditorReducer(state: ConfigEditorState, action: ConfigEditorAction): ConfigEditorState {
  switch (action.type) {
    case 'loaded': return { draft: action.content, editing: false };
    case 'edit': return { ...state, editing: true };
    case 'change': return state.editing ? { ...state, draft: action.value } : state;
    case 'cancel': return { draft: action.content, editing: false };
    case 'saved': return { draft: action.content, editing: false };
    case 'reset': return { draft: '', editing: false };
  }
}

export function normalizeStatus(value: unknown): ServiceState {
  return value === 'running' || value === 'stopped' || value === 'error' ? value : 'unknown';
}

export function formatBytes(bytes: number | null): string {
  if (bytes === null || !Number.isFinite(bytes) || bytes < 0) return '—';
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} КБ`;
  const mb = bytes / (1024 * 1024);
  return `${Number.isInteger(mb) ? mb : mb.toFixed(1)} МБ`;
}

export function isMutatingActionAllowed(value: string): value is ServiceAction {
  return value === 'start' || value === 'stop' || value === 'restart';
}

export function actionToApplyConfig(state: ServiceState): ServiceAction | null {
  if (state === 'running') return 'restart';
  if (state === 'stopped') return 'start';
  return null;
}

export function percent(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 && value <= 100 ? value : null;
}
