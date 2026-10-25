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
  const ipTarget = report.host.includes(':') || /^\d{1,3}(?:\.\d{1,3}){3}$/.test(report.host);
  if (ipTarget) {
    const records = report.reverseDns?.filter((record) => record.address === report.host) ?? report.reverseDns ?? [];
    const names = [...new Set(records.flatMap((record) => record.names))];
    dns.label = 'DNS-имя сайта';
    dns.kind = names.length ? 'success' : 'unknown';
    dns.detail = names.length
      ? `Для IP ${report.host} найдено имя: ${names.join(', ')}${records.find((record) => record.resolver)?.resolver ? ` · ${records.find((record) => record.resolver)?.resolver}` : ''}`
      : records[0]?.detail ?? `Для IP ${report.host} обратное DNS-имя не найдено.`;
  } else if (report.addresses.length) {
    dns.label = 'DNS-имя сайта';
    dns.detail = `DNS-имя сайта: ${report.host} · IP-адрес: ${report.addresses.join(', ')}`;
  }
  const doh = step('doh');
  if (doh && !ipTarget) { dns.detail += ` ${doh.detail}`; if (doh.status !== 'ok' && dns.kind === 'success') dns.kind = 'unknown'; }
  const rows = [dns,row('tcp','Соединение · TCP 443',step('tcp')),row('direct','Запрос мимо nfqws2',report.direct),
    row('sni','Проверка имени · SNI',step('sni')),row('http','Открытый HTTP · порт 80',step('http')),row('current','Как открывается сейчас',report.current)];
  if (report.quic) {
    const q = report.quic;
    const describe = (label: string, result: SiteDiagnosticStep) => `${label}: ${presentStep(result).text}${result.httpCode === null ? '' : ` · HTTP ${result.httpCode}`}${result.durationMs === null ? '' : ` · ${result.durationMs} мс`}`;
    rows.push({id:'quic',label:q.method === 'http3' ? 'QUIC · HTTP/3' : 'QUIC · UDP 443',kind:comparisonVerdict(q.direct,q.current).kind,
      detail:`${describe('Напрямую',q.direct)}\n${describe('Сейчас',q.current)}`,metrics:'',note:quicExplanation(q.method,q.http3Advertised)});
  } else rows.push(row('quic','QUIC · HTTP/3'));
  for (const [id,label] of [['route','Маршрут роутера'],['lists','Доменные списки и IP']]) if (step(id)) rows.push(row(id,label,step(id)));
  return rows;
}
