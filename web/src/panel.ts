import { diagnosisRows } from './diagnosis-layout';
import { comparisonVerdict, presentTrace, type Presentation } from './diagnosis-presenter';
import '@fontsource-variable/inter';
import '@fontsource-variable/jetbrains-mono';
import './panel.css';
import { api, nfqwsApply, nfqwsConfig, nfqwsDiagnostics, nfqwsInstallation, nfqwsLists, nfqwsRuntime, nfqwsUpdates, type ConfigResponse, type ConfigValidationResponse, type NfqwsListId, type NfqwsListResponse, type NfqwsModuleReference, type NfqwsRuntime, type PanelRelease, type SiteDiagnosticReport, type SiteTraceOperation, type SiteTraceReport, type StatusResponse } from './api';
import { appendListEntries, deduplicateListContent, findNextListMatch, isListTabSelected, listIdFromTabElementId, removeListLines } from './list-editor';
import { availableNfqwsModes, summarizeNfqwsConfig } from './config-facts';
import { resourceUsagePercent, usageTone } from './metrics';

type Route = 'conf' | 'lists' | 'check' | 'trace' | 'autotune' | 'monitor' | 'community';
export const UNAVAILABLE_PANEL_ROUTES = ['autotune', 'monitor', 'community'] as const;
let status: StatusResponse | null = null;
let config: ConfigResponse | null = null;
let discoveredNetworkInterfaces: string[] = [];
let activeList: NfqwsListResponse | null = null;
let activeListId: NfqwsListId = 'user.list';
let busy = false;
let statusPolling = false;
let diagnosisRunning = false;
let traceRunning = false;
let activeTraceOperationId: string | null = null;
let restartInProgress = false;
let availableReleases: PanelRelease[] = [];
let selectedReleaseTag: string | null = null;
let updatePolling = false;
let updateTarget: 'ui' | 'core' = 'ui';
let installedCoreTag: string | null = null;
const UPDATE_NOTICE_KEY = 'nfqws2-ui-update-notice';
const LIST_DISPLAY_NAMES: Record<NfqwsListId, string> = {
  'user.list': 'Пользовательские домены',
  'auto.list': 'Домены для автообучения',
  'exclude.list': 'Исключённые домены',
  'ipset.list': 'IP-адреса',
  'ipset_exclude.list': 'Исключённые IP-адреса',
};
export const listDisplayName = (id: NfqwsListId) => LIST_DISPLAY_NAMES[id];
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
function schedulePanelRefreshAfterUpdate(message: string) {
  try { window.sessionStorage.setItem(UPDATE_NOTICE_KEY, message); } catch { /* The refresh still ensures the new bundle is loaded. */ }
  toast(`${message} Страница обновится автоматически.`);
  window.setTimeout(() => window.location.reload(), 1200);
}
function showPendingUpdateNotice() {
  try {
    const message = window.sessionStorage.getItem(UPDATE_NOTICE_KEY);
    if (!message) return;
    window.sessionStorage.removeItem(UPDATE_NOTICE_KEY);
    window.setTimeout(() => toast(message), 100);
  } catch { /* Storage can be unavailable in restricted browser contexts. */ }
}
function showDiagnosisProgress(host: HTMLElement, domain: string) {
  const item = document.createElement('div');
  item.className = 'pointer-events-auto max-w-sm rounded-xl border border-blue-500/40 bg-slate-900 px-4 py-3 text-sm text-slate-100 shadow-2xl';
  item.setAttribute('role', 'status');
  item.setAttribute('aria-live', 'polite');
  const heading = document.createElement('p');
  heading.className = 'flex items-center gap-2 font-semibold text-blue-200';
  const spinner = document.createElement('span');
  spinner.className = 'diagnosis-spinner inline-block h-3.5 w-3.5 shrink-0 rounded-full border-2 border-blue-300/30 border-t-blue-300';
  spinner.setAttribute('aria-hidden', 'true');
  const title = document.createElement('span');
  title.textContent = `Проверка домена ${domain}`;
  heading.append(spinner, title);
  const details = document.createElement('p');
  details.className = 'mt-1 text-xs leading-5 text-slate-400';
  details.textContent = 'На роутере проверяются DNS и сверка DoH, TCP/HTTPS, HTTP, маршрут, контрольное SNI и поддержка QUIC.';
  item.append(heading, details);
  host.append(item);
  return () => item.remove();
}
function setBusy(value: boolean) {
  busy = value;
  all<HTMLButtonElement>('button[data-action-click]').forEach((button) => {
    if (value && !button.disabled) { button.disabled = true; button.dataset.busyDisabled = 'true'; }
    else if (!value && button.dataset.busyDisabled === 'true') { button.disabled = false; delete button.dataset.busyDisabled; }
  });
  all<HTMLSelectElement>('select[data-action-change]').forEach((select) => {
    if (value && !select.disabled) { select.disabled = true; select.dataset.busyDisabled = 'true'; }
    else if (!value && select.dataset.busyDisabled === 'true') { select.disabled = false; delete select.dataset.busyDisabled; }
  });
}
function setRestartInProgress(value: boolean) {
  restartInProgress = value;
  const indicator = $('#statusIndicator');
  indicator?.classList.toggle('status-restarting', value);
  indicator?.setAttribute('aria-busy', String(value));
  $('#headerStatusBar')?.classList.toggle('status-restarting', value);
  const label = $('#statusTextLabel');
  if (value && label) {
    label.textContent = 'Перезапуск nfqws2…';
    label.className = 'text-xs sm:text-sm font-semibold text-blue-300 tracking-tight';
  }
}
function showRoute(route: Route) {
  if ((UNAVAILABLE_PANEL_ROUTES as readonly string[]).includes(route)) return;
  document.querySelectorAll<HTMLElement>('[id^="tabContent-"]').forEach((section) => section.classList.toggle('hidden', section.id !== `tabContent-${route}`));
  all<HTMLButtonElement>('.side-nav-btn').forEach((button) => {
    const active = button.id === `navBtn-${route}`;
    button.classList.toggle('active', active);
    button.classList.toggle('text-slate-400', !active);
  });
  if (route === 'lists') void loadList(activeListId);
}
async function fetchStatus() {
  if (statusPolling) return;
  statusPolling = true;
  try {
    status = await api<StatusResponse>('/api/status');
    const service = status.service.state;
    const running = service === 'running';
    const stopped = service === 'stopped';
    const label = running ? 'nfqws2 запущен' : stopped ? 'nfqws2 остановлен' : service === 'error' ? 'Ошибка nfqws2' : 'Статус неизвестен';
    const tone = running ? 'emerald' : stopped || service === 'error' ? 'rose' : 'slate';
    if (!restartInProgress) {
      const text = $('#statusTextLabel'); if (text) { text.textContent = label; text.className = `text-xs sm:text-sm font-semibold text-${tone}-400 tracking-tight`; }
      const dot = $('#statusMainDot'); if (dot) dot.className = `${running ? 'status-dot-active' : ''} relative inline-flex rounded-full h-2.5 w-2.5 bg-${tone}-400`;
      const ring = $('#statusRadarRing'); if (ring) ring.className = `status-radar absolute inline-flex h-full w-full rounded-full bg-${running ? 'emerald' : stopped ? 'rose' : 'slate'}-400`;
    }
    const monitorBadge = $('#sideMonitorBadge'); if (monitorBadge) monitorBadge.className = `w-2 h-2 rounded-full bg-${running ? 'emerald' : 'slate'}-400`;
    const used = status.system.memoryUsedBytes, total = status.system.memoryTotalBytes;
    const ramPercent = resourceUsagePercent(used, total);
    setText('#ramMetricText', used !== null && total !== null ? `${fmtMb(used)} / ${fmtMb(total)} МБ` : '—');
    const ramText = $('#ramMetricText');
    if (ramText) ramText.title = ramPercent === null ? 'Загрузка памяти неизвестна' : `Использовано ${Math.round(ramPercent)}%. Источник: ${status.system.source === 'keenetic-rci' ? 'Keenetic RCI, как в XKeen UI' : 'Linux /proc/meminfo; объём после резервирования памяти прошивкой'}`;
    setWidth('#ramProgressBar', ramPercent);
    applyMetricTone('#ramMetricText', '#ramProgressBar', ramPercent);
    const cpu = status.system.cpuPercent;
    setText('#cpuMetricText', cpu === null ? '—' : `${Math.round(cpu)}%`);
    setWidth('#cpuProgressBar', cpu);
    applyMetricTone('#cpuMetricText', '#cpuProgressBar', cpu);
    setText('#headerCoreVersion', status.service.version ?? '—'); setText('#headerUiVersion', status.panel.version);
    const powerLabel = running ? 'Остановить nfqws2' : 'Запустить nfqws2';
    const power = $('#togglePowerText'); if (power) power.textContent = powerLabel;
    const powerBtn = $('#togglePowerBtn'); if (powerBtn) {
      powerBtn.title = powerLabel;
      powerBtn.setAttribute('aria-label', powerLabel);
      powerBtn.classList.toggle('text-rose-400', running);
      powerBtn.classList.toggle('text-emerald-400', stopped);
      powerBtn.classList.toggle('bg-rose-500/10', running);
      powerBtn.classList.toggle('hover:bg-rose-500/20', running);
      powerBtn.classList.toggle('border-rose-500/30', running);
      powerBtn.classList.toggle('bg-emerald-500/10', stopped);
      powerBtn.classList.toggle('hover:bg-emerald-500/20', stopped);
      powerBtn.classList.toggle('border-emerald-500/30', stopped);
    }
    setText('#checkedAt', status.service.checkedAt ? new Date(status.service.checkedAt * 1000).toLocaleTimeString() : '—');
  } catch (error) {
    status = null;
    if (!restartInProgress) {
      const label = $('#statusTextLabel'); if (label) { label.textContent = 'Статус неизвестен'; label.className = 'text-xs sm:text-sm font-semibold text-slate-400 tracking-tight'; }
      const dot = $('#statusMainDot'); if (dot) dot.className = 'relative inline-flex rounded-full h-2.5 w-2.5 bg-slate-400';
      const ring = $('#statusRadarRing'); if (ring) ring.className = 'absolute inline-flex h-full w-full rounded-full bg-slate-400';
    }
    setText('#ramMetricText', '—'); setWidth('#ramProgressBar', null); applyMetricTone('#ramMetricText', '#ramProgressBar', null);
    setText('#cpuMetricText', '—'); setWidth('#cpuProgressBar', null); applyMetricTone('#cpuMetricText', '#cpuProgressBar', null);
    setText('#headerCoreVersion', '—'); setText('#headerUiVersion', '—');
    const detail = $('#statusDetail'); if (detail) detail.textContent = error instanceof Error ? error.message : 'Ошибка проверки статуса';
  } finally { statusPolling = false; }
}
function fmtMb(bytes: number) { return (bytes / 1024 / 1024).toFixed(0); }
function applyMetricTone(textSelector: string, barSelector: string, percent: number | null) {
  const tone = usageTone(percent);
  const textClass = tone === 'critical' ? 'text-rose-400' : tone === 'warning' ? 'text-amber-400' : 'text-slate-400';
  const barClass = tone === 'critical' ? 'bg-rose-400' : tone === 'warning' ? 'bg-amber-400' : 'bg-slate-400';
  const metric = $(textSelector);
  if (metric) {
    metric.classList.remove('text-slate-100', 'text-slate-400', 'text-emerald-400', 'text-amber-400', 'text-rose-400');
    metric.classList.add(textClass);
  }
  const bar = $(barSelector);
  if (bar) {
    bar.classList.remove('bg-sky-400', 'bg-emerald-400', 'bg-slate-400', 'bg-amber-400', 'bg-rose-400');
    bar.classList.add(barClass);
  }
}
async function loadConfig() {

  try {
    config = await api<ConfigResponse>('/api/config?file=nfqws2.conf');
    const applyButton = $('[data-action-click="applyConfigAndRestart"]');
    let canWriteConfig = false;
    let moduleReferences: NfqwsModuleReference[] = [];
    let networkInterfaces: string[] = [];
    try {
      const installation = await nfqwsInstallation.get();
      canWriteConfig = installation.config?.writable === true;
      moduleReferences = installation.moduleReferences ?? [];
      networkInterfaces = installation.networkInterfaces ?? [];
      discoveredNetworkInterfaces = networkInterfaces;
    } catch (error) {
      toast(error instanceof Error ? error.message : 'Не удалось проверить права записи в конфиг', 'error');
    }
    if (applyButton instanceof HTMLButtonElement) {
      applyButton.disabled = !canWriteConfig;
      applyButton.title = canWriteConfig ? '' : 'Backend не подтвердил право записи в обнаруженный конфиг';
    }
    const editor = $('#nfqwsConfTextarea');
    if (editor instanceof HTMLTextAreaElement) editor.value = config.content;
    setText('#confLinesBadge', `${config.content.split(/\r?\n/).length} строк`);
    setText('#currentConfigPath', 'Конфигурация');
    const configTitle = $('#currentConfigPath'); if (configTitle) configTitle.title = config.file;
    setText('#moduleStatusBadge', `${moduleReferences.length} ссылок в конфигурации`);
    renderConfigModules(moduleReferences);
    renderConfigSections(config.content);
    await loadRuntimeProfiles();
    updateQuickParameters(config.content, networkInterfaces);
    const queue = configAssignmentValue(config.content, 'NFQUEUE_NUM');
    setText('#queueBadge', queue ? `Q:${queue.replace(/^['"]|['"]$/g, '')}` : 'Q:—');
    syncConfigLines();
  } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось прочитать конфигурацию', 'error'); }
}
async function loadRuntimeProfiles() {
  const host = $('#nfqwsRuntimeProfiles');
  if (!host) return;
  host.replaceChildren();
  try {
    const runtime = await nfqwsRuntime.get();
    renderRuntimeProfiles(host, runtime);
  } catch (error) {
    const message = document.createElement('div');
    message.className = 'text-rose-300';
    message.textContent = error instanceof Error ? `Снимок процесса недоступен: ${error.message}` : 'Снимок процесса недоступен';
    host.append(message);
  }
}
function renderRuntimeProfiles(host: HTMLElement, runtime: NfqwsRuntime) {
  host.replaceChildren();
  if (!runtime.running) {
    const stopped = document.createElement('div');
    stopped.className = 'text-amber-300';
    stopped.textContent = 'Процесс nfqws2 не обнаружен; фактические профили недоступны.';
    host.append(stopped);
  } else if (!runtime.profiles.length) {
    const empty = document.createElement('div');
    empty.className = 'text-amber-300';
    empty.textContent = `Процесс PID ${runtime.pid} найден, но профильные фильтры в его argv не обнаружены.`;
    host.append(empty);
  } else {
    for (const profile of runtime.profiles) {
      const row = document.createElement('div');
      row.className = 'rounded-xl bg-[#070b14] border border-slate-800/90 p-2.5 space-y-1';
      const title = document.createElement('div');
      title.className = 'text-slate-200 font-semibold';
      title.textContent = `#${profile.index}${profile.name ? ` · ${profile.name}` : ''}`;
      const details = document.createElement('div');
      details.className = 'text-slate-400 whitespace-pre-wrap break-all';
      details.textContent = [
        profile.filters.length ? `Фильтры: ${profile.filters.join(', ')}` : '',
        profile.strategies.length ? `Lua: ${profile.strategies.join(', ')}` : '',
        profile.payloads.length ? `Payload: ${profile.payloads.join(', ')}` : '',
      ].filter(Boolean).join('\n');
      row.append(title, details);
      host.append(row);
    }
    const source = document.createElement('div');
    source.className = 'text-slate-500';
    source.textContent = `Снято с /proc/${runtime.pid}/cmdline · это подтверждает аргументы процесса, не успешную обработку трафика.`;
    host.append(source);
  }
  for (const note of runtime.limitations) {
    const limitation = document.createElement('div');
    limitation.className = 'text-slate-500';
    limitation.textContent = note;
    host.append(limitation);
  }
}
function renderConfigSections(content: string) {
  const box = $('#nfqwsConfigSections');
  if (!box) return;
  const facts = summarizeNfqwsConfig(content);
  box.replaceChildren();
  setText('#profileStatusBadge', `${facts.sections.length} секций в конфигурации`);

  const mode = document.createElement('div');
  mode.className = 'p-2.5 rounded-xl bg-[#070b14] border border-slate-800/90 flex items-center justify-between gap-2';
  const modeLabel = document.createElement('span'); modeLabel.className = 'text-slate-300';
  modeLabel.textContent = `NFQWS_EXTRA_ARGS · выбранное значение: ${facts.selectedMode ?? 'не задано'}`;
  const modeState = document.createElement('span'); modeState.className = 'px-2 py-0.5 rounded bg-slate-800 text-slate-300 font-bold'; modeState.textContent = 'В конфигурации';
  mode.append(modeLabel, modeState); box.append(mode);

  if (!facts.sections.length) {
    const empty = document.createElement('div'); empty.className = 'p-2.5 rounded-xl bg-[#070b14] border border-slate-800/90 text-slate-400';
    empty.textContent = 'Не найдены буквальные значения секций NFQWS_*_ARGS; динамические shell-переменные здесь не интерпретируются.'; box.append(empty); return;
  }
  for (const section of facts.sections) {
    const row = document.createElement('div'); row.className = 'p-2.5 rounded-xl bg-[#070b14] border border-slate-800/90 flex items-center justify-between gap-2';
    const label = document.createElement('span'); label.className = 'text-slate-300'; label.textContent = section.variable;
    const details = document.createElement('span'); details.className = 'text-right text-slate-400';
    details.textContent = `${section.strategies} --lua-desync · ${section.profiles} --new · ${section.filters} фильтров`;
    row.append(label, details); box.append(row);
  }
}
function renderConfigModules(references: NfqwsModuleReference[]) {
  const host = $('#moduleFilesContainer'); if (!host) return;
  host.replaceChildren();
  if (!references.length) { const empty = document.createElement('span'); empty.className = 'text-slate-500'; empty.textContent = 'Backend не обнаружил literal-ссылки Lua/blob в активном конфиге'; host.append(empty); return; }
  for (const reference of references) {
    const path = reference.resolvedPath ?? reference.path;
    const button = document.createElement('button');
    button.className = `px-2 py-1 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-800 ${reference.exists && reference.readable ? 'text-emerald-300' : 'text-amber-300'}`;
    const fileName = path?.split(/[\\/]/).pop() ?? reference.argument;
    const fileStatus = reference.resolution === 'dynamic'
      ? 'путь вычисляется shell; файл не проверен'
      : reference.resolution === 'relative_unknown'
        ? 'относительный путь; базовый каталог неизвестен'
        : reference.exists && reference.readable
          ? 'файл существует и доступен для чтения'
          : reference.exists
            ? 'файл существует, но чтение не подтверждено'
            : 'файл не найден по literal-пути';
    button.textContent = `${reference.kind}: ${fileName}`;
    button.title = `${path ?? reference.argument} · ${fileStatus}`;
    button.addEventListener('click', () => void navigator.clipboard.writeText(reference.argument).then(() => toast('Аргумент скопирован')).catch(() => toast('Буфер обмена недоступен', 'error')));
    host.append(button);
  }
}
function configAssignmentValue(content: string, key: string): string {
  const line = content.split(/\r?\n/).find((item) => item.trimStart().startsWith(`${key}=`));
  if (!line) return '';
  const raw = line.slice(line.indexOf('=') + 1).trim();
  return raw.length >= 2 && ((raw.startsWith('"') && raw.endsWith('"')) || (raw.startsWith("'") && raw.endsWith("'"))) ? raw.slice(1, -1) : raw;
}
function updateQuickParameters(content: string, networkInterfaces: string[] = []) {
  const availableModes = new Set(availableNfqwsModes(content));
  const interfaceSelect = $('[data-key="ISP_INTERFACE"]');
  if (interfaceSelect instanceof HTMLSelectElement) {
    const configured = configAssignmentValue(content, 'ISP_INTERFACE');
    interfaceSelect.replaceChildren(new Option('Выберите обнаруженный интерфейс', '', !configured, false));
    for (const name of [...new Set(networkInterfaces)].sort()) interfaceSelect.add(new Option(name, name));
    if (configured && !networkInterfaces.includes(configured)) {
      interfaceSelect.add(new Option(`${configured} (настроен, но не обнаружен)`, configured, true, true));
    } else {
      interfaceSelect.value = configured;
    }
    interfaceSelect.disabled = networkInterfaces.length === 0;
    interfaceSelect.title = networkInterfaces.length ? 'Варианты получены из /sys/class/net на устройстве' : 'Backend не обнаружил сетевые интерфейсы; измените ISP_INTERFACE в raw-конфиге после проверки устройства';
  }
  all<HTMLSelectElement>('select[data-action-change="updateConfVariable"][data-key]').forEach((select) => {
    const key = select.dataset.key;
    if (!key) return;
    const value = configAssignmentValue(content, key);
    select.value = [...select.options].some((option) => option.value === value) ? value : '';
  });
  all<HTMLSelectElement>('select[data-action-change="updateConfVariable"][data-args]').forEach((select) => {
    let key = '';
    try { key = (JSON.parse(select.dataset.args ?? '[]') as unknown[])[0] as string ?? ''; } catch { return; }
    if (key !== 'NFQWS_EXTRA_ARGS') return;
    const selectedMode = configAssignmentValue(content, key);
    select.value = [...select.options].some((option) => option.value === selectedMode) ? selectedMode : '';
    for (const option of [...select.options]) {
      if (!/^\$MODE_(?:LIST|AUTO|ALL)$/.test(option.value)) continue;
      option.disabled = !availableModes.has(option.value);
      option.title = option.disabled ? `${option.value.slice(1)} отсутствует или пуст в загруженном конфиге` : '';
    }
  });
}
async function updateConfigVariable(key: string, value: string) {
  if (busy || !config) return;
  const editor = $('#nfqwsConfTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return;
  const before = editor.value;
  setBusy(true);
  try {
    const preview = await nfqwsConfig.preview(before, key, value);
    if (editor.value !== before) throw new Error('Черновик изменился во время запроса; повторите выбор настройки.');
    editor.value = preview.content;
    syncConfigLines();
    renderConfigSections(editor.value);
    toast('Черновик изменён. Нажмите «Применить конфигурацию», чтобы сохранить.');
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось изменить черновик', 'error');
  } finally {
    setBusy(false);
    updateQuickParameters(editor.value, discoveredNetworkInterfaces);
  }
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
  if (!config) return toast('Сначала загрузите конфигурацию backend.', 'error');
  const editor = $('#nfqwsConfTextarea'); if (!(editor instanceof HTMLTextAreaElement)) return;
  setBusy(true);
  let validation: ConfigValidationResponse;
  try {
    validation = await nfqwsConfig.validate(editor.value);
  } catch (error) {
    setBusy(false);
    return toast(error instanceof Error ? `Проверка backend не выполнена: ${error.message}` : 'Проверка backend не выполнена', 'error');
  }
  if (!validation.valid) {
    setBusy(false);
    const messages = validation.diagnostics.filter((item) => item.severity === 'error')
      .map((item) => `${item.line ? `Строка ${item.line}: ` : ''}${item.message}`);
    return toast(`Конфигурация не прошла проверку: ${messages.join(' · ')}`, 'error');
  }
  const warnings = validation.diagnostics.filter((item) => item.severity === 'warning');
  const binaryCheckNote = validation.checks.installedBinary === 'not_run'
    ? 'Проверка установленным бинарником не выполнялась.'
    : 'Проверка установленным бинарником выполнена.';
  if ((warnings.length || validation.checks.installedBinary === 'not_run') && !window.confirm([
    'Backend проверил только безопасно разбираемый синтаксис и известные ограничения init-скрипта.',
    ...warnings.map((item) => `${item.line ? `Строка ${item.line}: ` : ''}${item.message}`),
    binaryCheckNote,
    'Продолжить запись и перезапуск?'
  ].join('\n\n'))) {
    setBusy(false);
    return;
  }
  setRestartInProgress(true);
  try {
    const result = await nfqwsApply.apply(editor.value, config.revision);
    config = result.config;
    setRestartInProgress(false);
    await fetchStatus();
    await loadRuntimeProfiles();
    toast('Конфигурация сохранена, повторно прочитана; backend подтвердил перезапуск nfqws2.');
  } catch (error) { setRestartInProgress(false); await fetchStatus(); await loadRuntimeProfiles(); toast(error instanceof Error ? error.message : 'Не удалось сохранить конфигурацию', 'error'); }
  finally { setRestartInProgress(false); setBusy(false); }
}
async function serviceAction(action: 'start' | 'stop' | 'restart', confirmed = false) {
  if (busy) return;

  if (!confirmed && action === 'stop' && !window.confirm('Остановить nfqws2?')) return;
  if (!confirmed && action === 'restart' && !window.confirm('Перезапустить nfqws2?')) return;
  setBusy(true);
  if (action === 'restart') setRestartInProgress(true);
  try { await api('/api/service', { method: 'POST', body: JSON.stringify({ action }) }); toast('Команда выполнена'); }
  catch (error) { toast(error instanceof Error ? error.message : 'Команда не выполнена', 'error'); }
  finally {
    if (action === 'restart') setRestartInProgress(false);
    setBusy(false);
    await fetchStatus();
    await loadRuntimeProfiles();
  }
}
async function loadList(id: NfqwsListId) {

  try {
    const index = await nfqwsLists.index();
    const summary = index.files.find((file) => file.id === id);
    const availableListIds = new Set(index.files.map((file) => file.id));
    for (const candidate of all<HTMLButtonElement>('.list-file-tab')) {
      const tabId = listIdFromTabElementId(candidate.id);
      const available = tabId !== null && availableListIds.has(tabId);
      candidate.disabled = !available;
      candidate.classList.toggle('opacity-40', !available);
      const discovered = index.files.find((file) => file.id === tabId);
      candidate.title = discovered ? `${tabId ? listDisplayName(tabId) : 'Список'} — ${discovered.path}` : 'Файл не найден ни по ссылке конфигурации, ни в штатном каталоге lists';
    }
    if (!summary) {
      activeList = null;
      const editor = $('#listEditorTextarea');
      if (editor instanceof HTMLTextAreaElement) { editor.value = ''; editor.placeholder = 'Файл отсутствует; панель не создаёт списки автоматически.'; }
      setText('#currentListFilePath', 'Файл не найден');
      setText('#listFooterCount', 'Список недоступен');
      return toast(`Файл ${id} не найден по настройкам nfqws2 или в штатном каталоге lists.`, 'error');
    }
    const result = await nfqwsLists.get(id);
    activeList = result;
    activeListId = id;
    const editor = $('#listEditorTextarea');
    if (editor instanceof HTMLTextAreaElement) { editor.value = result.content; editor.placeholder = ''; }
    const locationLabel = $('#currentListFilePath');
    if (locationLabel) {
      locationLabel.textContent = listDisplayName(id);
      locationLabel.title = result.path;
      locationLabel.setAttribute('aria-label', `${listDisplayName(id)}. Путь: ${result.path}`);
    }
    for (const candidate of all<HTMLButtonElement>('.list-file-tab')) {
      const selected = isListTabSelected(candidate.id, id);
      candidate.classList.toggle('active', selected);
      candidate.classList.toggle('text-white', selected);
      candidate.classList.toggle('font-bold', selected);
      candidate.classList.toggle('border-sky-400', selected);
      candidate.classList.toggle('bg-slate-800/40', selected);
      candidate.classList.toggle('text-slate-400', !selected);
      candidate.classList.toggle('border-transparent', !selected);
    }
    syncListLineNumbers(editor instanceof HTMLTextAreaElement ? editor : undefined);
    const saveButton = $('[data-action="saveActiveListFile"]');
    if (saveButton instanceof HTMLButtonElement) saveButton.disabled = !summary.writable || busy;
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось прочитать список', 'error');
  }
}
async function saveActiveList() {
  if (busy) return;
  if (!activeList) return toast('Сначала загрузите список из backend.', 'error');
  const editor = $('#listEditorTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return;
  setBusy(true);
  try {
    activeList = await nfqwsLists.save(activeList.id, editor.value, activeList.revision);
    editor.value = activeList.content;
    syncListLineNumbers(editor);
    toast('Список записан, резервная копия создана, повторное чтение подтверждено. Служба не перезапускалась.');
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось сохранить список', 'error');
  } finally { setBusy(false); }
}
function syncListLineNumbers(editor = $('#listEditorTextarea')) {
  if (!(editor instanceof HTMLTextAreaElement)) return;
  const gutter = $('#listLineNumbers');
  if (!gutter) return;
  const lines = Math.max(1, editor.value.split(/\r?\n/).length);
  gutter.innerHTML = Array.from({ length: lines }, (_, index) => `${index + 1}`).join('<br>');
  gutter.scrollTop = editor.scrollTop;
  const entries = editor.value.split(/\r?\n/).filter((line) => line.trim() && !line.trimStart().startsWith('#')).length;
  setText('#listFooterCount', `Всего записей: ${entries}`);
}
function syncListScroll(editor: HTMLTextAreaElement) {
  const gutter = $('#listLineNumbers');
  if (gutter) gutter.scrollTop = editor.scrollTop;
}
function removeSelectedListLines() {
  if (!activeList) return toast('Сначала загрузите список из backend.', 'error');
  const editor = $('#listEditorTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return;
  const result = removeListLines(editor.value, editor.selectionStart, editor.selectionEnd);
  if (!result.removed) return toast('Список пуст.');
  editor.value = result.content;
  editor.focus();
  syncListLineNumbers(editor);
  toast(`Удалено строк: ${result.removed}. Изменён только черновик.`);
}
function findNextListEntry() {
  const editor = $('#listEditorTextarea'); const input = $('#listSearchInput');
  if (!(editor instanceof HTMLTextAreaElement) || !(input instanceof HTMLInputElement)) return;
  const match = findNextListMatch(editor.value, input.value, editor.selectionEnd);
  if (!match) return toast(input.value.trim() ? 'Совпадений не найдено.' : 'Введите текст для поиска.', 'error');
  editor.focus(); editor.setSelectionRange(match.start, match.end);
}
async function importListDraft() {
  if (!activeList) return toast('Сначала загрузите список из backend.', 'error');
  const input = $('#listImportFile'); const editor = $('#listEditorTextarea');
  if (!(input instanceof HTMLInputElement) || !(editor instanceof HTMLTextAreaElement)) return;
  const file = input.files?.[0];
  if (!file) return;
  try {
    editor.value = appendListEntries(editor.value, await file.text());
    syncListLineNumbers(editor);
    toast(`Содержимое ${file.name} добавлено в черновик; сохранение и серверная проверка выполняются отдельно.`);
  } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось прочитать файл', 'error'); }
  finally { input.value = ''; }
}
function exportListDraft() {
  if (!activeList) return toast('Сначала загрузите список из backend.', 'error');
  const editor = $('#listEditorTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return;
  const url = URL.createObjectURL(new Blob([editor.value], { type: 'text/plain;charset=utf-8' }));
  const link = document.createElement('a'); link.href = url; link.download = activeList.id; link.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
  toast('Текущий черновик экспортирован.');
}
function resultBanner(presentation: Presentation): HTMLElement {
  const banner = document.createElement('p'); banner.className = `diagnosis-verdict diagnosis-${presentation.kind} col-span-full`; banner.setAttribute('role', 'status'); banner.textContent = presentation.text; return banner;
}
function diagnosisIcon(kind: Presentation['kind']): HTMLElement {
  const icon = document.createElement('span'); icon.className = `diagnosis-state-icon diagnosis-icon-${kind}`;
  icon.textContent = kind === 'success' ? '✓' : kind === 'failure' ? '×' : '!';
  icon.setAttribute('role', 'img'); icon.setAttribute('aria-label', kind === 'success' ? 'Ответ получен' : kind === 'failure' ? 'Ошибка проверки' : 'Результат не определён'); return icon;
}
function renderSiteDiagnosis(container: HTMLElement, report: SiteDiagnosticReport) {
  container.replaceChildren();
  const verdict = comparisonVerdict(report.direct, report.current);
  const overview = document.createElement('section'); overview.className = `diagnosis-overview diagnosis-${verdict.kind}`; overview.setAttribute('role', 'status');
  const copy = document.createElement('div'); copy.className = 'diagnosis-overview-copy';
  const title = document.createElement('h3'); title.className = 'diagnosis-overview-title'; title.textContent = verdict.text;
  const subtitle = document.createElement('p'); subtitle.className = 'diagnosis-overview-detail'; subtitle.textContent = [report.host, report.addresses.join(', ')].filter(Boolean).join(' · '); copy.append(title, subtitle);
  const retry = document.createElement('button'); retry.type = 'button'; retry.className = 'diagnosis-retry'; retry.textContent = '↻ Ещё раз'; retry.setAttribute('aria-label', `Повторить проверку ${report.host}`);
  retry.addEventListener('click', () => { const input = $('#diagDomainInput'); if (input instanceof HTMLInputElement && !diagnosisRunning) { input.value = report.host; void runSiteDiagnosis(); } });
  overview.append(diagnosisIcon(verdict.kind), copy, retry); container.append(overview);
  const checks = document.createElement('section'); checks.className = 'diagnosis-checks ui-card rounded-2xl';
  const header = document.createElement('div'); header.className = 'diagnosis-checks-header';
  const heading = document.createElement('h3'); heading.className = 'diagnosis-checks-title'; heading.textContent = 'Что проверено';
  const host = document.createElement('span'); host.className = 'diagnosis-checks-host'; host.textContent = report.host; header.append(heading, host); checks.append(header);
  const list = document.createElement('div'); list.setAttribute('role', 'list');
  for (const item of diagnosisRows(report)) {
    const row = document.createElement('div'); row.className = `diagnosis-row${item.inactive ? ' diagnosis-row-inactive' : ''}`; row.setAttribute('role', 'listitem');
    const label = document.createElement('h4'); label.className = 'diagnosis-row-label'; label.textContent = item.label;
    const body = document.createElement('div'); body.className = 'diagnosis-row-body';
    if (item.metrics) { const metrics = document.createElement('p'); metrics.className = 'diagnosis-row-metrics'; metrics.textContent = item.metrics; body.append(metrics); }
    const detail = document.createElement('p'); detail.className = 'diagnosis-row-detail'; detail.textContent = item.detail; body.append(detail);
    if (item.note) { const note = document.createElement('p'); note.className = 'diagnosis-row-note'; note.textContent = item.note; body.append(note); }
    const icon = diagnosisIcon(item.kind); if (item.inactive) { icon.textContent = '—'; icon.setAttribute('aria-label', 'Этап не выполнялся'); }
    row.append(icon, label, body); list.append(row);
  }
  checks.append(list); container.append(checks);
  const notes = document.createElement('details'); notes.className = 'diagnosis-notes';
  const disclosure = document.createElement('summary'); disclosure.textContent = 'Как читать результаты'; notes.append(disclosure);
  for (const text of [report.summary, ...report.limitations]) { const p = document.createElement('p'); p.textContent = text; notes.append(p); } container.append(notes);
}
async function runSiteDiagnosis() {
  if (diagnosisRunning) return;
  const input = $('#diagDomainInput'); const container = $('#diagResultContainer');
  const button = $('[data-action-click="runSiteDiagnosis"]');
  if (!(input instanceof HTMLInputElement) || !container || !(button instanceof HTMLButtonElement)) return;
  if (!input.value.trim()) return toast('Введите домен для проверки.', 'error');
  diagnosisRunning = true; input.disabled = true; button.disabled = true;
  button.textContent = 'Проверяем…'; container.replaceChildren();
  const toastHost = $('#toastWrapper');
  const closeProgress = toastHost ? showDiagnosisProgress(toastHost, input.value.trim()) : () => undefined;
  const loading = document.createElement('p'); loading.className = 'col-span-full text-sm text-blue-300 animate-pulse';
  loading.setAttribute('role', 'status'); loading.textContent = 'Сравниваем доступ напрямую, текущий маршрут и QUIC с роутера…'; container.append(loading);
  try {
    const report = await nfqwsDiagnostics.site(input.value);
    container.replaceChildren();
    renderSiteDiagnosis(container, report);
    closeProgress();
    toast(`Проверка ${report.host} завершена: ${report.summary}`);
  } catch (error) {
    container.replaceChildren(); const message = error instanceof Error ? error.message : 'Не удалось выполнить проверку сайта';
    const row = document.createElement('p'); row.className = 'col-span-full text-sm text-rose-400'; row.textContent = message; container.append(row); closeProgress(); toast(message, 'error');
  } finally { diagnosisRunning = false; input.disabled = false; button.disabled = false; button.textContent = 'Проверить'; }
}

function renderTraceReport(container: HTMLElement, report: SiteTraceReport) {
  container.replaceChildren();
  const phaseCard = (title: string, phase: SiteTraceReport['withoutNfqws2'], route: 'direct' | 'current') => {
    const presentation = presentTrace(phase, route);
    const card = document.createElement('article'); card.className = 'ui-card rounded-2xl p-5 space-y-2';
    const heading = document.createElement('h3'); heading.className = 'text-sm font-bold text-white'; heading.textContent = title;
    const state = resultBanner(presentation);
    if (phase.code !== null && phase.code < 400) state.textContent += ` · HTTP ${phase.code}`;
    const reason = document.createElement('p'); reason.className = 'trace-result-copy text-xs whitespace-pre-wrap break-words'; reason.textContent = phase.reason;
    const timing = phase.durationMs === null ? null : document.createElement('p');
    if (timing) { timing.className = 'trace-result-metric text-xs'; timing.textContent = `${phase.durationMs} мс`; }
    card.append(heading, state, reason); if (timing) card.append(timing);
    return card;
  };
  const header = document.createElement('div'); header.className = 'col-span-full ui-card rounded-2xl p-4';
  const title = document.createElement('h3'); title.className = 'font-bold text-white'; title.textContent = `${report.host} · ${report.summary}`;
  const address = document.createElement('p'); address.className = 'mt-1 text-xs font-mono text-slate-400'; address.textContent = `Проверенный адрес: ${report.address ?? 'не определён'}`;
  header.append(title, address);
  container.append(header, phaseCard('Напрямую, мимо nfqws2', report.withoutNfqws2, 'direct'), phaseCard('Текущая стратегия nfqws2', report.withNfqws2, 'current'));
  const insights = document.createElement('section'); insights.className = 'col-span-full ui-card rounded-2xl p-5 space-y-3';
  const insightTitle = document.createElement('h3'); insightTitle.className = 'text-sm font-bold text-white'; insightTitle.textContent = 'Что произошло';
  const host = document.createElement('p'); host.className = 'text-xs text-slate-300'; host.textContent = `Имя сайта по SNI/Host: ${report.traceHostname ?? 'не определено'}`;
  const profile = document.createElement('p'); profile.className = 'text-xs text-slate-300'; profile.textContent = `Профиль: ${report.traceProfile ?? 'не обнаружен в журнале'}`;
  insights.append(insightTitle, host, profile);
  for (const [label, entries, color] of [['Действия', report.traceActions, 'text-emerald-300'], ['Предупреждения', report.traceWarnings, 'text-amber-300']] as const) {
    if (!entries.length) continue;
    const heading = document.createElement('p'); heading.className = `text-xs font-semibold ${color}`; heading.textContent = label; insights.append(heading);
    const list = document.createElement('ul'); list.className = 'pl-4 list-disc text-xs text-slate-300 space-y-1';
    for (const item of entries) { const row = document.createElement('li'); row.textContent = item; list.append(row); }
    insights.append(list);
  }
  container.append(insights);
  const logCard = document.createElement('section'); logCard.className = 'col-span-full ui-card rounded-2xl p-5 space-y-3';
  const logHeading = document.createElement('h3'); logHeading.className = 'text-sm font-bold text-white'; logHeading.textContent = `Журнал nfqws2 --debug · ${report.traceLines.length} строк${report.traceTruncated ? ' · журнал обрезан по лимиту' : ''}`;
  const filter = document.createElement('input'); filter.type = 'search'; filter.className = 'w-full bg-[#060a12] border border-slate-700 rounded-xl px-3 py-2 text-xs text-white'; filter.placeholder = 'Фильтр журнала'; filter.setAttribute('aria-label', 'Фильтр журнала трассировки');
  const pre = document.createElement('pre'); pre.className = 'max-h-[60vh] overflow-auto rounded-xl bg-[#060a12] border border-slate-800 p-4 text-[11px] leading-5 font-mono text-slate-300 whitespace-pre-wrap break-all';
  const draw = () => { const query = filter.value.trim().toLowerCase(); pre.textContent = report.traceLines.filter((line) => !query || line.toLowerCase().includes(query)).join('\n'); };
  filter.addEventListener('input', draw); draw(); logCard.append(logHeading, filter, pre); container.append(logCard);
}

async function runSiteTrace() {
  if (traceRunning) return;
  const input = $('#traceDomainInput'); const container = $('#traceResultContainer'); const button = $('[data-action-click="runSiteTrace"]');
  if (!(input instanceof HTMLInputElement) || !container || !(button instanceof HTMLButtonElement)) return;
  if (!input.value.trim()) return toast('Введите домен для трассировки.', 'error');
  const stop = $('#traceStopButton'); const operationStatus = $('#traceOperationStatus');
  traceRunning = true; activeTraceOperationId = null; input.disabled = true; button.disabled = true; button.textContent = 'Трассировка…';
  if (stop instanceof HTMLButtonElement) { stop.disabled = false; stop.classList.remove('hidden'); }
  if (operationStatus) { operationStatus.textContent = 'Подготовка фоновой проверки на роутере…'; operationStatus.classList.remove('hidden'); }
  container.replaceChildren();
  const progress = document.createElement('p'); progress.className = 'col-span-full text-sm text-blue-300 animate-pulse'; progress.setAttribute('role', 'status'); progress.textContent = 'Операция выполняется на роутере. Можно перейти в другой раздел; статус сохранится.'; container.append(progress);
  const protocol = $('#traceProtoSelect');
  const proto = protocol instanceof HTMLSelectElement && protocol.value === 'http' ? 'http' : 'https';
  try {
    const started = await nfqwsDiagnostics.traceStart(input.value, proto);
    activeTraceOperationId = started.id;
    let current: SiteTraceOperation = started;
    const terminal = ['complete', 'cancelled', 'failed'];
    for (let attempt = 0; attempt < 90 && !terminal.includes(current.state); attempt += 1) {
      if (operationStatus) operationStatus.textContent = current.message || `Этап: ${current.stage}`;
      await new Promise((resolve) => window.setTimeout(resolve, 900));
      try { current = await nfqwsDiagnostics.traceStatus(started.id); }
      catch (error) {
        if (operationStatus) operationStatus.textContent = error instanceof Error ? `${current.message} · переподключение…` : 'Связь с роутером восстановится автоматически…';
      }
    }
    if (current.report) renderTraceReport(container, current.report);
    else {
      container.replaceChildren(); const row = document.createElement('p'); row.className = `col-span-full text-sm ${current.state === 'failed' ? 'text-rose-400' : 'text-amber-300'}`;
      row.textContent = current.message || 'Результат трассировки пока не получен'; container.append(row);
    }
    if (operationStatus) operationStatus.textContent = current.message;
  } catch (error) {
    container.replaceChildren(); const row = document.createElement('p'); row.className = 'col-span-full text-sm text-rose-400'; row.textContent = error instanceof Error ? error.message : 'Трассировка не выполнена'; container.append(row);
  } finally {
    traceRunning = false; activeTraceOperationId = null; input.disabled = false; button.disabled = false; button.textContent = 'Запустить трассировку';
    if (stop instanceof HTMLButtonElement) { stop.disabled = false; stop.classList.add('hidden'); }
  }
}

async function stopSiteTrace() {
  if (!traceRunning || !activeTraceOperationId) return;
  const button = $('#traceStopButton');
  if (button instanceof HTMLButtonElement) { button.disabled = true; button.textContent = 'Останавливаю…'; }
  try {
    const operation = await nfqwsDiagnostics.traceStop(activeTraceOperationId);
    const status = $('#traceOperationStatus');
    if (status) status.textContent = operation.message;
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось остановить трассировку', 'error');
    if (button instanceof HTMLButtonElement) { button.disabled = false; button.textContent = 'Остановить'; }
  }
}

export const DISABLED_PANEL_ACTIONS = ['startStrategyAutoScan','applyStrategyToConf','sendTestTelegram','applyCommunityPreset','fetchAsnPrefixes','pushBackupToNas','exportFullBackup','excludeBrokenDomain','findTelegramChatId'] as const;

function renderReleaseNotes(target: HTMLElement, notes: string) {
  const lines = notes.split(/\r?\n/).map((line) => line.trim()).filter(Boolean).slice(0, 8);
  if (!lines.length) return;
  const list = document.createElement('ul');
  list.className = 'mt-2 list-disc pl-4 text-xs leading-5 text-slate-300';
  for (const line of lines) {
    const item = document.createElement('li');
    item.textContent = line.replace(/^[-*# ]+/, '').slice(0, 240);
    list.append(item);
  }
  target.append(list);
}

function renderReleases() {
  const list = $('#updateVersionsList');
  const install = $('#confirmUpdateInstallBtn');
  if (!list) return;
  list.replaceChildren();
  if (!availableReleases.length) {
    const empty = document.createElement('p');
    empty.className = 'py-8 text-center text-sm text-slate-400';
    empty.textContent = updateTarget === 'core' ? 'Для этой архитектуры нет релизов nfqws2 с проверяемым SHA-256.' : 'Стабильные версии панели с ARM64-пакетом не найдены.';
    list.append(empty);
  }
  for (const release of availableReleases) {
    const selected = selectedReleaseTag === release.tag;
    const card = document.createElement('button');
    card.type = 'button';
    card.dataset.actionClick = 'selectVersionItem';
    card.dataset.args = JSON.stringify([release.tag]);
    card.setAttribute('aria-pressed', String(selected));
    card.className = `relative block w-full rounded-xl border p-3 pr-10 text-left transition-colors ${selected ? 'border-blue-400 bg-blue-500/20' : 'border-slate-700 bg-slate-900/50 hover:border-slate-500'}`;
    const title = document.createElement('span');
    title.className = 'block text-sm font-bold text-slate-100';
    title.textContent = release.tag;
    card.append(title);
    if (updateTarget === 'core' && release.tag === installedCoreTag) {
      const installed = document.createElement('span');
      installed.className = 'ml-2 rounded bg-emerald-900/70 px-2 py-0.5 text-[10px] font-semibold text-emerald-300';
      installed.textContent = 'Установлена';
      title.append(installed);
    }
    const date = document.createElement('span');
    date.className = 'mt-1 block text-xs text-slate-400';
    date.textContent = release.publishedAt ? new Date(release.publishedAt).toLocaleDateString() : 'Дата публикации не указана';
    card.append(date);
    const radio = document.createElement('span');
    radio.setAttribute('aria-hidden', 'true');
    radio.className = `absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 rounded-full border-2 ${selected ? 'border-blue-300 bg-blue-500' : 'border-slate-500'}`;
    card.append(radio);
    renderReleaseNotes(card, release.notes);
    list.append(card);
  }
  if (install instanceof HTMLButtonElement) install.disabled = !selectedReleaseTag || updatePolling || (updateTarget === 'core' && selectedReleaseTag === installedCoreTag);
  setText('#updateModalTitle', updateTarget === 'core' ? 'Обновление nfqws2' : 'Обновление nfqws2-UI');
  setText('#updatePickerHint', updateTarget === 'core' ? 'Выберите версию ядра nfqws2' : 'Выберите версию панели');
  setText('#updateRepoSlug', updateTarget === 'core' ? 'nfqws/nfqws2-keenetic' : 'rangerxxyy/nfqws2-ui');
  const uiTab = $('#updateTargetUi'); const coreTab = $('#updateTargetCore');
  uiTab?.classList.toggle('bg-blue-500/20', updateTarget === 'ui');
  uiTab?.classList.toggle('text-blue-300', updateTarget === 'ui');
  coreTab?.classList.toggle('bg-blue-500/20', updateTarget === 'core');
  coreTab?.classList.toggle('text-blue-300', updateTarget === 'core');
}

async function openUpdatePicker(target: 'ui' | 'core' = 'ui') {
  updateTarget = target;
  const modal = $('#updateModalBackdrop');
  if (!modal) return;
  modal.classList.remove('hidden'); modal.classList.add('flex');
  const list = $('#updateVersionsList');
  if (list) list.textContent = 'Загружаю версии из GitHub Releases…';
  try {
    if (target === 'core') {
      const result = await nfqwsUpdates.coreReleases();
      availableReleases = result.releases;
      installedCoreTag = result.installedTag;
    } else {
      const result = await nfqwsUpdates.releases();
      availableReleases = result.releases;
      installedCoreTag = null;
    }
    selectedReleaseTag = availableReleases[0]?.tag ?? null;
    renderReleases();
  } catch (error) {
    if (list) list.textContent = error instanceof Error ? error.message : 'Не удалось загрузить версии.';
    toast('Не удалось получить список версий из GitHub Releases.', 'error');
  }
}

async function installSelectedRelease() {
  if (!selectedReleaseTag || updatePolling) return;
  const tag = selectedReleaseTag;
  updatePolling = true;
  renderReleases();
  const progress = $('#updateInstallProgressBox');
  progress?.classList.remove('hidden');
  const core = updateTarget === 'core';
  const labels: Record<string, string> = { downloading: 'Скачивание выбранной версии…', verifying: 'Проверка SHA-256…', installing: core ? 'Установка через opkg…' : 'Установка панели…', health_check: core ? 'Перезапуск и проверка nfqws2…' : 'Проверка запуска и откат при ошибке…' };
  toast(`Начато обновление ${core ? 'ядра nfqws2' : 'панели'} до ${tag}.`);
  try {
    const operation = core ? await nfqwsUpdates.installCore(tag) : await nfqwsUpdates.install(tag);
    let current = operation;
    for (let attempt = 0; attempt < 180 && !['success', 'rolled_back', 'failed'].includes(current.state); attempt += 1) {
      setText('#updateInstallStepText', labels[current.state] ?? 'Обновление…');
      setText('#updateInstallPercent', '');
      await new Promise((resolve) => window.setTimeout(resolve, 1000));
      try { current = await nfqwsUpdates.status(operation.id); }
      catch { await new Promise((resolve) => window.setTimeout(resolve, 1000)); }
    }
    if (current.state === 'success') schedulePanelRefreshAfterUpdate(`${core ? 'Ядро nfqws2' : 'Панель'} обновлено до ${tag}.`);
    else if (current.state === 'rolled_back') toast(current.message || 'Обновление не прошло проверку; восстановлена предыдущая версия.', 'error');
    else if (current.state === 'failed') toast(current.message || 'Не удалось установить обновление.', 'error');
    else toast('Статус обновления пока не получен. Проверьте панель через несколько секунд.', 'error');
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось запустить обновление.', 'error');
  } finally {
    updatePolling = false;
    renderReleases();
  }
}

function bindPrototypeActions() {
  const showDiff = () => {
    if (!config) return toast('Сначала загрузите конфигурацию backend.', 'error');
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
    switchMainTab: (route: Route) => showRoute(route),
    runSiteDiagnosis: () => void runSiteDiagnosis(),
    runSiteTrace: () => void runSiteTrace(),
    stopSiteTrace: () => void stopSiteTrace(),
    setDiagPreset: (host: string) => { const input = $('#diagDomainInput'); if (input instanceof HTMLInputElement && !diagnosisRunning) { input.value = host; input.focus(); } },
    restartService: () => void serviceAction('restart'),
    toggleDaemonPower: () => void serviceAction(status?.service.state === 'running' ? 'stop' : 'start'),
    applyConfigAndRestart: () => void saveConfigAndApply(),
    copyConfigText: () => { const editor = $('#nfqwsConfTextarea'); if (editor instanceof HTMLTextAreaElement) void navigator.clipboard.writeText(editor.value).then(() => toast('Конфигурация скопирована')).catch(() => toast('Буфер обмена недоступен', 'error')); },
    updateConfVariable: (key: string, value: string) => void updateConfigVariable(key, value),
    copySnippetToClipboard: (value: string) => void navigator.clipboard.writeText(String(value)).then(() => toast('Фрагмент скопирован')).catch(() => toast('Буфер обмена недоступен', 'error')),
    handleConfInput: () => syncConfigLines(),
    syncConfScroll: () => syncConfigLines(),
    toggleWordWrapConf: (enabled: boolean) => { const editor = $('#nfqwsConfTextarea'); if (editor instanceof HTMLTextAreaElement) editor.style.whiteSpace = enabled ? 'pre-wrap' : 'pre'; },
    scrollToConfMarker: (marker: string) => { const editor = $('#nfqwsConfTextarea'); if (!(editor instanceof HTMLTextAreaElement)) return; const offset = editor.value.toLowerCase().indexOf(String(marker).toLowerCase()); if (offset < 0) return toast(`Секция ${marker} не найдена`, 'error'); editor.scrollTop = editor.value.slice(0, offset).split('\n').length * 23; },
    toggleDiffModal: () => { const modal = $('#diffModalBackdrop'); if (modal && !modal.classList.contains('hidden')) { modal.classList.add('hidden'); modal.classList.remove('flex'); } else showDiff(); },
    resetDefaultConf: () => void loadConfig(),
    closeUpdateModal: () => { const modal = $('#updateModalBackdrop'); modal?.classList.add('hidden'); modal?.classList.remove('flex'); },
    openUpdateModal: (target: 'ui' | 'core') => void openUpdatePicker(target),
    selectVersionItem: (tag: string) => { if (availableReleases.some((release) => release.tag === tag) && !updatePolling) { selectedReleaseTag = tag; renderReleases(); } },
    installSelectedRelease: () => void installSelectedRelease(),
    switchUpdateTarget: (target: 'ui' | 'core') => { if (!updatePolling) void openUpdatePicker(target); },
    filterDocs: (query: string) => { const box = $('#docItemsContainer'); if (box) box.textContent = query ? 'Справочник флагов не подключён.' : 'Справочник флагов не загружен.'; },
    syncListLineNumbers: (editor?: HTMLTextAreaElement) => syncListLineNumbers(editor),
    syncListScroll: (editor: HTMLTextAreaElement) => syncListScroll(editor),
    syncCustomHostInput: () => undefined,
    selectListFile: (name: NfqwsListId) => void loadList(name),
    saveActiveListFile: () => void saveActiveList(),
    removeSelectedListLines: () => removeSelectedListLines(),
    findNextListEntry: () => findNextListEntry(),
    openListImport: () => $('#listImportFile')?.click(),
    importListDraft: () => void importListDraft(),
    exportListDraft: () => exportListDraft(),
    deduplicateCurrentList: () => {
      if (!activeList) return toast('Сначала загрузите список из backend.', 'error');
      const editor = $('#listEditorTextarea');
      if (!(editor instanceof HTMLTextAreaElement)) return;
      const result = deduplicateListContent(editor.value, activeList.id);
      if (!result.removed) return toast('Дубликаты не найдены.');
      editor.value = result.content;
      syncListLineNumbers(editor);
      toast(`Удалено дубликатов: ${result.removed}. Черновик изменён; нажмите «Сохранить» для записи.`);
    },
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
    if (control.dataset.args) {
      try {
        const descriptors = JSON.parse(control.dataset.args) as (string | { source: 'value' | 'checked' | 'self' })[];
        for (const descriptor of descriptors) {
          if (typeof descriptor === 'string') args.push(descriptor);
          else if (descriptor.source === 'value') args.push((control as HTMLInputElement | HTMLSelectElement).value);
          else if (descriptor.source === 'checked') args.push((control as HTMLInputElement).checked);
          else if (descriptor.source === 'self') args.push(control);
        }
      } catch { return; }
    } else {
      if (control.dataset.key !== undefined) args.push(control.dataset.key);
      if (control.dataset.valueSource === 'value') args.push((control as HTMLInputElement | HTMLSelectElement).value);
      else if (control.dataset.valueSource === 'checked') args.push((control as HTMLInputElement).checked);
      else if (control.dataset.valueSource === 'self') args.push(control);
      else if (control.dataset.arg !== undefined) args.push(control.dataset.arg);
      if (control.dataset.quoted !== undefined) args.push(control.dataset.quoted === 'true');
    }
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
  showPendingUpdateNotice();
  showRoute('conf');
  const applyButton = $('[data-action-click="applyConfigAndRestart"]');
  if (applyButton instanceof HTMLButtonElement) applyButton.disabled = true;
  $('#nfqwsConfTextarea')?.addEventListener('input', () => {
    syncConfigLines();
    const editor = $('#nfqwsConfTextarea');
    if (editor instanceof HTMLTextAreaElement) updateQuickParameters(editor.value);
  });
  $('#nfqwsConfTextarea')?.addEventListener('scroll', syncConfigLines);
  $('#refreshStatusBtn')?.addEventListener('click', () => void fetchStatus());
  void fetchStatus();
  void loadConfig();
  window.setInterval(() => { if (document.visibilityState === 'visible') void fetchStatus(); }, 3000);
  document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') void fetchStatus(); });
}

let booted = false;
export function bootPanel() {
  if (booted) return;
  booted = true;
  boot();
}
