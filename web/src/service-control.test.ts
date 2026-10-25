import { describe, expect, it } from 'vitest';
import { serviceControlPresentation } from './service-control';

describe('service control presentation', () => {
  it('offers stop while nfqws2 is running and start while it is stopped', () => {
    expect(serviceControlPresentation('running')).toEqual({ action: 'stop', icon: 'stop', label: 'Остановить nfqws2', tone: 'danger' });
    expect(serviceControlPresentation('stopped')).toEqual({ action: 'start', icon: 'play', label: 'Запустить nfqws2', tone: 'success' });
  });

  it('uses a neutral start presentation when the service state is unknown', () => {
    expect(serviceControlPresentation('unknown')).toEqual({ action: 'start', icon: 'play', label: 'Запустить nfqws2', tone: 'neutral' });
  });
});
