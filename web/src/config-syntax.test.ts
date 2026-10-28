import { describe, expect, it } from 'vitest';
import { highlightConfigLine } from './config-syntax';

describe('configuration syntax highlighting', () => {
  it('uses distinct key and value colors for Lua parameters in grouped and full config lines', () => {
    expect(highlightConfigLine('--lua-desync=circular:fails=2:time=300')).toBe(
      '<span class="conf-token-flag">--lua-desync</span><span class="conf-token-operator">=</span><span class="conf-token-mode">circular</span><span class="conf-token-operator">:</span><span class="conf-token-key">fails</span><span class="conf-token-operator">=</span><span class="conf-token-value">2</span><span class="conf-token-operator">:</span><span class="conf-token-key">time</span><span class="conf-token-operator">=</span><span class="conf-token-value">300</span>',
    );
  });

  it('uses the same Lua highlighting when the argument is wrapped in a config assignment', () => {
    const plain = highlightConfigLine('--lua-desync=circular:fails=2:time=300');
    const assignment = highlightConfigLine('NFQWS_ARGS="--lua-desync=circular:fails=2:time=300"');

    expect(assignment).toContain(plain);
  });

  it('highlights values of ordinary nfqws flags, including comma-separated numbers', () => {
    expect(highlightConfigLine('NFQWS_ARGS="--filter-tcp=80,443 --payload=tls_clienthello,http_req"')).toContain(
      '<span class="conf-token-flag">--filter-tcp</span><span class="conf-token-operator">=</span><span class="conf-token-value">80</span><span class="conf-token-operator">,</span><span class="conf-token-value">443</span>',
    );
    expect(highlightConfigLine('NFQWS_ARGS="--filter-tcp=80,443 --payload=tls_clienthello,http_req"')).toContain(
      '<span class="conf-token-flag">--payload</span><span class="conf-token-operator">=</span><span class="conf-token-value">tls_clienthello</span><span class="conf-token-operator">,</span><span class="conf-token-value">http_req</span>',
    );
  });

  it('escapes user-provided text before adding token markup', () => {
    const highlighted = highlightConfigLine('--hostlist-domains=<example.com>');
    expect(highlighted).toContain('&lt;');
    expect(highlighted).toContain('&gt;');
    expect(highlighted).not.toContain('<example.com>');
  });
});
