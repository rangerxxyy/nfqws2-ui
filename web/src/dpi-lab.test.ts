import { describe, expect, it } from 'vitest';
import { dpiCaptureAvailability, dpiExperimentSaveAvailability, dpiFindingStatus, dpiStrategyCorrelation, dpiNextSteps, formatDpiByteDump, dpiEvidencePresentation, formatDpiFlowByteRows, formatDpiTcpFlags, type DpiLabAnalysis, type DpiLabStrategyAnalysis } from './dpi-lab';

describe('DPI Lab finding status labels', () => {
  it('gives each independent diagnostic state a distinct plain-language label and tone', () => {
    expect(['confirmed', 'not_confirmed', 'error', 'not_checked'].map((state) => dpiFindingStatus(state).label)).toEqual([
      'Подтверждено', 'Не подтверждено', 'Ошибка', 'Не проверялось',
    ]);
    expect(dpiFindingStatus('not_confirmed').tone).not.toBe(dpiFindingStatus('confirmed').tone);
    expect(dpiFindingStatus('error').tone).not.toBe(dpiFindingStatus('not_checked').tone);
  });
});

describe('DPI Lab next steps', () => {
  it('suggests a new capture for incomplete evidence and flags uncertain reconstruction', () => {
    const analysis = { flows: [{ gapBytes: 3, conflictingOverlapBytes: 0, streamTruncated: false }], summary: { findings: [{ id: 'tcp_connected', state: 'not_confirmed' }, { id: 'reassembly', state: 'not_confirmed' }, { id: 'client_hello', state: 'not_confirmed' }] } } as DpiLabAnalysis;
    const tips = dpiNextSteps(analysis);
    expect(tips).toHaveLength(4);
    expect(tips.join(' ')).toContain('пропуски');
    expect(tips.join(' ')).toContain('не подтверждает выполнение Lua');
  });
  it('explains stream truncation without inventing gaps or conflicting bytes', () => {
    const analysis = { flows: [{ gapBytes: 0, conflictingOverlapBytes: 0, streamTruncated: true }], summary: { findings: [{ id: 'tcp_connected', state: 'confirmed' }, { id: 'reassembly', state: 'not_confirmed' }, { id: 'client_hello', state: 'confirmed' }] } } as DpiLabAnalysis;
    const tips = dpiNextSteps(analysis).join(' ');
    expect(tips).toContain('ограничение размера потока');
    expect(tips).not.toContain('пропуски данных');
    expect(tips).not.toContain('различающиеся перекрывающиеся байты');
  });
  it('does not recommend recapturing a connection whose beginning is observed', () => {
    const analysis = { summary: { findings: [{ id: 'tcp_connected', state: 'confirmed' }, { id: 'reassembly', state: 'confirmed' }, { id: 'client_hello', state: 'confirmed' }] } } as DpiLabAnalysis;
    expect(dpiNextSteps(analysis)).toHaveLength(1);
  });
});

describe('strategy correlation', () => {
  it('keeps the configured steps distinct from packet observations and unknown execution', () => {
    const strategy = { steps: [{ index: 1, function: 'fake', arguments: [{ name: 'repeats', value: '2' }] }] } as DpiLabStrategyAnalysis;
    const capture = { capture: { packetCount: 12 } } as DpiLabAnalysis;
    const result = dpiStrategyCorrelation(strategy, capture);
    expect(result.expected).toEqual(['1. fake: repeats=2']);
    expect(result.model).toContain('не моделируются');
    expect(result.observed).toContain('12 пакетов');
    expect(result.observed).toContain('не определена');
    expect(result.confirmed).toContain('не подтверждены');
  });
  it('describes missing inputs without inferring failure or success', () => {
    const result = dpiStrategyCorrelation(null, null);
    expect(result.expected).toEqual([]);
    expect(result.model).toContain('Сначала');
    expect(result.observed).toContain('Сначала');
  });
});

describe('DPI Lab capture availability', () => {
  it('enables a selected 276 KiB capture and restores it after analysis', () => {
    expect(dpiCaptureAvailability(null, false).disabled).toBe(true);
    expect(dpiCaptureAvailability(276 * 1024, false).disabled).toBe(false);
    expect(dpiCaptureAvailability(276 * 1024, true).disabled).toBe(true);
    expect(dpiCaptureAvailability(276 * 1024, false).disabled).toBe(false);
  });
  it('explains empty and oversized captures before upload', () => {
    expect(dpiCaptureAvailability(0, false).reason).toContain('пуст');
    expect(dpiCaptureAvailability(2 * 1024 * 1024, false).disabled).toBe(false);
    expect(dpiCaptureAvailability(2 * 1024 * 1024 + 1, false).reason).toContain('2 МиБ');
  });
  it('requires successful analysis of the selected file before saving and explains each busy state', () => {
    expect(dpiExperimentSaveAvailability(false, false, false, false).disabled).toBe(true);
    expect(dpiExperimentSaveAvailability(true, false, false, false).reason).toContain('Сначала нажмите');
    expect(dpiExperimentSaveAvailability(true, true, false, false).disabled).toBe(false);
    expect(dpiExperimentSaveAvailability(true, true, true, false).disabled).toBe(true);
    expect(dpiExperimentSaveAvailability(true, true, false, true).reason).toContain('сохраняется');
    expect(dpiExperimentSaveAvailability(true, false, false, false).disabled).toBe(true);
  });
});

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
