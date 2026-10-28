export type CommunityStrategyIssue = {
  outcome: 'works' | 'fails';
  asn: string;
  provider: string;
  proto: 'tls' | 'http';
  targetHost: string;
  targetAsn: number | null;
  strategy: string;
  result: string;
  nfqws2: string;
  ui: string;
  title: string;
};

export function buildCommunityStrategyIssueUrl(issue: CommunityStrategyIssue): string {
  const query = new URLSearchParams({
    template: 'strategy.yml', title: issue.title, outcome: issue.outcome,
    asn: issue.asn, provider: issue.provider, proto: issue.proto,
    target_host: issue.targetHost,
    target_asn: issue.targetAsn ? String(issue.targetAsn) : '',
    strategy: issue.strategy, result: issue.result, nfqws2: issue.nfqws2, ui: issue.ui,
  });
  return `https://github.com/zemidala/nfqws2-strategies/issues/new?${query.toString()}`;
}
