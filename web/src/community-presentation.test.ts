import { describe, expect, it } from 'vitest';
import { communityProviderLabel, shouldAutoDetectCommunityProvider, visibleCommunityProtocols } from './community-presentation';

describe('community strategies presentation', () => {
  it('auto-detects a provider only when no ASN is configured', () => {
    expect(shouldAutoDetectCommunityProvider(null)).toBe(true);
    expect(shouldAutoDetectCommunityProvider(35807)).toBe(false);
  });

  it('omits empty HTTP and TLS groups from the provider strategy list', () => {
    expect(visibleCommunityProtocols([
      { id: 'tls-1', proto: 'tls' },
    ])).toEqual([{ proto: 'tls', title: 'HTTPS / TLS', items: [{ id: 'tls-1', proto: 'tls' }] }]);
    expect(visibleCommunityProtocols([])).toEqual([]);
  });

  it('shows the ASN and provider name together like the community reference', () => {
    expect(communityProviderLabel(35807, 'AS-SKYNET-SPB SkyNet Ltd.')).toBe('AS35807 AS-SKYNET-SPB SkyNet Ltd.');
    expect(communityProviderLabel(null, '')).toBe('Провайдер не выбран');
  });
});
