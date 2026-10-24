import { describe, expect, it } from 'vitest';
import { configAssignmentValue, configEditorLayout, configOutlineEntries } from './config-values';

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

  it('numbers every configuration line and keeps short editor blocks compact', () => {
    expect(configEditorLayout('--one\n--two\n--three\n--four', 3)).toEqual({
      lineCount: 4,
      visibleRows: 4,
      lineNumbers: '1\n2\n3\n4',
    });
  });

  it('shows at most ten configuration rows while numbering the full content', () => {
    const content = Array.from({ length: 14 }, (_, index) => `--arg-${index + 1}`).join('\n');

    expect(configEditorLayout(content, 3)).toEqual({
      lineCount: 14,
      visibleRows: 10,
      lineNumbers: Array.from({ length: 14 }, (_, index) => String(index + 1)).join('\n'),
    });
  });

  it('lists uppercase config assignments with their source line and text offset', () => {
    const content = '# header\r\nISP_INTERFACE="eth0"\r\n\r\nNFQWS_ARGS="--filter-tcp=443"\r\nmode_list=x';

    expect(configOutlineEntries(content)).toEqual([
      { name: 'ISP_INTERFACE', line: 2, offset: 10 },
      { name: 'NFQWS_ARGS', line: 4, offset: 34 },
    ]);
  });
});
