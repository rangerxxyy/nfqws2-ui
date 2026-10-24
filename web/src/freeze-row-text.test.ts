import { describe, expect, it } from 'vitest';
import { freezeRowIdentity, freezeRowDetails } from './freeze-row-text';

describe('freeze row text', () => {
  it('does not repeat the shared SNI for pinned IP network probes', () => {
    expect(freezeRowIdentity({ label: 'Hetzner', host: 'example.com', path: '/', targetKind: 'pinned_address' }))
      .toBe('Hetzner');
  });

  it('shows the actual host and path for host-based probes without an SNI label', () => {
    expect(freezeRowIdentity({ label: 'OVH', host: 'proof.ovh.net', path: '/files/1Mb.dat', targetKind: 'host_reference' }))
      .toBe('OVH\nproof.ovh.net/files/1Mb.dat');
  });

  it('keeps strategy parameters and past attempts behind compact detail text', () => {
    expect(freezeRowDetails({
      foundStrategy: { sni: 'ya.ru', step: '--lua-desync=hostfakesplit repeats=4', successes: 3, attempts: 3 },
      triedCandidates: 4,
      candidateAttempts: [{ sni: 'vk.com', successes: 1, attempts: 3, reason: 'тайм-аут' }],
    })).toEqual({ summary: 'SNI ya.ru · 3/3', details: 'Стратегия: --lua-desync=hostfakesplit repeats=4\nПроверено имён: 4\nДругие попытки: vk.com · 1/3 · тайм-аут' });
  });
});
