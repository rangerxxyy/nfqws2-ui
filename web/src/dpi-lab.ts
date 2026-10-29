export type DpiLabTlsField = { name: string; offset: number; length: number; value: string; hex: string; evidence: string };
export type DpiLabPacket = {
  number: number; timestampMicros: number; source: string; sourcePort: number | null; destination: string;
  destinationPort: number | null; protocol: string; tcpSequence: number | null; tcpAcknowledgement: number | null;
  tcpFlags: number | null; ttl: number | null; payloadOffset: number | null; payloadLen: number; frameHex: string; evidence: string;
};
export type DpiLabFlow = {
  source: string; sourcePort: number; destination: string; destinationPort: number; packetNumbers: number[];
  reassemblyPolicy: string; sequenceStart: number; dataHex: string; dataAscii: string; byteSources: Array<number | null>;
  gapBytes: number; overlapBytes: number; conflictingOverlapBytes: number; streamTruncated: boolean;
  tlsClientHello: { serverName: string | null; fields: DpiLabTlsField[] } | null;
  tlsParseError: string | null; evidence: string[];
};
export type DpiLabAnalysis = {
  capture: { name: string; sizeBytes: number; packetCount: number; linkType: number; timestampResolution: string };
  packets: DpiLabPacket[]; flows: DpiLabFlow[]; warnings: string[]; evidence: string[];
  summary: { findings: Array<{ id: string; title: string; state: 'confirmed' | 'not_confirmed' | 'error' | 'not_checked'; detail: string; packetNumbers: number[]; flowIndices: number[] }>; events: Array<{ kind: string; title: string; detail: string; packetNumbers: number[]; flowIndex: number | null; timestampMicros: number }>; eventsTruncated: boolean; limitations: string[] };
};
export type DpiLabSavedExperiment = { id: string; name: string; savedAt: number; sizeBytes: number; packetCount: number; flowCount: number };
export function dpiFindingStatus(state: string): { label: string; tone: 'positive' | 'caution' | 'negative' | 'neutral' } {
  switch (state) {
    case 'confirmed': return { label: 'Подтверждено', tone: 'positive' };
    case 'not_confirmed': return { label: 'Не подтверждено', tone: 'caution' };
    case 'error': return { label: 'Ошибка', tone: 'negative' };
    default: return { label: 'Не проверялось', tone: 'neutral' };
  }
}
export type DpiLabStrategyArgument = { name: string; value: string | null };
export type DpiLabStrategyStep = { index: number; profileIndex: number; raw: string; function: string; arguments: DpiLabStrategyArgument[]; installed: boolean; file: string | null; docs: string[]; declaredArguments: string[]; requiredArguments: string[]; unverifiedArguments: string[]; evidence: string; modeling: string };
export type DpiLabStrategyAnalysis = { stepCount: number; steps: DpiLabStrategyStep[]; profiles: Array<{ index: number; name: string | null; source: string | null; filters: string[]; controls: string[] }>; filters: string[]; otherOptions: string[]; warnings: string[]; luaExecuted: false };
export function dpiStrategyCorrelation(strategy: DpiLabStrategyAnalysis | null, capture: DpiLabAnalysis | null) {
  return {
    expected: strategy?.steps.length ? strategy.steps.map((step) => `${step.index}. ${step.function}: ${step.arguments.map((argument) => `${argument.name}${argument.value === null ? '' : `=${argument.value}`}`).join(', ') || 'без параметров'}`) : [],
    model: strategy?.steps.length ? 'Сетевые действия этих шагов пока не моделируются.' : 'Сначала разберите стратегию во вкладке «Визуализатор стратегии».',
    observed: capture ? `В захвате ${capture.capture.packetCount} пакетов. Привязка пакетов к стратегии не определена.` : 'Сначала проанализируйте PCAP.',
    confirmed: 'Выполнение Lua и соответствие профилю по одному PCAP не подтверждены.',
  };
}
export function dpiNextSteps(analysis: DpiLabAnalysis): string[] {
  const find = (id: string) => analysis.summary.findings.find((item) => item.id === id);
  const tips: string[] = [];
  if (find('tcp_connected')?.state !== 'confirmed') tips.push('Чтобы увидеть начало соединения, запустите захват до открытия сайта и повторите запрос.');
  if (find('reassembly')?.state === 'not_confirmed') {
    const reasons: string[] = [];
    if (analysis.flows.some((flow) => flow.gapBytes > 0)) reasons.push('пропуски данных');
    if (analysis.flows.some((flow) => flow.conflictingOverlapBytes > 0)) reasons.push('различающиеся перекрывающиеся байты');
    if (analysis.flows.some((flow) => flow.streamTruncated)) reasons.push('ограничение размера потока');
    tips.push(reasons.length ? `Откройте связанный TCP-поток: точность восстановления ограничивают ${reasons.join(', ')}.` : 'Откройте связанный TCP-поток и проверьте ограничения захвата и восстановления.');
  }
  if (find('client_hello')?.state !== 'confirmed') tips.push('Для поиска имени сайта нужен захват начала TLS-соединения с достаточными данными.');
  tips.push('Для проверки стратегии сравните отдельные испытания с ней и без неё; один PCAP не подтверждает выполнение Lua.');
  return tips;
}
export type DpiByteRow = { offset: string; hex: string; ascii: string };
export type DpiByteDump = { rows: DpiByteRow[]; hiddenBytes: number; error?: string };
export type DpiFlowByteRow = { offset: string; sequence: string; hex: string; ascii: string; sources: string };

export function dpiCaptureAvailability(size: number | null, analyzing: boolean): { disabled: boolean; reason: string } {
  if (analyzing) return { disabled: true, reason: 'Анализ PCAP выполняется…' };
  if (size === null) return { disabled: true, reason: 'Выберите файл classic PCAP.' };
  if (size === 0) return { disabled: true, reason: 'Выбранный файл пуст.' };
  if (size > 2 * 1024 * 1024) return { disabled: true, reason: 'Файл больше лимита 2 МиБ.' };
  return { disabled: false, reason: 'Проанализировать выбранный PCAP' };
}

export function dpiExperimentSaveAvailability(selected: boolean, analyzed: boolean, analyzing: boolean, saving: boolean): { disabled: boolean; reason: string } {
  if (saving) return { disabled: true, reason: 'Эксперимент сохраняется…' };
  if (analyzing) return { disabled: true, reason: 'Дождитесь завершения анализа PCAP.' };
  if (!selected) return { disabled: true, reason: 'Выберите PCAP и нажмите «Анализировать PCAP».' };
  if (!analyzed) return { disabled: true, reason: 'Сначала нажмите «Анализировать PCAP». Сохранение станет доступно после успешного анализа.' };
  return { disabled: false, reason: 'Анализ завершён — можно сохранить эксперимент.' };
}

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
