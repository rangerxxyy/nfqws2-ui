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
  const quic = rows.find(row => row.id === 'quic');
  expect(quic?.detail).toContain('Напрямую: Ответ получен');
  expect(quic?.detail).toContain('Сейчас: Ответ не получен');
  expect(quic?.detail).not.toContain('direct detail');
  expect(quic?.note).toBe('QUIC отвечает; HTTP/3 заявлен.');
});
it('shows resolved site IP addresses in the DNS row without an extra reverse DNS row', () => {
  const report: SiteDiagnosticReport = {host:'example.com',addresses:['203.0.113.9','203.0.113.10'],reverseDns:[
    {address:'203.0.113.9',names:['edge.example.net'],resolver:'dig',detail:'Найдено обратным DNS (dig)'},
  ],steps:[step('dns')],direct:step('direct'),current:step('current'),quic:null,summary:'',limitations:[]};
  const rows = diagnosisRows(report);
  expect(rows.map(row => row.id)).not.toContain('reverse-dns');
  expect(rows.find(row => row.id === 'dns')).toMatchObject({label:'DNS-имя сайта'});
  expect(rows.find(row => row.id === 'dns')?.detail).toContain('IP-адрес: 203.0.113.9, 203.0.113.10');
});
it('marks SNI as intentionally skipped when the direct HTTPS request already responds', () => {
  const report: SiteDiagnosticReport = {host:'example.com',addresses:['203.0.113.9'],steps:[step('dns'),step('tcp')],direct:{...step('direct'),httpCode:200},current:step('current'),quic:null,summary:'',limitations:[]};
  const row = diagnosisRows(report).find(item => item.id === 'sni');
  expect(row).toMatchObject({inactive:true,detail:'Не нужно: сайт отвечает напрямую.'});
});
it('shows PTR domain in the DNS site-name row when the target is an IP address', () => {
  const report: SiteDiagnosticReport = {host:'142.251.1.190',addresses:['142.251.1.190'],reverseDns:[
    {address:'142.251.1.190',names:['lga34s32-in-f14.1e100.net'],resolver:'nslookup',detail:'Найдено обратным DNS (nslookup)'},
  ],steps:[step('dns','unavailable')],direct:step('direct'),current:step('current'),quic:null,summary:'',limitations:[]};
  const rows = diagnosisRows(report);
  expect(rows.map(row => row.id)).not.toContain('reverse-dns');
  expect(rows.find(row => row.id === 'dns')).toMatchObject({kind:'success',label:'DNS-имя сайта'});
  expect(rows.find(row => row.id === 'dns')?.detail).toContain('142.251.1.190 найдено имя: lga34s32-in-f14.1e100.net');
});
it('makes the DNS row clear when an IP has no reverse DNS name', () => {
  const report: SiteDiagnosticReport = {host:'203.0.113.9',addresses:['203.0.113.9'],reverseDns:[
    {address:'203.0.113.9',names:[],resolver:'nslookup',detail:'PTR-запись не найдена (nslookup)'},
  ],steps:[step('dns','unavailable')],direct:step('direct'),current:step('current'),quic:null,summary:'',limitations:[]};
  expect(diagnosisRows(report).find(row => row.id === 'dns')?.detail).toContain('PTR-запись не найдена');
});
