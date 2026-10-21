import '@fontsource-variable/inter';
import '@fontsource-variable/jetbrains-mono';
import './panel.css';
import { api, ApiError, setCsrfToken, type ConfigResponse, type SessionResponse, type StatusResponse } from './api';

type Route = 'conf' | 'lists' | 'check' | 'autotune' | 'monitor' | 'community' | 'snapshots' | 'logs' | 'auth';
let authenticated = false;
let csrf = '';
let status: StatusResponse | null = null;
let config: ConfigResponse | null = null;
let busy = false;
let currentRoute: Route = 'conf';
const $ = <T extends HTMLElement>(selector: string) => document.querySelector<T>(selector);
const all = <T extends HTMLElement>(selector: string) => [...document.querySelectorAll<T>(selector)];
const setText = (selector: string, value: string) => { const node = $(selector); if (node) node.textContent = value; };
const setWidth = (selector: string, value: number | null) => { const node = $(selector); if (node) node.style.width = `${Math.min(100, Math.max(0, value ?? 0))}%`; };
function toast(message: string, type: 'success' | 'error' = 'success') {
  const host = $('#toastWrapper'); if (!host) return;
  const item = document.createElement('div');
  item.className = `pointer-events-auto px-4 py-3 rounded-xl border shadow-xl text-sm ${type === 'error' ? 'bg-rose-950 border-rose-700 text-rose-200' : 'bg-slate-900 border-slate-700 text-slate-100'}`;
  item.textContent = message; host.append(item); window.setTimeout(() => item.remove(), 4500);
}
function revealAuthRequired() { toast('Войдите через раздел «Авторизация», чтобы открыть защищённые данные.', 'error'); }
function setBusy(value: boolean) {
  busy = value;
  all<HTMLButtonElement>('button[data-protected]').forEach((button) => { button.disabled = value || !authenticated; });
  all<HTMLButtonElement>('button[data-action="restartService"], button[data-action="toggleDaemonPower"], button[data-action="applyConfigAndRestart"], button[data-action="copyConfigText"]')
    .forEach((button) => { button.disabled = value || !authenticated; });
  all<HTMLSelectElement>('select[data-action="updateConfVariable"]').forEach((select) => { select.disabled = value || !authenticated; });
}
function updateAuthView() {
  $('#authSignedOut')?.classList.toggle('hidden', authenticated);
  $('#authSignedIn')?.classList.toggle('hidden', !authenticated);
  setText('#authMenuBadge', authenticated ? 'Вход выполнен' : 'Войти');
  setText('#statusTextLabel', authenticated ? 'Статус неизвестен' : 'Требуется вход');
  setText('#headerUiVersion', status?.panel.version ?? '—');
  if (!authenticated) { setText('#queueBadge', 'Q:—'); setText('#snapshotCountBadge', '—'); }
  all<HTMLButtonElement>('button[data-protected]').forEach((button) => { button.disabled = !authenticated || busy; });
  const protectedControls = all<HTMLButtonElement>('button[data-action="restartService"], button[data-action="toggleDaemonPower"], button[data-action="applyConfigAndRestart"], button[data-action="copyConfigText"]');
  protectedControls.forEach((button) => {
    button.disabled = !authenticated || busy;
    button.classList.toggle('opacity-50', !authenticated || busy);
    button.classList.toggle('cursor-not-allowed', !authenticated || busy);
  });
  all<HTMLSelectElement>('select[data-action="updateConfVariable"]').forEach((select) => {
    select.disabled = !authenticated;
    if (!authenticated) select.value = '';
  });
}
function showRoute(route: Route) {
  currentRoute = route;
  document.querySelectorAll<HTMLElement>('[id^="tabContent-"]').forEach((section) => section.classList.toggle('hidden', section.id !== `tabContent-${route}`));
  all<HTMLButtonElement>('.side-nav-btn').forEach((button) => {
    const active = button.id === `navBtn-${route}`;
    button.classList.toggle('active', active);
    button.classList.toggle('text-slate-400', !active);
  });
  if (route === 'auth') updateAuthView();
  if (authenticated && route === 'logs') void loadLogs();
}
async function fetchSession() {
  try {
    const session = await api<SessionResponse>('/api/session');
    authenticated = session.authenticated;
    csrf = session.csrfToken ?? '';
    setCsrfToken(csrf);
    updateAuthView();
    if (authenticated) { await fetchStatus(); await loadConfig(); }
  } catch (error) { authenticated = false; updateAuthView(); setText('#authError', error instanceof Error ? error.message : 'Не удалось проверить сессию'); }
}
async function fetchStatus() {
  if (!authenticated) return;
  try {
    status = await api<StatusResponse>('/api/status');
    const service = status.service.state;
    const running = service === 'running';
    const stopped = service === 'stopped';
    const label = running ? 'nfqws2 запущен' : stopped ? 'nfqws2 остановлен' : service === 'error' ? 'Ошибка nfqws2' : 'Статус неизвестен';
    const tone = running ? 'emerald' : stopped || service === 'error' ? 'rose' : 'slate';
    const text = $('#statusTextLabel'); if (text) { text.textContent = label; text.className = `text-xs sm:text-sm font-semibold text-${tone}-400 tracking-tight`; }
    const dot = $('#statusMainDot'); if (dot) dot.className = `status-dot-active relative inline-flex rounded-full h-2.5 w-2.5 bg-${tone}-400`;
    const ring = $('#statusRadarRing'); if (ring) ring.className = `status-radar absolute inline-flex h-full w-full rounded-full bg-${running ? 'emerald' : 'slate'}-400`;
    const monitorBadge = $('#sideMonitorBadge'); if (monitorBadge) monitorBadge.className = `w-2 h-2 rounded-full bg-${running ? 'emerald' : 'slate'}-400`;
    const used = status.system.memoryUsedBytes, total = status.system.memoryTotalBytes;
    setText('#ramMetricText', used !== null && total !== null ? `${fmtMb(used)} / ${fmtMb(total)} MB` : '—');
    setWidth('#ramProgressBar', used !== null && total ? used * 100 / total : null);
    const cpu = status.system.cpuPercent;
    setText('#cpuMetricText', cpu === null ? '—' : `${Math.round(cpu)}%`); setWidth('#cpuProgressBar', cpu);
    setText('#headerCoreVersion', status.service.version ?? '—'); setText('#headerUiVersion', status.panel.version);
    const power = $('#togglePowerText'); if (power) power.textContent = running ? 'Остановить' : 'Запустить';
    const powerBtn = $('#togglePowerBtn'); if (powerBtn) { powerBtn.classList.toggle('text-rose-400', running); powerBtn.classList.toggle('text-emerald-400', stopped); }
    setText('#authStatusText', 'Доступ подтверждён');
    setText('#checkedAt', status.service.checkedAt ? new Date(status.service.checkedAt * 1000).toLocaleTimeString() : '—');
  } catch (error) {
    status = null;
    if (error instanceof ApiError && error.status === 401) {
      authenticated = false; setCsrfToken(''); updateAuthView();
      setText('#statusTextLabel', 'Требуется вход');
      return;
    }
    const label = $('#statusTextLabel'); if (label) { label.textContent = 'Статус неизвестен'; label.className = 'text-xs sm:text-sm font-semibold text-slate-400 tracking-tight'; }
    const dot = $('#statusMainDot'); if (dot) dot.className = 'relative inline-flex rounded-full h-2.5 w-2.5 bg-slate-400';
    const ring = $('#statusRadarRing'); if (ring) ring.className = 'absolute inline-flex h-full w-full rounded-full bg-slate-400';
    setText('#ramMetricText', '—'); setWidth('#ramProgressBar', null);
    setText('#cpuMetricText', '—'); setWidth('#cpuProgressBar', null);
    setText('#headerCoreVersion', '—'); setText('#headerUiVersion', '—');
    const detail = $('#statusDetail'); if (detail) detail.textContent = error instanceof Error ? error.message : 'Ошибка проверки статуса';
  }
}
function fmtMb(bytes: number) { return (bytes / 1024 / 1024).toFixed(0); }
async function loadConfig() {
  if (!authenticated) return revealAuthRequired();
  try {
    config = await api<ConfigResponse>('/api/config?file=nfqws2.conf');
    const editor = $('#nfqwsConfTextarea');
    if (editor instanceof HTMLTextAreaElement) editor.value = config.content;
    setText('#confLinesBadge', `${config.content.split(/\r?\n/).length} строк`);
    setText('#currentConfigPath', config.file);
    const moduleRefs = config.content.split(/\r?\n/).filter((line) => /^\s*[^#].*--(?:lua-init|blob)=/.test(line));
    setText('#moduleStatusBadge', `${moduleRefs.length} ссылок в конфигурации`);
    renderConfigModules(config.content);
    updateQuickParameters(config.content);
    const queue = configAssignmentValue(config.content, 'NFQUEUE_NUM');
    setText('#queueBadge', queue ? `Q:${queue.replace(/^['"]|['"]$/g, '')}` : 'Q:—');
    syncConfigLines();
  } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось прочитать конфигурацию', 'error'); }
}
function renderConfigModules(content: string) {
  const host = $('#moduleFilesContainer'); if (!host) return;
  host.replaceChildren();
  const args = content.split(/\r?\n/).filter((line) => !line.trimStart().startsWith('#')).flatMap((line) => line.match(/--(?:lua-init=@|blob=[^\s:@]+:@)[^\s'"\\]+/g) ?? []);
  if (!args.length) { const empty = document.createElement('span'); empty.className = 'text-slate-500'; empty.textContent = 'Ссылки на Lua/blob не найдены'; host.append(empty); return; }
  for (const argument of [...new Set(args)]) {
    const path = argument.slice(argument.indexOf('@') + 1);
    const button = document.createElement('button');
    button.className = 'px-2 py-1 rounded-lg bg-slate-900 hover:bg-slate-800 text-emerald-300 border border-slate-800';
    button.textContent = path.split('/').pop() || path;
    button.title = `${path} · наличие файла не проверено`;
    button.addEventListener('click', () => void navigator.clipboard.writeText(argument).then(() => toast('Аргумент скопирован')).catch(() => toast('Буфер обмена недоступен', 'error')));
    host.append(button);
  }
}
function configAssignmentValue(content: string, key: string): string {
  const line = content.split(/\r?\n/).find((item) => item.trimStart().startsWith(`${key}=`));
  if (!line) return '';
  const raw = line.slice(line.indexOf('=') + 1).trim();
  return raw.length >= 2 && ((raw.startsWith('"') && raw.endsWith('"')) || (raw.startsWith("'") && raw.endsWith("'"))) ? raw.slice(1, -1) : raw;
}
function updateQuickParameters(content: string) {
  all<HTMLSelectElement>('select[data-action="updateConfVariable"][data-key]').forEach((select) => {
    const key = select.dataset.key;
    if (!key) return;
    const value = configAssignmentValue(content, key);
    select.value = [...select.options].some((option) => option.value === value) ? value : '';
  });
}
function updateConfigVariable(key: string, value: string, quoted: boolean) {
  if (!authenticated) return revealAuthRequired();
  const editor = $('#nfqwsConfTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return;
  const escaped = quoted ? `"${value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"` : value;
  const lines = editor.value.split(/\r?\n/);
  const index = lines.findIndex((line) => line.trimStart().startsWith(`${key}=`));
  if (index >= 0) lines[index] = `${key}=${escaped}`;
  else lines.push(`${key}=${escaped}`);
  editor.value = lines.join('\n');
  syncConfigLines();
  toast('Черновик изменён. Нажмите «Применить конфигурацию», чтобы сохранить.');
}
function syncConfigLines() {
  const editor = $('#nfqwsConfTextarea'); const gutter = $('#confLineNumbers');
  if (!(editor instanceof HTMLTextAreaElement) || !gutter) return;
  const count = Math.max(1, editor.value.split('\n').length);
  gutter.innerHTML = Array.from({ length: count }, (_, i) => `${i + 1}`).join('<br>');
  gutter.scrollTop = editor.scrollTop;
  setText('#confLinesBadge', `${count} строк`);
}
async function saveConfigAndApply() {
  if (busy) return;
  if (!authenticated || !config) return revealAuthRequired();
  const editor = $('#nfqwsConfTextarea'); if (!(editor instanceof HTMLTextAreaElement)) return;
  setBusy(true);
  try {
    config = await api<ConfigResponse>('/api/config?file=nfqws2.conf', { method: 'PUT', body: JSON.stringify({ content: editor.value, revision: config.revision }) });
    toast('Конфигурация сохранена. Перезапускаю nfqws2…');
    await serviceAction('restart', true);
  } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось сохранить конфигурацию', 'error'); }
  finally { setBusy(false); }
}
async function serviceAction(action: 'start' | 'stop' | 'restart', confirmed = false) {
  if (busy) return;
  if (!authenticated) return revealAuthRequired();
  if (!confirmed && action === 'stop' && !window.confirm('Остановить nfqws2?')) return;
  if (!confirmed && action === 'restart' && !window.confirm('Перезапустить nfqws2?')) return;
  setBusy(true);
  try { await api('/api/service', { method: 'POST', body: JSON.stringify({ action }) }); await fetchStatus(); toast('Команда выполнена'); }
  catch (error) { toast(error instanceof Error ? error.message : 'Команда не выполнена', 'error'); }
  finally { setBusy(false); }
}
async function loadLogs() {
  if (!authenticated) return revealAuthRequired();
  try { const result = await api<{ content: string }>('/api/logs?source=nfqws2&tail=500'); const host = $('#liveLogContainer'); if (host) host.textContent = result.content; }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось загрузить журнал', 'error'); }
}
async function loadBackups() {
  if (!authenticated) return revealAuthRequired();
  try {
    const response = await api<{ backups: { id: string; createdAt: number; file: string }[] }>('/api/backups');
    setText('#snapshotCountBadge', String(response.backups.length));
    const box = $('#snapshotsListBox'); if (!box) return;
    box.replaceChildren();
    if (!response.backups.length) { box.textContent = 'Резервных копий пока нет'; return; }
    for (const backup of response.backups) { const row = document.createElement('div'); row.className = 'p-3 rounded-lg bg-slate-900 border border-slate-800 flex justify-between gap-3'; row.textContent = `${backup.file} · ${new Date(backup.createdAt * 1000).toLocaleString()}`; box.append(row); }
  } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось загрузить копии', 'error'); }
}
async function login(event: SubmitEvent) {
  event.preventDefault(); const input = $('#authPassword'); const err = $('#authError');
  if (!(input instanceof HTMLInputElement)) return;
  const button = $('#authSubmit'); if (button instanceof HTMLButtonElement) button.disabled = true;
  try {
    const session = await api<SessionResponse>('/api/login', { method: 'POST', body: JSON.stringify({ password: input.value }) });
    authenticated = true; csrf = session.csrfToken ?? ''; setCsrfToken(csrf); input.value = '';
    if (err) err.classList.add('hidden'); updateAuthView(); showRoute(currentRoute === 'auth' ? 'conf' : currentRoute);
    await fetchStatus(); await loadConfig(); toast('Вход выполнен');
  } catch (error) { if (err) { err.textContent = error instanceof Error ? error.message : 'Войти не удалось'; err.classList.remove('hidden'); } }
  finally { if (button instanceof HTMLButtonElement) button.disabled = false; }
}
async function logout() {
  try { await api('/api/logout', { method: 'POST', body: '{}' }); } catch { /* session may have expired */ }
  authenticated = false; csrf = ''; setCsrfToken(''); status = null; config = null; updateAuthView(); showRoute('auth'); toast('Вы вышли из панели');
}
export const DISABLED_PANEL_ACTIONS = ['runSiteDiagnosis','startStrategyAutoScan','applyStrategyToConf','sendTestTelegram','applyCommunityPreset','fetchAsnPrefixes','createNewSnapshot','restoreSnapshot','pushBackupToNas','exportFullBackup','installSelectedRelease','selectVersionItem','switchUpdateTarget','setDiagPreset','excludeBrokenDomain','injectPresetDomains','deduplicateCurrentList','saveActiveListFile','findTelegramChatId'] as const;

function bindPrototypeActions() {
  const showDiff = () => {
    if (!authenticated || !config) return revealAuthRequired();
    const editor = $('#nfqwsConfTextarea'); const body = $('#diffModalBody'); const modal = $('#diffModalBackdrop');
    if (!(editor instanceof HTMLTextAreaElement) || !body || !modal) return;
    const before = config.content.split(/\r?\n/); const after = editor.value.split(/\r?\n/);
    body.replaceChildren();
    const count = Math.max(before.length, after.length);
    for (let i = 0; i < count; i += 1) {
      if (before[i] === after[i]) continue;
      const row = document.createElement('div');
      row.className = 'px-2 py-1 rounded bg-amber-500/10 text-amber-200 whitespace-pre-wrap break-all';
      row.textContent = `${i + 1}: ${before[i] ?? '(нет строки)'}  →  ${after[i] ?? '(удалено)'}`;
      body.append(row);
    }
    if (!body.childElementCount) { const row = document.createElement('div'); row.className = 'text-slate-400'; row.textContent = 'Изменений нет'; body.append(row); }
    modal.classList.remove('hidden'); modal.classList.add('flex');
  };
  const handlers: Record<string, (...args: any[]) => unknown> = {
    switchMainTab: (route: Route) => { showRoute(route); if (!authenticated && route !== 'auth') revealAuthRequired(); if (authenticated && route === 'snapshots') void loadBackups(); },
    restartService: () => void serviceAction('restart'),
    toggleDaemonPower: () => void serviceAction(status?.service.state === 'running' ? 'stop' : 'start'),
    applyConfigAndRestart: () => void saveConfigAndApply(),
    copyConfigText: () => { const editor = $('#nfqwsConfTextarea'); if (editor instanceof HTMLTextAreaElement) void navigator.clipboard.writeText(editor.value).then(() => toast('Конфигурация скопирована')).catch(() => toast('Буфер обмена недоступен', 'error')); },
    updateConfVariable: (key: string, value: string, quoted: boolean) => updateConfigVariable(key, value, quoted),
    copySnippetToClipboard: (value: string) => void navigator.clipboard.writeText(String(value)).then(() => toast('Фрагмент скопирован')).catch(() => toast('Буфер обмена недоступен', 'error')),
    handleConfInput: () => syncConfigLines(),
    syncConfScroll: () => syncConfigLines(),
    toggleWordWrapConf: (enabled: boolean) => { const editor = $('#nfqwsConfTextarea'); if (editor instanceof HTMLTextAreaElement) editor.style.whiteSpace = enabled ? 'pre-wrap' : 'pre'; },
    scrollToConfMarker: (marker: string) => { const editor = $('#nfqwsConfTextarea'); if (!(editor instanceof HTMLTextAreaElement)) return; const offset = editor.value.toLowerCase().indexOf(String(marker).toLowerCase()); if (offset < 0) return toast(`Секция ${marker} не найдена`, 'error'); editor.scrollTop = editor.value.slice(0, offset).split('\n').length * 23; },
    toggleDiffModal: () => { const modal = $('#diffModalBackdrop'); if (modal && !modal.classList.contains('hidden')) { modal.classList.add('hidden'); modal.classList.remove('flex'); } else showDiff(); },
    resetDefaultConf: () => void loadConfig(),
    closeUpdateModal: () => $('#updateModalBackdrop')?.classList.add('hidden'),
    openUpdateModal: () => toast('Обновление требует API безопасной установки и пока отключено.', 'error'),
    filterDocs: (query: string) => { const box = $('#docItemsContainer'); if (box) box.textContent = query ? 'Справочник флагов не подключён.' : 'Справочник флагов не загружен.'; },
    syncListLineNumbers: () => { const box = $('#listLineNumbers'); if (box) box.textContent = '—'; },
    syncListScroll: () => undefined,
    syncCustomHostInput: () => undefined,
    selectListFile: (name: string) => { setText('#currentListFilePath', `/opt/etc/nfqws2/lists/${name}`); toast('API управления списками пока не включён в backend.', 'error'); },
    clearLogsBox: () => { const host = $('#liveLogContainer'); if (host) host.textContent = ''; },
    emitTestLogEvent: () => toast('Тестовая генерация журналов отключена.'),
    showToast: (message: string) => toast(String(message)),
    ...Object.fromEntries(DISABLED_PANEL_ACTIONS.map((name) => [name, () => toast('Эта функция пока не подключена к backend и отключена, чтобы не показывать фиктивный результат.', 'error')])),
  };
  const dispatch = (event: Event, eventName: string) => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const control = target.closest<HTMLElement>(`[data-action-${eventName}], [data-action][data-event="${eventName}"]`);
    if (!control) return;
    const actionName = control.getAttribute(`data-action-${eventName}`) ?? control.dataset.action ?? '';
    const action = handlers[actionName];
    if (!action) return;
    const args: unknown[] = [];
    if (control.dataset.key !== undefined) args.push(control.dataset.key);
    if (control.dataset.valueSource === 'value') args.push((control as HTMLInputElement | HTMLSelectElement).value);
    else if (control.dataset.valueSource === 'checked') args.push((control as HTMLInputElement).checked);
    else if (control.dataset.valueSource === 'self') args.push(control);
    else if (control.dataset.arg !== undefined) args.push(control.dataset.arg);
    if (control.dataset.quoted !== undefined) args.push(control.dataset.quoted === 'true');
    void action(...args);
  };
  document.addEventListener('click', (event) => dispatch(event, 'click'));
  document.addEventListener('input', (event) => dispatch(event, 'input'));
  document.addEventListener('change', (event) => dispatch(event, 'change'));
  document.addEventListener('scroll', (event) => dispatch(event, 'scroll'), true);
}

function boot() {
  bindPrototypeActions();
  document.documentElement.classList.add('dark');
  showRoute('conf');
  $('#authForm')?.addEventListener('submit', (event) => void login(event as SubmitEvent));
  $('#authLogout')?.addEventListener('click', () => void logout());
  $('#nfqwsConfTextarea')?.addEventListener('input', syncConfigLines);
  $('#nfqwsConfTextarea')?.addEventListener('scroll', syncConfigLines);
  $('#refreshStatusBtn')?.addEventListener('click', () => void fetchStatus());
  void fetchSession();
  window.setInterval(() => { if (document.visibilityState === 'visible' && authenticated) void fetchStatus(); }, 7000);
  document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible' && authenticated) void fetchStatus(); });
  if (!authenticated) updateAuthView();
}

if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot, { once: true }); else boot();
