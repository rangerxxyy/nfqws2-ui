export type DpiLabTlsField = { name: string; offset: number; length: number; value: string; hex: string; evidence: string };
export type DpiLabPacket = {
  number: number; timestampMicros: number; source: string; sourcePort: number | null; destination: string;
  destinationPort: number | null; protocol: string; tcpSequence: number | null; tcpAcknowledgement: number | null;
  tcpFlags: number | null; ttl: number | null; payloadOffset: number | null; payloadLen: number; frameHex: string; evidence: string;
};
export type DpiLabFlow = {
  source: string; sourcePort: number; destination: string; destinationPort: number; packetNumbers: number[];
  reassemblyPolicy: string; sequenceStart: number; dataHex: string; dataAscii: string; byteSources: Array<number | null>;
  gapBytes: number; overlapBytes: number; streamTruncated: boolean;
  tlsClientHello: { serverName: string | null; fields: DpiLabTlsField[] } | null;
  tlsParseError: string | null; evidence: string[];
};
export type DpiLabAnalysis = {
  capture: { name: string; sizeBytes: number; packetCount: number; linkType: number; timestampResolution: string };
  packets: DpiLabPacket[]; flows: DpiLabFlow[]; warnings: string[]; evidence: string[];
};
export type DpiLabSavedExperiment = { id: string; name: string; savedAt: number; sizeBytes: number; packetCount: number; flowCount: number };
export type DpiByteRow = { offset: string; hex: string; ascii: string };
export type DpiByteDump = { rows: DpiByteRow[]; hiddenBytes: number; error?: string };
export type DpiFlowByteRow = { offset: string; sequence: string; hex: string; ascii: string; sources: string };

export function formatDpiFlowByteRows(dataHex: string, byteSources: Array<number | null>, sequenceStart: number, maxBytes = 4096): DpiFlowByteRow[] {
  const tokens = dataHex ? dataHex.split(' ') : [];
  if (!Number.isInteger(sequenceStart) || sequenceStart < 0 || sequenceStart > 0xffff_ffff || !Number.isSafeInteger(maxBytes) || maxBytes < 0 || tokens.length !== byteSources.length || tokens.some((token) => token !== '??' && !/^[0-9a-f]{2}$/i.test(token)) || byteSources.some((source) => source !== null && (!Number.isSafeInteger(source) || source < 1))) return [];
  const rows: DpiFlowByteRow[] = [];
  const shown = Math.min(tokens.length, maxBytes);
  for (let offset = 0; offset < shown; offset += 16) {
    const end = Math.min(shown, offset + 16);
    const bytes = tokens.slice(offset, end);
    const sources = [...new Set(byteSources.slice(offset, end).filter((source): source is number => source !== null))];
    rows.push({
      offset: offset.toString(16).padStart(8, '0'),
      sequence: ((sequenceStart + offset) >>> 0).toString(),
      hex: bytes.join(' '),
      ascii: bytes.map((token) => { const byte = token === '??' ? -1 : Number.parseInt(token, 16); return byte >= 32 && byte <= 126 ? String.fromCharCode(byte) : byte < 0 ? '?' : '.'; }).join(''),
      sources: sources.length ? sources.map((source) => `#${source}`).join(', ') : '—',
    });
  }
  return rows;
}

export function formatDpiByteDump(hex: string, maxBytes = 4096, bytesPerRow = 16): DpiByteDump {
  if (hex.length % 2 !== 0 || !/^[0-9a-f]*$/i.test(hex) || !Number.isSafeInteger(maxBytes) || maxBytes < 0 || !Number.isSafeInteger(bytesPerRow) || bytesPerRow < 1 || bytesPerRow > 64) {
    return { rows: [], hiddenBytes: 0, error: 'Некорректные шестнадцатеричные данные' };
  }
  const byteCount = hex.length / 2;
  const shown = Math.min(byteCount, maxBytes);
  const rows: DpiByteRow[] = [];
  for (let offset = 0; offset < shown; offset += bytesPerRow) {
    const lineEnd = Math.min(shown, offset + bytesPerRow);
    const bytes: number[] = [];
    for (let index = offset; index < lineEnd; index += 1) bytes.push(Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16));
    rows.push({
      offset: offset.toString(16).padStart(8, '0'),
      hex: bytes.map((byte) => byte.toString(16).padStart(2, '0')).join(' '),
      ascii: bytes.map((byte) => byte >= 32 && byte <= 126 ? String.fromCharCode(byte) : '.').join(''),
    });
  }
  return { rows, hiddenBytes: byteCount - shown };
}

export function dpiEvidencePresentation(source: string): { label: string; tone: 'observed' | 'derived' | 'simulated' | 'hypothesis' | 'unknown' } {
  switch (source) {
    case 'CAPTURE OBSERVED': return { label: 'Наблюдалось в PCAP', tone: 'observed' };
    case 'PROTOCOL DERIVED': return { label: 'По структуре протокола', tone: 'derived' };
    case 'SIMULATED': return { label: 'Реконструкция', tone: 'simulated' };
    case 'HYPOTHESIS': return { label: 'Гипотеза', tone: 'hypothesis' };
    default: return { label: 'Неизвестно', tone: 'unknown' };
  }
}

export function formatDpiTcpFlags(flags: number | null): string {
  if (flags === null) return '—';
  const names: Array<[number, string]> = [[0x100, 'NS'], [0x80, 'CWR'], [0x40, 'ECE'], [0x20, 'URG'], [0x10, 'ACK'], [0x08, 'PSH'], [0x04, 'RST'], [0x02, 'SYN'], [0x01, 'FIN']];
  const set = names.filter(([bit]) => flags & bit).map(([, name]) => name);
  return set.length ? set.join(' ') : 'нет';
}
