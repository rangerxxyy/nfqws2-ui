import { describe, expect, it } from 'vitest';
import { configAssignmentValue, configDraftDisposition, configEditorLayout, configEditorViewportHeight, configOutlineEntries, sameConfigText } from './config-values';

describe('sameConfigText', () => {
  it('treats CRLF and LF line endings as equivalent', () => {
    expect(sameConfigText('A=1\r\nB=2\r\n', 'A=1\nB=2\n')).toBe(true);
  });

  it('keeps actual text edits distinct', () => {
    expect(sameConfigText('A=1\r\n', 'A=2\n')).toBe(false);
  });
});

describe('configDraftDisposition', () => {
  it('treats a draft matching the latest configuration as unchanged after a concurrent refresh', () => {
    expect(configDraftDisposition('A=1\n', 'A=1\n', 'A=1\r\n', 'rev-1', 'rev-2')).toBe('unchanged');
  });

  it('rejects a draft based on a stale configuration revision', () => {
    expect(configDraftDisposition('A=2\n', 'A=1\n', 'A=3\n', 'rev-1', 'rev-2')).toBe('stale');
  });

  it('allows a genuine edit against the same configuration revision', () => {
    expect(configDraftDisposition('A=2\n', 'A=1\n', 'A=1\n', 'rev-1', 'rev-1')).toBe('changed');
  });
});

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

  it('caps the shared viewport height to the rendered row count', () => {
    expect(configEditorViewportHeight(3)).toBe('calc(3 * 1.25rem + 1rem)');
    expect(configEditorViewportHeight(10)).toBe('calc(10 * 1.25rem + 1rem)');
  });

  it('keeps empty and trailing-newline rows explicit without padding short values', () => {
    expect(configEditorLayout('', 3).lineCount).toBe(1);
    expect(configEditorLayout('--one\n', 3)).toMatchObject({ lineCount: 2, visibleRows: 3, lineNumbers: '1\n2' });
  });

  it('lists uppercase config assignments with their source line and text offset', () => {
    const content = '# header\r\nISP_INTERFACE="eth0"\r\n\r\nNFQWS_ARGS="--filter-tcp=443"\r\nmode_list=x';

    expect(configOutlineEntries(content)).toEqual([
      { name: 'ISP_INTERFACE', line: 2, offset: 10 },
      { name: 'NFQWS_ARGS', line: 4, offset: 34 },
    ]);
  });
});
