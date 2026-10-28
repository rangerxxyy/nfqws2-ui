import { describe, expect, it } from 'vitest';
import { buildCommunityStrategyIssueUrl } from './community-share';

describe('community strategy sharing', () => {
  it('includes the detected target ASN and verified host in the public issue URL', () => {
    const url = new URL(buildCommunityStrategyIssueUrl({
      outcome: 'works', asn: '12345', provider: 'Example ISP', proto: 'tls',
      targetHost: 'example.org', targetAsn: 13335, strategy: '--lua-desync=fake',
      result: '3/3', nfqws2: '1.9.7', ui: '1.9.7', title: 'Confirmed strategy',
    }));

    expect(url.searchParams.get('target_host')).toBe('example.org');
    expect(url.searchParams.get('target_asn')).toBe('13335');
    expect(url.searchParams.get('strategy')).toBe('--lua-desync=fake');
  });

  it('keeps target ASN empty when lookup has no public network result', () => {
    const url = new URL(buildCommunityStrategyIssueUrl({
      outcome: 'works', asn: '12345', provider: '', proto: 'tls', targetHost: 'example.org',
      targetAsn: null, strategy: '--lua-desync=fake', result: '3/3', nfqws2: '', ui: '', title: 'Confirmed strategy',
    }));
    expect(url.searchParams.get('target_asn')).toBe('');
  });
});
