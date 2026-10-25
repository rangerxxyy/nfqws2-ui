const TCP_TS_ONLY_EXCEPTIONS = /^(?:tcp_md5|badsum|tcp_seq=|tcp_ack=|ip_ttl=|ip6_ttl=|ip_autottl=|ip6_autottl=|tcp_flags_(?:set|unset)=|ip6_(?:hopbyhop|destopt|routing|ah)|fool=)/;

function isTcpTimestampOnlyStep(step: string): boolean {
  const params = step.replace(/^--lua-desync=/, '').split(':');
  if (!params.some((param) => param.startsWith('tcp_ts='))) return false;
  return !params.some((param) => TCP_TS_ONLY_EXCEPTIONS.test(param));
}

/** True when the strategy includes a step whose only fooling method is TCP timestamps. */
export function strategyNeedsWindowsTcpTimestamps(steps: string[]): boolean {
  return steps.some(isTcpTimestampOnlyStep);
}
