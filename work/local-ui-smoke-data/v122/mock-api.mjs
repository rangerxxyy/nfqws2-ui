import http from 'node:http';

const profile = (id, index, name, source, args, editable = true, components) => ({
  id, index, name, source, editable, args,
  filters: args.filter((arg) => arg.startsWith('--filter-')),
  protocols: args.filter((arg) => arg.startsWith('--filter-l7=')),
  payloads: args.filter((arg) => arg.startsWith('--payload=')),
  lists: args.filter((arg) => arg.includes('hostlist') || arg.includes('ipset')),
  steps: args.filter((arg) => arg.startsWith('--lua-desync=')), components,
});

const strategy = ['--filter-tcp=443', '--filter-l7=tls', '--payload=tls_client_hello', '--lua-desync=hostfakesplit:host=example.com:midhost=midsld:strategy=1'];
const udp = ['--filter-udp=50000-50099', '--lua-desync=fake'];
const quic = ['--filter-udp=443', '--payload=quic_initial', '--lua-desync=fake'];
const tcp = ['--filter-tcp=80,443', '--filter-l7=tls', '--payload=tls_client_hello', '--lua-desync=fake'];
const ipset = ['--ipset=/opt/etc/nfqws2/lists/ipset.list'];
const sites = ['--hostlist=/opt/etc/nfqws2/lists/user.list'];
const profiles = [
  profile('custom-1', 1, 'Свой профиль 1', 'NFQWS_ARGS_CUSTOM', strategy),
  profile('managed-2', 2, 'UDP: игры, звонки', 'NFQWS_ARGS_UDP', udp, false, [{ source: 'NFQWS_ARGS_UDP', args: udp, editable: true, note: null }]),
  profile('managed-3', 3, 'QUIC по IP', 'NFQWS_ARGS_QUIC_IPSET', [...quic, ...ipset, '--ipset-ip=0.0.0.0'], false, [
    { source: 'NFQWS_ARGS_QUIC', args: quic, editable: true, note: null },
    { source: 'NFQWS_ARGS_IPSET', args: ipset, editable: true, note: null },
  ]),
  profile('managed-4', 4, 'QUIC по сайтам', 'NFQWS_ARGS_QUIC_EXTRA', [...quic, ...sites], false, [
    { source: 'NFQWS_ARGS_QUIC', args: quic, editable: true, note: null },
    { source: 'NFQWS_EXTRA_ARGS', args: sites, editable: true, note: null },
  ]),
  profile('managed-5', 5, 'HTTP/HTTPS по IP', 'NFQWS_ARGS_IPSET', [...tcp, ...ipset, '--ipset-ip=0.0.0.0'], false, [
    { source: 'NFQWS_ARGS', args: tcp, editable: true, note: null },
    { source: 'NFQWS_ARGS_IPSET', args: ipset, editable: true, note: null },
  ]),
  profile('managed-6', 6, 'HTTP/HTTPS по сайтам', 'NFQWS_ARGS', [...tcp, ...sites], false, [
    { source: 'NFQWS_ARGS', args: tcp, editable: true, note: null },
    { source: 'NFQWS_EXTRA_ARGS', args: sites, editable: true, note: null },
  ]),
];

const routes = {
  '/api/auth/status': { enabled: false, hasPassword: false, authenticated: true, rememberDays: 30 },
  '/api/status': { service: { state: 'running', detail: 'mock', checkedAt: Date.now(), version: 'test' }, system: { memoryTotalBytes: 268435456, memoryUsedBytes: 100663296, cpuPercent: 4 }, panel: { version: '1.2.2', port: 1002 } },
  '/api/config': { file: '/mock/nfqws2.conf', content: 'ISP_INTERFACE="eth0"\nNFQWS_ARGS="--filter-tcp=80,443 --filter-l7=tls --payload=tls_client_hello --lua-desync=fake"\nNFQWS_ARGS_UDP="--filter-udp=50000-50099 --lua-desync=fake"\nNFQWS_ARGS_CUSTOM="--filter-tcp=443 --filter-l7=tls --payload=tls_client_hello --lua-desync=hostfakesplit:host=example.com:strategy=1"\n', revision: 'mock-revision' },
  '/api/nfqws2/installation': { config: { writable: true }, networkInterfaces: ['eth0'] },
  '/api/profiles': { profiles, customArgs: strategy.join(' '), revision: 'mock-revision', limitation: null },
  '/api/profiles/catalog': { functions: [{ name: 'hostfakesplit', file: 'zapret-antidpi.lua', docs: [], arguments: ['host', 'midhost', 'strategy', 'seqovl', 'seqovl_pattern'], fileArguments: [], standardGroups: [], required: ['host'] }, { name: 'fake', file: 'zapret-antidpi.lua', docs: [], arguments: ['repeats', 'strategy'], fileArguments: [], standardGroups: [], required: [] }], standardGroups: [], cli: { full: true, options: [], l7: ['tls', 'http', 'quic'], payloads: ['tls_client_hello'], }, },
  '/api/nfqws2/lists': { files: [] },
  '/api/ui/appearance': null,
};

http.createServer((req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET,POST,PUT,OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
  if (req.method === 'OPTIONS') { res.writeHead(204).end(); return; }
  const value = routes[new URL(req.url, 'http://localhost').pathname] ?? {};
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(value));
}).listen(1002, '127.0.0.1', () => console.log('Local UI smoke API listening on 127.0.0.1:1002'));
