import type { SiteDiagnosticReport, SiteDiagnosticStep } from './api';
import { comparisonVerdict, presentStep, quicExplanation, type Presentation } from './diagnosis-presenter';
export type DiagnosisRow = { id: string; label: string; kind: Presentation['kind']; detail: string; metrics: string; note?: string; inactive?: boolean };
function row(id: string, label: string, step?: SiteDiagnosticStep): DiagnosisRow {
  return {id,label,kind: step ? presentStep(step).kind : 'unknown',inactive:!step,detail:step?.detail || 'Этап не выполнялся.',
    metrics: step ? [step.httpCode === null ? '' : `HTTP ${step.httpCode}`, step.durationMs === null ? '' : `${step.durationMs} мс`].filter(Boolean).join(' · ') : ''};
}
export function diagnosisRows(report: SiteDiagnosticReport): DiagnosisRow[] {
  const step = (id: string) => report.steps.find(item => item.id === id);
  const dns = row('dns','Имя сайта · DNS',step('dns'));
  const doh = step('doh');
  if (doh) { dns.detail += ` ${doh.detail}`; if (doh.status !== 'ok' && dns.kind === 'success') dns.kind = 'unknown'; }
  const rows = [dns,row('tcp','Соединение · TCP 443',step('tcp')),row('direct','Запрос мимо nfqws2',report.direct),
    row('sni','Проверка имени · SNI',step('sni')),row('http','Открытый HTTP · порт 80',step('http')),row('current','Как открывается сейчас',report.current)];
  if (report.reverseDns) {
    const records = report.reverseDns;
    const namesFound = records.some((record) => record.names.length > 0);
    rows.splice(1, 0, {
      id: 'reverse-dns',
      label: 'Обратный DNS · PTR',
      kind: namesFound ? 'success' : 'unknown',
      detail: records.length
        ? records.map((record) => `${record.address}: ${record.names.length ? record.names.join(', ') : record.detail}${record.resolver ? ` · ${record.resolver}` : ''}`).join('\n')
        : 'Не выполнен: адреса сайта не получены.',
      metrics: '',
      note: 'PTR-имя показывает обратную DNS-запись IP-адреса; оно может отличаться от домена сайта.',
    });
  }
  if (report.quic) {
    const q = report.quic;
    const describe = (label: string, result: SiteDiagnosticStep) => `${label}: ${presentStep(result).text}${result.durationMs === null ? '' : ` · ${result.durationMs} мс`}. ${result.detail}`;
    rows.push({id:'quic',label:q.method === 'http3' ? 'QUIC · HTTP/3' : 'QUIC · UDP 443',kind:comparisonVerdict(q.direct,q.current).kind,
      detail:`${describe('Напрямую',q.direct)}\n${describe('Сейчас',q.current)}`,metrics:'',note:quicExplanation(q.method,q.http3Advertised)});
  } else rows.push(row('quic','QUIC · HTTP/3'));
  for (const [id,label] of [['route','Маршрут роутера'],['lists','Доменные списки и IP']]) if (step(id)) rows.push(row(id,label,step(id)));
  return rows;
}
