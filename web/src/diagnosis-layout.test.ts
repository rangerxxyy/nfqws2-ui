import { expect, it } from 'vitest';
import type { SiteDiagnosticReport, SiteDiagnosticStep } from './api';
import { diagnosisRows } from './diagnosis-layout';
const step = (id: string, status: SiteDiagnosticStep['status'] = 'ok'): SiteDiagnosticStep => ({id,status,detail:`${id} detail`,httpCode:null,durationMs:null});
it('places real direct/current results in the diagnostic sequence without inventing missing measurements', () => {
  const report: SiteDiagnosticReport = {host:'example.com',addresses:[],steps:[step('dns'),step('tcp')],direct:{...step('direct'),httpCode:206,durationMs:91},current:step('current','error'),quic:null,summary:'test',limitations:[]};
  const rows = diagnosisRows(report);
  expect(rows.map(row => row.id)).toEqual(['dns','tcp','direct','sni','http','current','quic']);
  expect(rows.find(row => row.id === 'direct')).toMatchObject({kind:'success',metrics:'HTTP 206 · 91 мс'});
  expect(rows.find(row => row.id === 'current')?.kind).toBe('failure');
  expect(rows.find(row => row.id === 'sni')?.kind).toBe('unknown');
  expect(rows.find(row => row.id === 'quic')?.kind).toBe('unknown');
});
it('keeps QUIC transport and advertised HTTP3 separate and includes both paths', () => {
  const report: SiteDiagnosticReport = {host:'example.com',addresses:[],steps:[step('dns'),step('doh','warning')],direct:step('direct'),current:step('current'),quic:{direct:step('direct'),current:step('current','error'),method:'quic_initial',http3Advertised:true},summary:'',limitations:[]};
  const rows = diagnosisRows(report);
  expect(rows.find(row => row.id === 'dns')?.kind).toBe('unknown');
  expect(rows.find(row => row.id === 'quic')?.detail).toContain('Напрямую:');
  expect(rows.find(row => row.id === 'quic')?.detail).toContain('Сейчас:');
  expect(rows.find(row => row.id === 'quic')?.note).toContain('полный запрос HTTP/3 не выполнялся');
});
