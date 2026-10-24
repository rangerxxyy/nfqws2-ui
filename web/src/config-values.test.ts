import { describe, expect, it } from 'vitest';
import { configAssignmentValue } from './config-values';

describe('configAssignmentValue', () => {
  it('reads a full multiline quoted assignment without its shell quotes', () => {
    const config = [
      'NFQWS_ARGS="--filter-tcp=443',
      '--filter-l7=tls',
      '# Strategy 1',
      '--lua-desync=fake:blob=tls_clienthello:strategy=1',
      '--lua-desync=multisplit:pos=1:strategy=1" # inline note',
      'NFQWS_ARGS_QUIC="--filter-udp=443"',
    ].join('\n');

    expect(configAssignmentValue(config, 'NFQWS_ARGS')).toBe([
      '--filter-tcp=443',
      '--filter-l7=tls',
      '# Strategy 1',
      '--lua-desync=fake:blob=tls_clienthello:strategy=1',
      '--lua-desync=multisplit:pos=1:strategy=1',
    ].join('\n'));
  });

  it('preserves shell variable references in an assignment', () => {
    expect(configAssignmentValue('NFQWS_EXTRA_ARGS="$MODE_LIST"\n', 'NFQWS_EXTRA_ARGS')).toBe('$MODE_LIST');
  });
});
