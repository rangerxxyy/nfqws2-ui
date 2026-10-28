import { describe, expect, it } from 'vitest';
import { freezeRowIdentity, freezeRowDetails } from './freeze-row-text';

describe('freeze row text', () => {
  it('does not repeat the shared SNI for pinned IP network probes', () => {
    expect(freezeRowIdentity({ label: 'Hetzner', host: 'example.com', path: '/', targetKind: 'pinned_address' }))
      .toBe('Hetzner');
  });

  it('shows only the hostname for host-based probes without an SNI label', () => {
    expect(freezeRowIdentity({ label: 'OVH', host: 'proof.ovh.net', path: '/files/1Mb.dat', targetKind: 'host_reference' }))
      .toBe('OVH\nproof.ovh.net');
    expect(freezeRowIdentity({ label: 'Amazon CloudFront', host: 'ip-ranges.amazonaws.com', path: '/ip-ranges.json', targetKind: 'host_reference' }))
      .toBe('Amazon CloudFront\nip-ranges.amazonaws.com');
  });

  it('keeps strategy parameters and past attempts behind compact detail text', () => {
    expect(freezeRowDetails({
      foundStrategy: { sni: 'ya.ru', step: '--lua-desync=hostfakesplit repeats=4', successes: 3, attempts: 3 },
      triedCandidates: 4,
      candidateTotal: 24,
      candidateAttempts: [{ sni: 'vk.com', successes: 1, attempts: 3, reason: 'тайм-аут' }],
    })).toEqual({ summary: 'SNI ya.ru · 3/3', details: 'Стратегий проверено: 4 из 24\nПодтверждение варианта: 3/3\nПараметры: --lua-desync=hostfakesplit repeats=4\nДругие варианты: vk.com · 1/3 · тайм-аут' });
  });

  it('shows progress against the full catalog before the first strategy result', () => {
    expect(freezeRowDetails({ foundStrategy: null, triedCandidates: 0, candidateTotal: 24, candidateAttempts: [] }))
      .toEqual({ summary: 'Стратегий проверено: 0 из 24', details: 'Стратегий проверено: 0 из 24' });
  });
});
