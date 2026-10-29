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

  it('keeps ordinary nfqws flag values in the v1.11.9 number and word palette', () => {
    const highlighted = highlightConfigLine('NFQWS_ARGS="--filter-tcp=80,443 --payload=tls_clienthello,http_req"');
    expect(highlighted).toContain(
      '<span class="conf-token-flag">--filter-tcp</span><span class="conf-token-operator">=</span><span class="conf-token-number">80</span><span class="conf-token-operator">,</span><span class="conf-token-number">443</span>',
    );
    expect(highlighted).toContain(
      '<span class="conf-token-flag">--payload</span><span class="conf-token-operator">=</span><span class="conf-token-word">tls_clienthello</span><span class="conf-token-operator">,</span><span class="conf-token-word">http_req</span>',
    );
  });

  it('escapes user-provided text before adding token markup', () => {
    const highlighted = highlightConfigLine('--hostlist-domains=<example.com>');
    expect(highlighted).toContain('&lt;');
    expect(highlighted).toContain('&gt;');
    expect(highlighted).not.toContain('<example.com>');
  });
});
