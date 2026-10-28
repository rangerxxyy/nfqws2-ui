import { describe, expect, it } from 'vitest';
import { formatDpiByteDump, dpiEvidencePresentation, formatDpiFlowByteRows, formatDpiTcpFlags } from './dpi-lab';

describe('DPI Lab byte inspector', () => {
  it('formats captured hex into offset, byte, and printable ASCII columns', () => {
    expect(formatDpiByteDump('4500002048690000', 8).rows).toEqual([
      { offset: '00000000', hex: '45 00 00 20 48 69 00 00', ascii: 'E.. Hi..' },
    ]);
  });

  it('stops at the display bound and reports hidden bytes', () => {
    const result = formatDpiByteDump('41424344', 2);
    expect(result.rows).toEqual([{ offset: '00000000', hex: '41 42', ascii: 'AB' }]);
    expect(result.hiddenBytes).toBe(2);
  });

  it('refuses malformed byte data instead of rendering it as trusted capture', () => {
    expect(formatDpiByteDump('00xz')).toEqual({ rows: [], hiddenBytes: 0, error: 'Некорректные шестнадцатеричные данные' });
  });
});

describe('DPI Lab evidence labels', () => {
  it('keeps measured packets, protocol interpretation, and unknowns distinct', () => {
    expect(dpiEvidencePresentation('CAPTURE OBSERVED').label).toBe('Наблюдалось в PCAP');
    expect(dpiEvidencePresentation('PROTOCOL DERIVED').label).toBe('По структуре протокола');
    expect(dpiEvidencePresentation('UNKNOWN').label).toBe('Неизвестно');
    expect(dpiEvidencePresentation('HYPOTHESIS').tone).not.toBe(dpiEvidencePresentation('CAPTURE OBSERVED').tone);
  });

  it('names observed TCP flags without inventing flags for non-TCP packets', () => {
    expect(formatDpiTcpFlags(0x12)).toBe('ACK SYN');
    expect(formatDpiTcpFlags(null)).toBe('—');
  });
});

describe('DPI Lab reconstructed stream provenance', () => {
  it('shows byte offsets, wrapping TCP sequence numbers, gaps, and first-seen packet sources', () => {
    expect(formatDpiFlowByteRows('61 62 ?? 64', [1, 1, null, 3], 0xfffffffe)).toEqual([
      { offset: '00000000', sequence: '4294967294', hex: '61 62 ?? 64', ascii: 'ab?d', sources: '#1, #3' },
    ]);
  });

  it('rejects invalid provenance vectors and bounds rendered stream rows', () => {
    expect(formatDpiFlowByteRows('61 62', [1], 1)).toEqual([]);
    expect(formatDpiFlowByteRows('61', [-1], 1)).toEqual([]);
    expect(formatDpiFlowByteRows('61', [1], 0x1_0000_0000)).toEqual([]);
    expect(formatDpiFlowByteRows('61 62', [1, 2], 1, 1)).toHaveLength(1);
  });
});
