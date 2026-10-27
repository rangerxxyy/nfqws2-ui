export type CommunityProtocol = 'tls' | 'http';

export type CommunityProtocolGroup<T> = {
  proto: CommunityProtocol;
  title: string;
  items: T[];
};

export function shouldAutoDetectCommunityProvider(asn: number | null): boolean {
  return asn === null;
}

export function communityProviderLabel(asn: number | null, provider: string): string {
  if (asn === null) return 'Провайдер не выбран';
  return `AS${asn}${provider.trim() ? ` ${provider.trim()}` : ''}`;
}

export function visibleCommunityProtocols<T extends { proto: CommunityProtocol }>(items: T[]): CommunityProtocolGroup<T>[] {
  return ([
    { proto: 'tls', title: 'HTTPS / TLS' },
    { proto: 'http', title: 'HTTP' },
  ] as const).map(({ proto, title }) => ({
    proto,
    title,
    items: items.filter((item) => item.proto === proto),
  })).filter((group) => group.items.length > 0);
}
