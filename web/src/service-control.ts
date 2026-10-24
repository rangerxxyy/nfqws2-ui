export type ServiceControlPresentation = {
  action: 'start' | 'stop';
  icon: 'play' | 'stop';
  label: string;
  tone: 'success' | 'danger' | 'neutral';
};

export function serviceControlPresentation(state: string | null | undefined): ServiceControlPresentation {
  if (state === 'running') return { action: 'stop', icon: 'stop', label: 'Остановить nfqws2', tone: 'danger' };
  if (state === 'stopped') return { action: 'start', icon: 'play', label: 'Запустить nfqws2', tone: 'success' };
  return { action: 'start', icon: 'play', label: 'Запустить nfqws2', tone: 'neutral' };
}
