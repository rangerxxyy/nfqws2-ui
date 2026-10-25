import { diagnosisRows } from './diagnosis-layout';
import { parseCustomStrategies } from './custom-strategy-input';
import { comparisonVerdict, presentTrace, type Presentation } from './diagnosis-presenter';
import { summarizeFreezeScan } from './freeze-summary';
import { freezeCanApplyToHost, freezeRowPresentation } from './freeze-presentation';
import { freezeProgressPresentation } from './freeze-progress';
import { uiUpdateNoticePresentation } from './ui-update-notice';
import { freezeRowDetails, freezeRowIdentity } from './freeze-row-text';
import { configAssignmentValue, configEditorLayout, configEditorViewportHeight, configOutlineEntries } from './config-values';
import '@fontsource-variable/inter';
import '@fontsource-variable/jetbrains-mono';
import './panel.css';
import { api, nfqwsApply, nfqwsSnapshots, nfqwsAppearance, nfqwsConfig, nfqwsDiagnostics, nfqwsInstallation, nfqwsLists, nfqwsScripts, nfqwsUpdates, nfqwsMonitoring, type ConfigResponse, type ConfigValidationResponse, type ForeignConfigCheckOperation, type FreezeScanOperation, type NfqwsListId, type NfqwsListResponse, type NfqwsListSummary, type NfqwsLuaScriptInfo, type PanelRelease, type SiteDiagnosticReport, type SiteTraceOperation, type SiteTraceReport, type SnapshotOverview, type StatusResponse, type StrategyPickOperation, type StrategyPickPreview, type UiUpdateStatus } from './api';
import { appendListEntries, deduplicateListContent, findNextListMatch, isListTabSelected, listIdFromTabElementId, listRowsFromContent, listTabIdsFromInventory, removeListLines, removeListLinesByIndices, toggleListComments } from './list-editor';
import { availableNfqwsModes } from './config-facts';
import { resourceUsagePercent, usageTone } from './metrics';
import { installStyledSelects } from './styled-select';
import { installActionTooltips } from './action-tooltips';
import { strategyNeedsWindowsTcpTimestamps } from './strategy-capability';
import { serviceControlPresentation } from './service-control';
import { renderReleaseMarkdown } from './release-markdown';
import { diagnosisQueueState, SITE_DIAGNOSIS_STAGES } from './diagnosis-stages';
import { canShowToast, lifecycleMessage, toastPresentation } from './notifications';
import { loadMonitoringState, normalizeMonitoringHost, normalizeMonitoringInterval, saveMonitoringState, type MonitoringState } from './monitoring';
import { applyAppearance, loadAppearance, saveAppearance, type AppearanceSettings, type UiLayout, type UiTheme } from './appearance';

type Route = 'conf' | 'lists' | 'check' | 'trace' | 'autotune' | 'diagnostic16k' | 'configCheck' | 'monitor' | 'community' | 'snapshots' | 'appearance' | 'about';
export const UNAVAILABLE_PANEL_ROUTES = ['community'] as const;
let status: StatusResponse | null = null;
let config: ConfigResponse | null = null;
let luaScripts: NfqwsLuaScriptInfo[] = [];
let activeLuaScript: string | null = null;
let discoveredNetworkInterfaces: string[] = [];
let activeList: NfqwsListResponse | null = null;
let activeListId: NfqwsListId = 'user.list';
const LIST_EDITOR_VIEW_KEY = 'nfqws2-ui-list-editor-view';
let listEditorView: 'text' | 'rows' = (() => {
  try { return localStorage.getItem(LIST_EDITOR_VIEW_KEY) === 'rows' ? 'rows' : 'text'; }
  catch { return 'text'; }
})();
const selectedListRows = new Set<number>();
let busy = false;
let statusPolling = false;
let diagnosisRunning = false;
let monitoringRunning = false;
let monitoringState: MonitoringState | null = null;
let monitoringIntervalMinutes = 30;
let appearanceSettings: AppearanceSettings = { theme: 'dark', layout: 'standard', accent: 'blue', statusBar: 'standard', remindWindowsTcpTimestamps: true };
let traceRunning = false;
let activeTraceOperationId: string | null = null;
let strategyPickRunning = false;
let activeStrategyPickId: string | null = null;
type ForeignConfigInput = { name: string; source: 'Текст' | 'Файл' | 'Ссылка'; content?: string; url?: string };
let foreignConfigInputs: ForeignConfigInput[] = [];
let activeForeignConfigCheckId: string | null = null;
let foreignConfigCheckRunning = false;
let freezeScanRunning = false;
let activeFreezeScanId: string | null = null;
const freezeStrategyApplyPending = new Set<string>();
const freezeStrategyApplyDone = new Set<string>();
let activeStrategyPreview: { source: 'strategyPick' | 'freezeScan'; operationId: string; candidateIndex: number; revision: string; panel: 'autotune' | 'freeze' } | null = null;
let restartInProgress = false;
let availableReleases: PanelRelease[] = [];
let selectedReleaseTag: string | null = null;
let expandedReleaseTag: string | null = null;
let updatePolling = false;
let updateTarget: 'ui' | 'core' = 'ui';
let installedCoreTag: string | null = null;
let uiUpdateStatusPolling = false;
let uiUpdateNoticeOpenedTag: string | null = null;
const UPDATE_NOTICE_KEY = 'nfqws2-ui-update-notice';
const UI_UPDATE_DISMISSED_TAG_KEY = 'nfqws2-ui-update-dismissed-tag';
const LIST_DISPLAY_NAMES: Partial<Record<NfqwsListId, string>> = {
  'user.list': 'Пользовательские домены',
  'auto.list': 'Домены для автообучения',
  'exclude.list': 'Исключённые домены',
  'ipset.list': 'IP-адреса',
  'ipset_exclude.list': 'Исключённые IP-адреса',
};
export const listDisplayName = (id: NfqwsListId) => LIST_DISPLAY_NAMES[id] ?? id;
const $ = <T extends HTMLElement>(selector: string) => document.querySelector<T>(selector);
const all = <T extends HTMLElement>(selector: string) => [...document.querySelectorAll<T>(selector)];
const setText = (selector: string, value: string) => { const node = $(selector); if (node) node.textContent = value; };
const setWidth = (selector: string, value: number | null) => { const node = $(selector); if (node) node.style.width = `${Math.min(100, Math.max(0, value ?? 0))}%`; };
const recentToastTimes = new Map<string, number>();
function toast(message: string, type: 'success' | 'error' = 'success') {
  const host = $('#toastWrapper'); if (!host) return;
  if (!message.trim() || !canShowToast(message, type, Date.now(), recentToastTimes)) return;
  const presentation = toastPresentation(type);
  const item = document.createElement('div');
  item.className = `toast-card toast-card-${type} pointer-events-auto`;
  item.setAttribute('role', presentation.role);
  item.setAttribute('aria-live', type === 'error' ? 'assertive' : 'polite');
  const icon = document.createElement('span');
  icon.className = `toast-card-icon toast-card-icon-${type}`;
  icon.setAttribute('aria-hidden', 'true');
  const iconSvg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  iconSvg.setAttribute('viewBox', '0 0 24 24');
  iconSvg.setAttribute('fill', 'none');
  iconSvg.setAttribute('stroke', 'currentColor');
  iconSvg.setAttribute('stroke-width', '2');
  iconSvg.setAttribute('stroke-linecap', 'round');
  iconSvg.setAttribute('stroke-linejoin', 'round');
  iconSvg.setAttribute('width', '20');
  iconSvg.setAttribute('height', '20');
  if (type === 'success') {
    // Match XKeen UI's Tabler IconCircleCheck instead of a font glyph inside a hand-drawn circle.
    const circle = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    circle.setAttribute('d', 'M3 12a9 9 0 1 0 18 0a9 9 0 1 0 -18 0');
    const check = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    check.setAttribute('d', 'M9 12l2 2l4 -4');
    iconSvg.append(circle, check);
  } else {
    // XKeen UI uses Tabler IconAlertCircle for error notifications.
    const circle = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    circle.setAttribute('d', 'M12 3a9 9 0 1 0 0 18a9 9 0 0 0 0 -18');
    const mark = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    mark.setAttribute('d', 'M12 8v4');
    const dot = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    dot.setAttribute('d', 'M12 16h.01');
    iconSvg.append(circle, mark, dot);
  }
  icon.append(iconSvg);
  const copy = document.createElement('span'); copy.className = 'toast-card-copy';
  const title = document.createElement('strong'); title.textContent = presentation.title;
  const detail = document.createElement('span'); detail.textContent = message;
  copy.append(title, detail); item.append(icon, copy); host.append(item);
  window.setTimeout(() => item.remove(), 5200);
}
function schedulePanelRefreshAfterUpdate(message: string) {
  try { window.sessionStorage.setItem(UPDATE_NOTICE_KEY, message); }
  catch { toast(`${message} Страница обновится автоматически.`); }
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
function hideUiUpdateNotice() { $('#uiUpdateNotice')?.remove(); }
function dismissUiUpdateNotice(tag: string) {
  try { window.localStorage.setItem(UI_UPDATE_DISMISSED_TAG_KEY, tag); } catch { /* Still allow the notice to be closed if storage is blocked. */ }
  hideUiUpdateNotice();
}
function renderUiUpdateNotice(status: UiUpdateStatus, manualCheck = false) {
  let dismissedTag: string | null = null;
  try { dismissedTag = window.localStorage.getItem(UI_UPDATE_DISMISSED_TAG_KEY); } catch { /* Treat storage failure as not dismissed. */ }
  const suppressedTag = status.latestVersion && ((dismissedTag === status.latestVersion && !manualCheck) || uiUpdateNoticeOpenedTag === status.latestVersion)
    ? status.latestVersion
    : null;
  const presentation = uiUpdateNoticePresentation(status, suppressedTag);
  const updateButton = $('#updateUiButton');
  updateButton?.classList.toggle('update-available', presentation.highlight);
  updateButton?.setAttribute('aria-label', presentation.highlight
    ? `Доступно обновление панели: ${presentation.version}`
    : `Версия панели ${status.currentVersion}`);
  if (!presentation.notify || !presentation.version || !status.releaseUrl) {
    hideUiUpdateNotice();
    return;
  }
  const host = $('#uiUpdateNoticeWrapper');
  if (!host || $('#uiUpdateNotice')?.getAttribute('data-version') === presentation.version) return;
  hideUiUpdateNotice();
  const item = document.createElement('section');
  item.id = 'uiUpdateNotice';
  item.dataset.version = presentation.version;
  item.className = 'ui-update-notice pointer-events-auto relative w-[22.5rem] max-w-[calc(100vw-2.5rem)] rounded-xl border px-4 py-3 text-slate-100 shadow-2xl';
  item.setAttribute('role', 'status');
  item.setAttribute('aria-live', 'polite');
  const heading = document.createElement('div');
  heading.className = 'flex min-w-0 items-center gap-2 text-sm font-bold text-blue-100';
  const icon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  icon.setAttribute('viewBox', '0 0 24 24');
  icon.setAttribute('fill', 'none');
  icon.setAttribute('stroke', 'currentColor');
  icon.setAttribute('stroke-width', '2');
  icon.setAttribute('stroke-linecap', 'round');
  icon.setAttribute('stroke-linejoin', 'round');
  icon.setAttribute('aria-hidden', 'true');
  icon.classList.add('h-[18px]', 'w-[18px]', 'shrink-0', 'text-slate-100');
  icon.innerHTML = '<path d="M3 12a9 9 0 1 0 18 0a9 9 0 1 0 -18 0"/><path d="M9 12l2 2l4 -4"/>';
  const title = document.createElement('span');
  title.textContent = 'Доступно обновление';
  const releaseLink = document.createElement('a');
  releaseLink.href = status.releaseUrl;
  releaseLink.target = '_blank';
  releaseLink.rel = 'noopener noreferrer';
  releaseLink.setAttribute('aria-label', 'Открыть релиз на GitHub');
  releaseLink.className = 'inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-slate-300 transition-colors hover:bg-slate-700 hover:text-white';
  releaseLink.innerHTML = '<svg aria-hidden="true" viewBox="0 0 16 16" fill="currentColor" class="h-4 w-4"><path d="M8 0C3.58 0 0 3.58 0 8a8 8 0 0 0 5.47 7.59c.4.07.55-.17.55-.38v-1.33c-2.23.49-2.7-.95-2.7-.95-.36-.93-.89-1.18-.89-1.18-.73-.5.06-.49.06-.49.8.06 1.22.82 1.22.82.72 1.22 1.9.87 2.36.66.07-.52.28-.87.51-1.07-1.78-.2-3.65-.89-3.65-3.96 0-.88.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82A7.7 7.7 0 0 1 8 4.99c.68 0 1.36.09 2 .27 1.53-1.03 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.08-1.87 3.75-3.66 3.95.29.25.54.73.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0 0 16 8c0-4.42-3.58-8-8-8Z"/></svg>';
  const actions = document.createElement('span');
  actions.className = 'ml-auto flex shrink-0 items-center gap-1';
  actions.append(releaseLink);
  const body = document.createElement('button');
  body.type = 'button';
  body.dataset.actionClick = 'openLatestUiUpdate';
  body.dataset.args = JSON.stringify([presentation.version]);
  body.className = 'mt-1 block w-full text-left text-sm leading-relaxed text-slate-300 hover:text-white';
  body.textContent = `Доступна новая версия nfqws2-UI ${presentation.version}`;
  const close = document.createElement('button');
  close.type = 'button';
  close.dataset.actionClick = 'dismissUiUpdateNotice';
  close.dataset.args = JSON.stringify([presentation.version]);
  close.className = 'inline-flex h-7 w-7 items-center justify-center rounded-md text-[18px] leading-none text-slate-400 transition-colors hover:bg-slate-700/70 hover:text-white';
  close.setAttribute('aria-label', 'Скрыть уведомление об обновлении');
  close.textContent = '×';
  actions.append(close);
  heading.append(icon, title, actions);
  item.append(heading, body);
  host.append(item);
}
async function refreshUiUpdateStatus() {
  if (uiUpdateStatusPolling) return;
  uiUpdateStatusPolling = true;
  try { renderUiUpdateNotice(await nfqwsUpdates.uiStatus()); }
  catch { /* Background update checks must not interrupt panel use. */ }
  finally { uiUpdateStatusPolling = false; }
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
  all<HTMLTextAreaElement>('textarea[data-config-argument]').forEach((field) => {
    if (value && !field.disabled) { field.disabled = true; field.dataset.busyDisabled = 'true'; }
    else if (!value && field.dataset.busyDisabled === 'true') { field.disabled = false; delete field.dataset.busyDisabled; }
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
  if (route === 'snapshots') void loadBackups();
  if (route === 'about') void loadAboutStatus();
}
let snapshotOverview: SnapshotOverview | null = null;
function renderBackups(data: SnapshotOverview) {
  const host = $('#backupList'); if (!host) return;
  snapshotOverview = data;
  setText('#snapshotCount', `${data.localCount} ${pluralize(data.localCount, 'снимок', 'снимка', 'снимков')} · ${formatBytes(data.totalBytes)}`);
  setText('#snapshotDirectory', data.directory);
  setText('#snapshotRemoteStatus', !data.remote.tool ? 'На роутере нет SSH-клиента' : data.remote.error ? `Ошибка отправки: ${data.remote.error}` : data.remote.on ? `Включено · на NAS ${data.remote.stored} · ждут отправки ${data.remote.pending}` : 'Отправка выключена — снимки хранятся на роутере');
  setValue('#snapshotDays', String(data.settings.maxDays)); setValue('#snapshotMaxCount', String(data.settings.maxCount));
  setValue('#snapshotRemoteEnabled', String(data.settings.remote.enabled), true); setValue('#snapshotRemoteHost', data.settings.remote.host); setValue('#snapshotRemotePort', String(data.settings.remote.port)); setValue('#snapshotRemoteUser', data.settings.remote.user); setValue('#snapshotRemoteDir', data.settings.remote.dir); setValue('#snapshotRemoteKeep', String(data.settings.remote.keepLocal));
  setText('#snapshotPublicKey', data.remote.publicKey);
  const command = $('#snapshotSetupCommand'); if (command instanceof HTMLTextAreaElement) command.value = data.remote.setupCommand;
  $('#snapshotSetupDetails')?.classList.toggle('hidden', !data.remote.setupCommand);
  const keyButton = $('#snapshotCreateKey'); if (keyButton instanceof HTMLButtonElement) { keyButton.textContent = data.remote.publicKey ? 'Ключ роутера готов' : '＋ Создать ключ роутера'; keyButton.disabled = Boolean(data.remote.publicKey) || !data.remote.tool; }
  const pushButton = $('#snapshotRemotePush'); if (pushButton instanceof HTMLButtonElement) pushButton.disabled = !data.remote.on || data.remote.pending === 0;
  const testButton = $('#snapshotRemoteTest'); if (testButton instanceof HTMLButtonElement) testButton.disabled = !data.remote.tool || !data.remote.publicKey;
  host.replaceChildren();
  if (!data.items.length) { const empty = document.createElement('p'); empty.className = 'rounded-xl border border-slate-800 bg-[#060a12] p-4 text-sm text-slate-400'; empty.textContent = 'Снимков пока нет. Создайте первый снимок вручную.'; host.append(empty); return; }
  for (const item of data.items) {
    const row = document.createElement('article'); row.className = 'backup-row';
    const details = document.createElement('div'); details.className = 'min-w-0';
    const title = document.createElement('strong'); title.className = 'block text-sm text-white'; title.textContent = `${snapshotReason(item.reason)}${item.pinned ? ' · закреплён' : ''}`;
    const date = document.createElement('span'); date.className = 'mt-1 block text-xs text-slate-400'; date.textContent = new Date(item.ts * 1000).toLocaleString();
    const note = document.createElement('span'); note.className = 'mt-1 block break-words text-xs text-slate-500'; note.textContent = item.note || `${item.local && item.remote ? 'Роутер и NAS' : item.local ? 'На роутере' : 'На NAS'} · ${formatBytes(item.size)}`;
    details.append(title, date, note);
    const actions = document.createElement('div'); actions.className = 'flex flex-wrap gap-2';
    const download = document.createElement('a'); download.className = 'backup-action'; download.href = `/api/snapshots/${encodeURIComponent(item.id)}/archive`; download.textContent = 'Скачать';
    const restore = document.createElement('button'); restore.type = 'button'; restore.className = 'backup-action'; restore.dataset.actionClick = 'restoreSnapshot'; restore.dataset.args = JSON.stringify([item.id]); restore.textContent = 'Восстановить';
    const pin = document.createElement('button'); pin.type = 'button'; pin.className = 'backup-action'; pin.dataset.actionClick = 'pinSnapshot'; pin.dataset.args = JSON.stringify([item.id, !item.pinned]); pin.textContent = item.pinned ? 'Открепить' : 'Закрепить';
    pin.disabled = !item.local;
    actions.append(download, restore, pin); row.append(details, actions); host.append(row);
  }
}
async function loadBackups() {
  const label = $('#backupStatus'); if (label) label.textContent = 'Загружаем список снимков с роутера…';
  try { renderBackups(await nfqwsSnapshots.get()); if (label) label.textContent = 'Автоснимки создаются перед изменениями через панель и ежедневно в 04:05.'; }
  catch (error) { if (label) label.textContent = error instanceof Error ? `Не удалось загрузить снимки: ${error.message}` : 'Не удалось загрузить резервные копии.'; }
}
function setValue(selector: string, value: string, checked = false) { const element = $(selector); if (element instanceof HTMLInputElement) { if (checked) element.checked = value === 'true'; else element.value = value; } }
function formatBytes(value: number): string { if (value < 1024) return `${value} Б`; if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} КБ`; return `${(value / (1024 * 1024)).toFixed(1)} МБ`; }
function pluralize(value: number, one: string, few: string, many: string): string { const n = Math.abs(value) % 100; const last = n % 10; return n > 10 && n < 20 ? many : last > 1 && last < 5 ? few : last === 1 ? one : many; }
function snapshotReason(reason: string): string { return ({ 'вручную': 'Вручную', 'перед изменением': 'Перед изменением', 'ежедневный': 'Ежедневный', 'перед восстановлением': 'Перед восстановлением', 'из файла': 'Из файла' } as Record<string, string>)[reason] ?? reason; }
async function createSnapshot() {
  const note = window.prompt('Подпись к снимку (необязательно):', ''); if (note === null) return;
  try { const item = await nfqwsSnapshots.create(note); toast(`Снимок создан · ${new Date(item.ts * 1000).toLocaleString()}`); await loadBackups(); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось создать снимок.', 'error'); }
}
async function restoreSnapshot(id: string) {
  if (!window.confirm('Восстановить конфигурацию, списки и настройки из этого снимка? Перед записью сохранится текущее состояние.')) return;
  try { const result = await nfqwsSnapshots.restore(id); toast(result.message); await Promise.all([loadBackups(), loadConfig(), fetchStatus(), syncAppearanceFromRouter()]); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось восстановить снимок.', 'error'); }
}
async function restoreSnapshotFile(input: HTMLInputElement) {
  const file = input.files?.[0]; if (!file) return;
  if (!window.confirm(`Восстановить состояние из «${file.name}»? Перед записью создастся снимок текущего состояния.`)) { input.value = ''; return; }
  try { const result = await nfqwsSnapshots.restoreFile(file); toast(result.message ?? `Восстановлено файлов: ${result.changed?.length ?? 0}`); await Promise.all([loadBackups(), loadConfig(), fetchStatus(), syncAppearanceFromRouter()]); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось восстановить архив.', 'error'); }
  finally { input.value = ''; }
}
function readSnapshotSettings() {
  const current = snapshotOverview?.settings; if (!current) throw new Error('Сначала загрузите настройки снимков.');
  const number = (selector: string, fallback: number) => Number.parseInt(($<HTMLInputElement>(selector)?.value ?? ''), 10) || fallback;
  return { ...current, maxDays: number('#snapshotDays', current.maxDays), maxCount: number('#snapshotMaxCount', current.maxCount), remote: { ...current.remote, enabled: ($<HTMLInputElement>('#snapshotRemoteEnabled')?.checked ?? false), host: $<HTMLInputElement>('#snapshotRemoteHost')?.value.trim() ?? '', port: number('#snapshotRemotePort', current.remote.port), user: $<HTMLInputElement>('#snapshotRemoteUser')?.value.trim() ?? '', dir: $<HTMLInputElement>('#snapshotRemoteDir')?.value.trim() ?? '', keepLocal: number('#snapshotRemoteKeep', current.remote.keepLocal) } };
}
async function saveSnapshotSettings(remoteOnly = false) {
  const settings = readSnapshotSettings();
  if (remoteOnly && snapshotOverview) { settings.maxDays = snapshotOverview.settings.maxDays; settings.maxCount = snapshotOverview.settings.maxCount; }
  await nfqwsSnapshots.saveSettings(settings); toast('Настройки резервных копий сохранены.'); await loadBackups();
}
async function runSnapshotRemote(action: 'test' | 'push' | 'key') {
  try {
    if (action !== 'key') await saveSnapshotSettings(true);
    if (action === 'test') { const result = await nfqwsSnapshots.testRemote(); toast(`Связь с NAS есть · найдено файлов: ${result.count}`); }
    else if (action === 'push') { const result = await nfqwsSnapshots.pushRemote(); toast(`Отправлено снимков: ${result.sent}`); }
    else { await nfqwsSnapshots.createKey(); toast('Ключ создан. Выполните команду для NAS, затем проверьте связь.'); }
    await loadBackups();
  } catch (error) { toast(error instanceof Error ? error.message : 'Операция с NAS не выполнена.', 'error'); }
}
function syncAppearanceControls() {
  all<HTMLInputElement>('input[name="appearanceTheme"]').forEach((input) => { input.checked = input.value === appearanceSettings.theme; });
  all<HTMLInputElement>('input[name="appearanceLayout"]').forEach((input) => { input.checked = input.value === appearanceSettings.layout; });
  all<HTMLInputElement>('input[name="appearanceAccent"]').forEach((input) => { input.checked = input.value === appearanceSettings.accent; });
  all<HTMLInputElement>('input[name="appearanceStatusBar"]').forEach((input) => { input.checked = input.value === appearanceSettings.statusBar; });
  all<HTMLInputElement>('[data-appearance-remind-timestamps]').forEach((input) => { input.checked = appearanceSettings.remindWindowsTcpTimestamps; });
}
function updateAppearance(change: Partial<AppearanceSettings>) {
  let storage: Storage | null = null; try { storage = window.localStorage; } catch { /* Apply for this session without persistent storage. */ }
  appearanceSettings = saveAppearance(storage, { ...appearanceSettings, ...change });
  applyAppearance(appearanceSettings);
  syncAppearanceControls();
  void nfqwsAppearance.save(appearanceSettings).catch((error) => toast(error instanceof Error ? `Оформление сохранено только в этом браузере: ${error.message}` : 'Оформление сохранено только в этом браузере.', 'error'));
}
async function syncAppearanceFromRouter() {
  const saved = await nfqwsAppearance.get(); if (!saved) return;
  let storage: Storage | null = null; try { storage = window.localStorage; } catch { /* Use the restored settings for this session. */ }
  appearanceSettings = saveAppearance(storage, saved); applyAppearance(appearanceSettings); syncAppearanceControls();
}
function initializeAppearanceSettings() {
  let storage: Storage | null = null;
  try { storage = window.localStorage; } catch { /* Use safe defaults if browser storage is blocked. */ }
  appearanceSettings = loadAppearance(storage);
  applyAppearance(appearanceSettings);
  syncAppearanceControls();
  void nfqwsAppearance.get().then((saved) => {
    if (!saved) { void nfqwsAppearance.save(appearanceSettings).catch(() => undefined); return; }
    appearanceSettings = saveAppearance(storage, saved); applyAppearance(appearanceSettings); syncAppearanceControls();
  }).catch(() => undefined);
}
async function loadAboutStatus(statusOverride?: UiUpdateStatus) {
  try {
    const result = statusOverride ?? await nfqwsUpdates.uiStatus();
    setText('#aboutCurrentVersion', result.currentVersion);
    setText('#aboutLatestVersion', result.latestVersion ?? 'не удалось определить');
    setText('#aboutUpdateStatus', result.updateAvailable ? `Доступна версия ${result.latestVersion}. Обновите панель через кнопку обновления в верхней панели.` : result.latestVersion ? `Установлена актуальная версия ${result.currentVersion}.` : 'Последнюю версию пока не удалось определить. Проверьте соединение с GitHub.');
  } catch (error) { setText('#aboutUpdateStatus', error instanceof Error ? error.message : 'Не удалось загрузить сведения об обновлении.'); }
}
async function checkUiUpdate() {
  const button = $('[data-action-click="checkUiUpdate"]'); if (button instanceof HTMLButtonElement) { button.disabled = true; button.textContent = 'Проверяем…'; }
  setText('#aboutUpdateStatus', 'Запрашиваем список релизов GitHub…');
  try { const response = await nfqwsUpdates.releases(); await loadAboutStatus(response.updateStatus); renderUiUpdateNotice(response.updateStatus, true); }
  catch (error) { setText('#aboutUpdateStatus', error instanceof Error ? `Проверка не удалась: ${error.message}` : 'Не удалось проверить обновления.'); }
  finally { if (button instanceof HTMLButtonElement) { button.disabled = false; button.textContent = '↻ Проверить обновление'; } }
}
function renderMonitoringSites() {
  const host = $('#monitorSites'); if (!host || !monitoringState) return;
  host.replaceChildren();
  if (!monitoringState.sites.length) {
    const empty = document.createElement('p'); empty.className = 'rounded-xl border border-slate-800 bg-[#060a12] p-4 text-sm text-slate-400'; empty.textContent = 'Добавьте домены, которые хотите проверять вручную.'; host.append(empty); return;
  }
  for (const site of monitoringState.sites) {
    const card = document.createElement('article'); card.className = 'monitor-site-card';
    const heading = document.createElement('div'); heading.className = 'monitor-site-heading';
    const name = document.createElement('strong'); name.className = 'font-mono text-sm text-white'; name.textContent = site.host;
    const latest = site.results[0];
    const summary = document.createElement('span'); summary.className = `monitor-site-status monitor-status-${latest?.status ?? 'unavailable'}`;
    summary.textContent = latest ? `${latest.status === 'ok' ? 'Доступен' : latest.status === 'error' ? 'Ошибка' : latest.status === 'warning' ? 'Есть предупреждение' : 'Нет результата'}${latest.latencyMs === null ? '' : ` · ${latest.latencyMs} мс`}` : 'Ещё не проверялся';
    heading.append(name, summary);
    const check = document.createElement('button'); check.type = 'button'; check.className = 'monitor-row-action'; check.dataset.actionClick = 'checkMonitoringSite'; check.dataset.args = JSON.stringify([site.host]); check.textContent = '⟳ Проверить'; check.title = `Проверить ${site.host}`;
    const remove = document.createElement('button'); remove.type = 'button'; remove.className = 'monitor-remove'; remove.dataset.actionClick = 'removeMonitoringSite'; remove.dataset.args = JSON.stringify([site.host]); remove.textContent = '×'; remove.title = `Убрать ${site.host} из мониторинга`;
    heading.append(check, remove); card.append(heading);
    if (latest) {
      const bars = document.createElement('div'); bars.className = 'monitor-history-bars'; bars.setAttribute('aria-label', `История: ${site.results.length} циклов проверки`);
      for (const result of [...site.results].slice(0, 48).reverse()) { const bar = document.createElement('span'); bar.dataset.status = result.status; bar.title = `${new Date(result.checkedAt).toLocaleString()}: ${result.summary}`; bars.append(bar); }
      card.append(bars);
      const detail = document.createElement('p'); detail.className = 'mt-2 text-xs text-slate-400'; detail.textContent = `${new Date(latest.checkedAt).toLocaleString()} · ${latest.summary}`; card.append(detail);
      if (latest.steps.length) {
        const labels: Record<string, string> = { dns: 'DNS', tcp: 'TCP connect', tls: 'TLS', http: 'HTTP', transfer: 'Передача' };
        const stageDetails = document.createElement('details'); stageDetails.className = 'monitor-stage-list mt-2';
        const stageSummary = document.createElement('summary'); stageSummary.textContent = `Этапы последнего цикла · ${latest.steps.filter((step) => step.status === 'ok').length}/${latest.steps.length} успешно`;
        const stageList = document.createElement('div'); stageList.className = 'mt-2 space-y-1.5';
        for (const step of latest.steps) {
          const row = document.createElement('div'); row.className = 'monitor-stage-row';
          const stageName = document.createElement('strong'); stageName.textContent = labels[step.id] ?? step.id;
          const stageInfo = document.createElement('span'); stageInfo.textContent = `${step.detail}${step.durationMs === null ? '' : ` · ${step.durationMs} мс`}`;
          row.dataset.status = step.status;
          row.append(stageName, stageInfo); stageList.append(row);
        }
        stageDetails.append(stageSummary, stageList); card.append(stageDetails);
      }
    }
    if (site.results.length) { const history = document.createElement('p'); history.className = 'mt-1 text-[11px] text-slate-500'; history.textContent = `Проверок в истории: ${site.results.length} · доступность: ${Math.round(site.results.filter((r) => r.status === 'ok').length * 100 / site.results.length)}%`; card.append(history); }
    host.append(card);
  }
}
function getMonitoringStorage(): Storage | null { try { return window.localStorage; } catch { return null; } }
function currentMonitoringInterval() {
  const choice = ($('#monitoringInterval') as HTMLSelectElement | null)?.value ?? String(monitoringIntervalMinutes);
  if (choice !== 'custom') return Number(choice) || 30;
  const hours = Number(($('#monitoringHours') as HTMLInputElement | null)?.value ?? 0);
  const minutes = Number(($('#monitoringMinutes') as HTMLInputElement | null)?.value ?? 30);
  return normalizeMonitoringInterval(hours, minutes);
}
function persistMonitoring() {
  if (!monitoringState) return;
  saveMonitoringState(getMonitoringStorage(), monitoringState); renderMonitoringSites();
  const enabled = ($('#monitoringEnabled') as HTMLInputElement | null)?.checked ?? true;
  void nfqwsMonitoring.save({ enabled, intervalMinutes: monitoringIntervalMinutes, sites: monitoringState.sites.map((site) => ({ host: site.host, results: site.results })) }).catch((e) => toast(e instanceof Error ? e.message : 'Не удалось сохранить мониторинг на роутере.', 'error'));
}
async function loadBackendMonitoring() {
  const saved = await nfqwsMonitoring.get();
  monitoringIntervalMinutes = saved.intervalMinutes;
  const select = $('#monitoringInterval') as HTMLSelectElement | null; const custom = $('#monitoringCustom');
  const quick = [30, 60, 120].includes(saved.intervalMinutes);
  if (select) select.value = quick ? String(saved.intervalMinutes) : 'custom';
  if (custom) custom.classList.toggle('hidden', quick);
  if (!quick) { const hours = $('#monitoringHours') as HTMLInputElement | null; const minutes = $('#monitoringMinutes') as HTMLInputElement | null; if (hours) hours.value = String(Math.floor(saved.intervalMinutes / 60)); if (minutes) minutes.value = String(saved.intervalMinutes % 60); }
  const enabled = $('#monitoringEnabled') as HTMLInputElement | null; if (enabled) enabled.checked = saved.enabled;
  const legacy = monitoringState?.sites ?? [];
  monitoringState = { version: 1, sites: (saved.sites.length ? saved.sites : legacy).map((site) => ({ host: site.host, results: site.results.slice(0, 96) })) };
  if (!saved.sites.length && legacy.length) await nfqwsMonitoring.save({ enabled: saved.enabled, intervalMinutes: saved.intervalMinutes, sites: monitoringState.sites });
  renderMonitoringSites();
  const last = saved.lastAutoCheck ? new Date(saved.lastAutoCheck * 1000).toLocaleString() : 'ещё не запускалась';
  setText('#monitoringStatus', `Последняя автоматическая проверка: ${last}. Интервал: ${saved.intervalMinutes} мин.`);
}
function showMonitorSection(section: 'sites' | 'telegram') {
  $('#monitorSection-sites')?.classList.toggle('hidden', section !== 'sites');
  $('#monitorSection-telegram')?.classList.toggle('hidden', section !== 'telegram');
  all<HTMLButtonElement>('.monitor-subtab').forEach((button) => button.classList.toggle('active', button.textContent?.includes(section === 'sites' ? 'Мониторинг' : 'Telegram') ?? false));
  if (section === 'telegram') void loadTelegramNotifications().catch(() => {});
}
async function saveMonitoringSettings() {
  if (!monitoringState) return;
  monitoringIntervalMinutes = currentMonitoringInterval();
  await nfqwsMonitoring.save({ enabled: ($('#monitoringEnabled') as HTMLInputElement | null)?.checked ?? false, intervalMinutes: monitoringIntervalMinutes, sites: monitoringState.sites.map((site) => ({ host: site.host, results: site.results })) });
  toast('Настройки мониторинга сохранены на роутере.');
}
async function loadTelegramSettings() {
  const cfg = await nfqwsMonitoring.telegram();
  const via = $('#telegramVia') as HTMLSelectElement | null; if (via) via.value = cfg.sendVia;
  toggleTelegramTransportFields(cfg.sendVia);
  const iface = $('#telegramInterface') as HTMLInputElement | null; if (iface) iface.value = cfg.interface;
  const proxy = $('#telegramProxy') as HTMLInputElement | null; if (proxy) proxy.value = cfg.proxy;
  const chat = $('#telegramChatId') as HTMLInputElement | null; if (chat) chat.value = cfg.chatId;
  setText('#telegramStatus', cfg.tokenSet ? 'Токен бота сохранён на роутере.' : 'Токен ещё не задан.');
  renderTelegramNotifications(cfg.notifications);
}
function toggleTelegramTransportFields(via: string) {
  $('#telegramInterfaceField')?.classList.toggle('hidden', via !== 'interface');
  $('#telegramProxyField')?.classList.toggle('hidden', via !== 'proxy');
}
async function loadTelegramNotifications() {
  renderTelegramNotifications(await nfqwsMonitoring.telegramNotifications());
}
function renderTelegramNotifications(notifications: { queue: Array<{ timestamp: number; text: string; tries: number; error?: string | null }>; log: Array<{ timestamp: number; eventTimestamp: number; text: string; ok: boolean; error?: string | null }> }) {
  const host = $('#telegramNotifications'); if (!host) return;
  host.replaceChildren();
  if (!notifications.queue.length && !notifications.log.length) return;
  const heading = document.createElement('h3'); heading.className = 'text-sm font-semibold text-white'; heading.textContent = 'Последние сообщения'; host.append(heading);
  const date = (seconds: number) => new Date(seconds * 1000).toLocaleString();
  for (const message of notifications.queue) {
    const row = document.createElement('article'); row.className = 'rounded-lg border border-amber-500/30 bg-amber-500/5 p-3';
    const status = document.createElement('strong'); status.className = 'text-sm text-amber-300'; status.textContent = `Ждёт отправки · ${date(message.timestamp)}`;
    const body = document.createElement('p'); body.className = 'mt-1 whitespace-pre-wrap break-words text-sm text-slate-200'; body.textContent = message.text;
    const detail = document.createElement('p'); detail.className = 'mt-1 text-xs text-slate-400'; detail.textContent = `Попыток: ${message.tries}. ${message.error || 'Повторная отправка будет выполнена автоматически, срок хранения — 24 часа.'}`;
    row.append(status, body, detail); host.append(row);
  }
  for (const message of notifications.log.slice(0, 20)) {
    const row = document.createElement('article'); row.className = `rounded-lg border p-3 ${message.ok ? 'border-emerald-500/25 bg-emerald-500/5' : 'border-rose-500/25 bg-rose-500/5'}`;
    const status = document.createElement('strong'); status.className = `text-sm ${message.ok ? 'text-emerald-300' : 'text-rose-300'}`; status.textContent = `${message.ok ? 'Отправлено' : 'Не отправлено'} · ${date(message.eventTimestamp)}`;
    const body = document.createElement('p'); body.className = 'mt-1 whitespace-pre-wrap break-words text-sm text-slate-200'; body.textContent = message.text;
    row.append(status, body);
    if (message.error) { const detail = document.createElement('p'); detail.className = 'mt-1 text-xs text-rose-300'; detail.textContent = message.error; row.append(detail); }
    if (message.ok && message.timestamp - message.eventTimestamp > 120) { const detail = document.createElement('p'); detail.className = 'mt-1 text-xs text-slate-400'; detail.textContent = `Доставлено с задержкой · ${date(message.timestamp)}`; row.append(detail); }
    host.append(row);
  }
}
async function saveTelegramSettings() {
  const token = ($('#telegramToken') as HTMLInputElement | null)?.value ?? '';
  const result = await nfqwsMonitoring.saveTelegram({ ...(token ? { token } : {}), chatId: ($('#telegramChatId') as HTMLInputElement | null)?.value ?? '', sendVia: ($('#telegramVia') as HTMLSelectElement | null)?.value ?? 'direct', interface: ($('#telegramInterface') as HTMLInputElement | null)?.value ?? '', proxy: ($('#telegramProxy') as HTMLInputElement | null)?.value ?? '' });
  const field = $('#telegramToken') as HTMLInputElement | null; if (field) field.value = '';
  setText('#telegramStatus', result.tokenSet ? 'Настройки и токен сохранены на роутере.' : 'Настройки сохранены. Укажите токен бота.'); toast('Настройки Telegram сохранены.');
}
async function findTelegramChatId() {
  await saveTelegramSettings();
  const result = await nfqwsMonitoring.findTelegramChats();
  if (!result.chats.length) return toast('Чаты не найдены. Напишите боту любое сообщение и нажмите «Найти» ещё раз.', 'error');
  const chat = result.chats[0]; const field = $('#telegramChatId') as HTMLInputElement | null; if (field) field.value = chat.id;
  setText('#telegramStatus', `Найден чат: ${chat.title} (${chat.id}). Сохраните настройки.`);
}
async function sendTestTelegram() { await saveTelegramSettings(); await nfqwsMonitoring.testTelegram(); toast('Проверочное сообщение отправлено в Telegram.'); }
async function runMonitoring(host?: string) {
  if (monitoringRunning || !monitoringState) return;
  const hosts = host ? monitoringState.sites.filter((site) => site.host === host).map((site) => site.host) : monitoringState.sites.map((site) => site.host);
  if (!hosts.length) return toast('Сначала добавьте домен для проверки.', 'error');
  monitoringRunning = true;
  const button = $('[data-action-click="runMonitoring"]'); if (button instanceof HTMLButtonElement) { button.disabled = true; button.textContent = 'Проверяем…'; }
  const statusNode = $('#monitoringStatus'); if (statusNode) { statusNode.textContent = `Проверка 0 из ${hosts.length}`; statusNode.setAttribute('aria-busy', 'true'); }
  try {
    if (statusNode) statusNode.textContent = `Проверка ${hosts.length} сайтов параллельно: DNS → TCP → TLS → HTTP и до 64 КиБ`;
    await nfqwsMonitoring.run(hosts);
    await loadBackendMonitoring();
    toast(`Мониторинг завершён: ${hosts.length} сайтов проверено.`);
  } finally {
    monitoringRunning = false;
    if (button instanceof HTMLButtonElement) { button.disabled = false; button.textContent = 'Проверить сайты'; }
    if (statusNode) { statusNode.textContent = 'Проверка завершена'; statusNode.removeAttribute('aria-busy'); }
  }
}
function addMonitoringSite() {
  const input = $('#monitorHostInput');
  if (!(input instanceof HTMLInputElement) || !monitoringState) return;
  const host = normalizeMonitoringHost(input.value);
  if (!host) return toast('Введите доменное имя без https://, пути или IP-адреса.', 'error');
  if (monitoringState.sites.some((site) => site.host === host)) return toast('Этот домен уже добавлен.', 'error');
  if (monitoringState.sites.length >= 20) return toast('Можно добавить не более 20 доменов.', 'error');
  monitoringState = { ...monitoringState, sites: [{ host, results: [] }, ...monitoringState.sites] };
  input.value = ''; persistMonitoring();
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
    syncServiceControl(service);
    setText('#checkedAt', status.service.checkedAt ? new Date(status.service.checkedAt * 1000).toLocaleTimeString() : '—');
    setText('#statusDetail', `Опрос: ${status.service.checkedAt ? new Date(status.service.checkedAt * 1000).toLocaleTimeString() : 'выполняется'} · метрики: ${status.system.source === 'keenetic-rci' ? 'Keenetic RCI' : 'Linux'}`);
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
    syncServiceControl('unknown');
    const detail = $('#statusDetail'); if (detail) detail.textContent = error instanceof Error ? error.message : 'Ошибка проверки статуса';
  } finally { statusPolling = false; }
}

function syncServiceControl(state: string) {
  const presentation = serviceControlPresentation(state);
  const button = $('#togglePowerBtn');
  if (!button) return;
  const label = $('#togglePowerText'); if (label) label.textContent = presentation.label;
  button.setAttribute('aria-label', presentation.label);
  button.dataset.actionTooltip = presentation.label;
  button.dataset.serviceAction = presentation.action;
  button.querySelector<SVGElement>('[data-power-icon="play"]')?.classList.toggle('hidden', presentation.icon !== 'play');
  button.querySelector<SVGElement>('[data-power-icon="stop"]')?.classList.toggle('hidden', presentation.icon !== 'stop');
  const tones = {
    success: ['text-emerald-400', 'bg-emerald-500/10', 'hover:bg-emerald-500/20', 'border-emerald-500/30', 'focus-visible:ring-emerald-400'],
    danger: ['text-rose-400', 'bg-rose-500/10', 'hover:bg-rose-500/20', 'border-rose-500/30', 'focus-visible:ring-rose-400'],
    neutral: ['text-slate-400', 'bg-slate-500/10', 'hover:bg-slate-500/20', 'border-slate-500/30', 'focus-visible:ring-slate-400'],
  } as const;
  for (const tone of Object.values(tones)) for (const className of tone) button.classList.toggle(className, tone === tones[presentation.tone]);
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
    let networkInterfaces: string[] = [];
    try {
      const installation = await nfqwsInstallation.get();
      canWriteConfig = installation.config?.writable === true;
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
    renderNfqwsArgumentEditors(config.content);
    updateQuickParameters(config.content, networkInterfaces);
    const queue = configAssignmentValue(config.content, 'NFQUEUE_NUM');
    setText('#queueBadge', queue ? `Q:${queue.replace(/^['"]|['"]$/g, '')}` : 'Q:—');
    syncConfigLines();
    renderConfigOutline();
  } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось прочитать конфигурацию', 'error'); }
}
function switchConfigSection(section: 'editor' | 'settings' | 'scripts') {
  const selectedId = { editor: 'configEditorPanel', settings: 'configSettingsPanel', scripts: 'configScriptsPanel' }[section];
  for (const id of ['configEditorPanel', 'configSettingsPanel', 'configScriptsPanel']) {
    const panel = document.getElementById(id);
    if (!panel) continue;
    panel.classList.toggle('hidden', id !== selectedId);
  }
  const activePanel = document.getElementById(selectedId);
  if (!activePanel) return;
  activePanel.classList.remove('config-panel-enter');
  void activePanel.offsetWidth;
  activePanel.classList.add('config-panel-enter');
  for (const [id, selected] of [['configTabEditor', section === 'editor'], ['configTabSettings', section === 'settings'], ['configTabScripts', section === 'scripts']] as const) {
    const button = document.getElementById(id);
    if (!button) continue;
    button.setAttribute('aria-pressed', String(selected));
    button.classList.toggle('config-subtab-active', selected);
  }
  if (section === 'scripts') void loadLuaScripts();
}

function renderLuaScriptTabs() {
  const host = $('#luaScriptTabs');
  if (!host) return;
  host.replaceChildren();
  for (const script of luaScripts) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = `lua-script-tab px-3 py-2 text-xs font-medium transition-colors ${script.name === activeLuaScript ? 'lua-script-tab-active text-cyan-200' : 'text-slate-400 hover:text-slate-100'}`;
    button.setAttribute('aria-pressed', String(script.name === activeLuaScript));
    button.textContent = script.name;
    button.addEventListener('click', () => void showLuaScript(script.name));
    host.append(button);
  }
}

async function loadLuaScripts() {
  const host = $('#luaScriptTabs');
  if (!host) return;
  host.textContent = 'Читаю список скриптов…';
  try {
    luaScripts = await nfqwsScripts.list();
    renderLuaScriptTabs();
    const selected = activeLuaScript && luaScripts.some((script) => script.name === activeLuaScript)
      ? activeLuaScript
      : luaScripts.find((script) => script.exists)?.name ?? luaScripts[0]?.name;
    if (selected) await showLuaScript(selected);
    else setText('#luaScriptContent', 'Backend не обнаружил установку nfqws2 и каталог скриптов.');
  } catch (error) {
    host.textContent = 'Не удалось прочитать список скриптов';
    setText('#luaScriptContent', error instanceof Error ? error.message : 'Ошибка чтения списка');
  }
}

async function showLuaScript(name: string) {
  activeLuaScript = name;
  renderLuaScriptTabs();
  setText('#luaScriptPath', `${name} · чтение…`);
  setText('#luaScriptContent', 'Загружаю файл…');
  try {
    const script = await nfqwsScripts.get(name);
    setText('#luaScriptPath', `${script.path}${script.compressed ? ' · gzip' : ''}${script.truncated ? ' · показаны первые 512 КБ' : ''}`);
    setText('#luaScriptContent', script.content || '(файл пуст)');
  } catch (error) {
    setText('#luaScriptPath', `${name} · файл отсутствует или недоступен`);
    setText('#luaScriptContent', error instanceof Error ? error.message : 'Не удалось прочитать файл');
  }
}
const NFQWS_ARGUMENT_GROUPS = [
  ['NFQWS_EXTRA_ARGS', 'Режим и общие параметры'],
  ['NFQWS_BASE_ARGS', 'Базовые параметры'],
  ['NFQWS_ARGS', 'TCP / TLS'],
  ['NFQWS_ARGS_QUIC', 'QUIC'],
  ['NFQWS_ARGS_UDP', 'UDP'],
  ['NFQWS_ARGS_IPSET', 'IPSET'],
  ['NFQWS_ARGS_CUSTOM', 'Пользовательские доменные профили'],
] as const;
function renderNfqwsArgumentEditors(content: string) {
  const host = $('#nfqwsArgEditors');
  if (!host) return;
  host.replaceChildren();
  for (const [key, label] of NFQWS_ARGUMENT_GROUPS) {
    const card = document.createElement('section');
    card.className = 'block rounded-xl border border-slate-800 bg-[#070b14] p-3';
    const heading = document.createElement('label');
    const fieldId = `configArgument-${key}`;
    heading.htmlFor = fieldId;
    heading.className = 'mb-2 block font-mono text-xs font-bold text-cyan-200';
    heading.textContent = `${key} · ${label}`;
    const field = document.createElement('textarea');
    const minimumRows = key === 'NFQWS_ARGS_CUSTOM' ? 5 : 3;
    field.id = fieldId;
    field.rows = minimumRows;
    field.spellcheck = false;
    field.dataset.configArgument = 'true';
    field.className = 'min-h-0 min-w-0 flex-1 resize-y border-0 bg-transparent px-3 py-2 font-mono text-xs leading-5 text-slate-100 outline-none focus:ring-0 disabled:opacity-60';
    field.value = configAssignmentValue(content, key);
    field.placeholder = key === 'NFQWS_ARGS_CUSTOM' ? '--new --filter-tcp=443 --hostlist-domains=example.com …' : 'Введите аргументы для этой группы';
    field.title = 'Изменение обновит черновик; сохранить его можно кнопкой «Применить конфигурацию».';
    const editor = document.createElement('div');
    editor.className = 'config-argument-editor flex min-w-0 overflow-hidden rounded-lg border border-slate-700 bg-slate-950 transition focus-within:border-blue-500 focus-within:ring-2 focus-within:ring-blue-500/20';
    const gutter = document.createElement('div');
    gutter.className = 'config-argument-line-numbers line-numbers-col min-h-0 shrink-0 overflow-hidden border-r border-slate-800 bg-[#05080f] px-2.5 py-2 font-mono text-xs leading-5 text-slate-500';
    gutter.setAttribute('aria-hidden', 'true');
    editor.append(gutter, field);
    const syncEditor = () => {
      const layout = configEditorLayout(field.value, minimumRows);
      const viewportHeight = configEditorViewportHeight(layout.visibleRows);
      field.rows = layout.visibleRows;
      field.style.maxHeight = viewportHeight;
      gutter.style.maxHeight = viewportHeight;
      gutter.textContent = layout.lineNumbers;
      gutter.scrollTop = field.scrollTop;
    };
    field.addEventListener('input', syncEditor);
    field.addEventListener('scroll', syncEditor);
    syncEditor();
    field.addEventListener('change', () => void updateConfigVariable(key, field.value));
    card.append(heading, editor);
    host.append(card);
  }
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
    renderConfigOutline();
    renderNfqwsArgumentEditors(editor.value);
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
  syncConfigHighlight(editor);
  setText('#confLinesBadge', `${count} строк`);
}
function escapeConfigHtml(value: string): string {
  return value.replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character] ?? character);
}
function highlightConfigTokens(source: string, comments: boolean): string {
  let html = '';
  let index = 0;
  let inLua = false;
  let afterLuaFlag = false;
  let awaitingLuaMode = false;
  let awaitingParamKey = false;
  let awaitingParamValue = false;
  const append = (value: string, token?: string) => {
    const safe = escapeConfigHtml(value);
    html += token ? `<span class="conf-token-${token}">${safe}</span>` : safe;
  };
  while (index < source.length) {
    const rest = source.slice(index);
    if (comments && rest.startsWith('#')) { append(rest, 'comment'); break; }
    if (rest[0] === '"' || rest[0] === "'") {
      const quote = rest[0]; let end = 1; let escaped = false;
      while (end < rest.length) { const ch = rest[end]; if (ch === quote && !escaped) break; escaped = ch === '\\' && !escaped; if (ch !== '\\') escaped = false; end += 1; }
      if (end < rest.length) {
        append(quote, 'string');
        if (quote === '"') html += highlightConfigTokens(rest.slice(1, end), false);
        else append(rest.slice(1, end), 'string');
        append(quote, 'string'); index += end + 1; continue;
      }
      append(quote, 'string'); index += 1; continue;
    }
    const variable = rest.match(/^\$\{?[A-Za-z_][A-Za-z0-9_]*\}?/);
    if (variable) { append(variable[0], 'variable'); index += variable[0].length; continue; }
    const flag = rest.match(/^--[A-Za-z0-9_-]+/);
    if (flag) {
      append(flag[0], 'flag');
      if (flag[0] === '--lua-desync') { inLua = true; afterLuaFlag = true; awaitingLuaMode = false; awaitingParamKey = false; awaitingParamValue = false; }
      index += flag[0].length; continue;
    }
    const delimiter = rest.match(/^[=,:]/);
    if (delimiter) {
      append(delimiter[0], 'operator');
      if (delimiter[0] === '=' && afterLuaFlag) { afterLuaFlag = false; awaitingLuaMode = true; }
      else if (delimiter[0] === ':') { awaitingParamKey = true; awaitingParamValue = false; }
      else if (delimiter[0] === '=' && awaitingParamKey) { awaitingParamKey = false; awaitingParamValue = true; }
      index += 1; continue;
    }
    const number = rest.match(/^-?\d+(?:[._+-]\d+)*%?/);
    if (number) { append(number[0], 'number'); index += number[0].length; continue; }
    const word = rest.match(/^[A-Za-z_][A-Za-z0-9_.-]*/);
    if (word) {
      const value = word[0]; const isKey = rest.slice(value.length).startsWith('=');
      if (isKey) append(value, 'key');
      else if (awaitingLuaMode) { append(value, 'mode'); awaitingLuaMode = false; }
      else if (awaitingParamKey) { append(value, 'mode'); awaitingParamKey = false; }
      else if (awaitingParamValue && inLua) append(value, 'value');
      else if (/^(if|then|else|elif|fi|for|while|do|done|case|esac|in|export|local|return|function)$/i.test(value)) append(value, 'keyword');
      else append(value, 'word');
      index += value.length; continue;
    }
    append(source[index]); index += 1;
  }
  return html;
}
function highlightConfigLine(line: string): string {
  const trimmed = line.trimStart();
  if (trimmed.startsWith('#')) return escapeConfigHtml(line.slice(0, line.length - trimmed.length)) + highlightConfigTokens(trimmed, true);
  return highlightConfigTokens(line, true) || '&nbsp;';
}
function syncConfigHighlight(editor?: HTMLTextAreaElement) {
  const input = editor ?? $('#nfqwsConfTextarea'); const highlight = $('#confSyntaxHighlight');
  if (!(input instanceof HTMLTextAreaElement) || !highlight) return;
  highlight.innerHTML = input.value.split('\n').map(highlightConfigLine).join('\n');
  highlight.scrollTop = input.scrollTop;
  highlight.scrollLeft = input.scrollLeft;
  highlight.style.whiteSpace = input.style.whiteSpace || 'pre-wrap';
}
function renderConfigOutline() {
  const editor = $('#nfqwsConfTextarea');
  const entriesHost = $('#configOutlineEntries');
  if (!(editor instanceof HTMLTextAreaElement) || !entriesHost) return;
  const entries = configOutlineEntries(editor.value);
  const active = entries.filter((entry) => entry.offset <= editor.selectionStart).at(-1);
  entriesHost.replaceChildren();
  if (!entries.length) {
    const empty = document.createElement('div');
    empty.className = 'px-2 py-2 text-xs text-slate-500';
    empty.textContent = 'В конфигурации не найдены переменные.';
    entriesHost.append(empty);
    return;
  }
  for (const entry of entries) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = `config-outline-entry flex w-full items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-left font-mono text-xs transition-colors hover:bg-slate-800 ${active?.offset === entry.offset ? 'bg-blue-500/10 text-blue-200' : 'text-slate-200'}`;
    button.title = `Перейти к строке ${entry.line}`;
    const name = document.createElement('span');
    name.className = 'min-w-0 truncate';
    name.textContent = entry.name;
    const line = document.createElement('span');
    line.className = 'shrink-0 tabular-nums text-slate-500';
    line.textContent = String(entry.line);
    button.append(name, line);
    button.addEventListener('click', () => {
      editor.focus();
      editor.setSelectionRange(entry.offset, entry.offset + entry.name.length);
      const lineHeight = Number.parseFloat(getComputedStyle(editor).lineHeight) || 23;
      editor.scrollTop = Math.max(0, (entry.line - 1) * lineHeight);
      renderConfigOutline();
    });
    entriesHost.append(button);
  }
}
function toggleConfigOutline() {
  const outline = $('#configOutline');
  const workspace = $('#configEditorWorkspace');
  const button = $('#configStructureToggle');
  if (!outline || !workspace || !button) return;
  const open = outline.classList.contains('hidden');
  outline.classList.toggle('hidden', !open);
  workspace.classList.toggle('config-editor-workspace-open', open);
  button.setAttribute('aria-expanded', String(open));
  button.setAttribute('aria-pressed', String(open));
  button.title = open ? 'Скрыть структуру переменных конфигурации справа' : 'Показать структуру переменных конфигурации справа';
  button.classList.toggle('config-structure-toggle-active', open);
  if (open) renderConfigOutline();
}
async function saveConfigAndApply() {
  if (busy) return;
  if (!config) return toast('Сначала загрузите конфигурацию backend.', 'error');
  const editor = $('#nfqwsConfTextarea'); if (!(editor instanceof HTMLTextAreaElement)) return;
  const draft = editor.value;
  setBusy(true);
  let validation: ConfigValidationResponse;
  try {
    validation = await nfqwsConfig.validate(draft);
  } catch (error) {
    setBusy(false);
    return toast(error instanceof Error ? `Проверка backend не выполнена: ${error.message}` : 'Проверка backend не выполнена', 'error');
  }
  if (editor.value !== draft) {
    setBusy(false);
    return toast('Черновик изменился во время проверки. Проверьте конфигурацию повторно.', 'error');
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
    const result = await nfqwsApply.apply(draft, config.revision);
    config = result.config;
    setRestartInProgress(false);
    await fetchStatus();
    toast(lifecycleMessage('restart'));
  } catch (error) { setRestartInProgress(false); await fetchStatus(); toast(error instanceof Error ? error.message : 'Не удалось сохранить конфигурацию', 'error'); }
  finally { setRestartInProgress(false); setBusy(false); }
}
async function serviceAction(action: 'start' | 'stop' | 'restart') {
  if (busy) return;

  setBusy(true);
  if (action === 'restart') setRestartInProgress(true);
  try { await api('/api/service', { method: 'POST', body: JSON.stringify({ action }) }); toast(lifecycleMessage(action)); }
  catch (error) { toast(error instanceof Error ? error.message : 'Команда не выполнена', 'error'); }
  finally {
    if (action === 'restart') setRestartInProgress(false);
    setBusy(false);
    await fetchStatus();
  }
}
async function loadList(id: NfqwsListId) {

  try {
    const index = await nfqwsLists.index();
    renderCustomListTabs(index.files);
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
    selectedListRows.clear();
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

function setListEditorView(view: 'text' | 'rows') {
  listEditorView = view === 'rows' ? 'rows' : 'text';
  try { localStorage.setItem(LIST_EDITOR_VIEW_KEY, view); } catch { /* The view preference is optional. */ }
  const textView = $('#listTextView');
  const rowsView = $('#listRowsView');
  textView?.classList.toggle('hidden', listEditorView !== 'text');
  rowsView?.classList.toggle('hidden', listEditorView !== 'rows');
  for (const [id, active] of [['listViewTextButton', listEditorView === 'text'], ['listViewRowsButton', listEditorView === 'rows']] as const) {
    const button = $(`#${id}`);
    button?.setAttribute('aria-pressed', String(active));
    button?.classList.toggle('active', active);
  }
  const remove = $('[data-action-click="removeSelectedListLines"]');
  if (remove) remove.textContent = listEditorView === 'rows' ? `Удалить выбранные (${selectedListRows.size})` : 'Удалить строку/выделение';
  $('#findNextListEntryButton')?.classList.toggle('hidden', listEditorView === 'rows');
  if (listEditorView === 'rows') renderListRows();
}

function removeListRows(indices: readonly number[]) {
  const editor = $('#listEditorTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return;
  const result = removeListLinesByIndices(editor.value, indices);
  if (!result.removed) return toast('Выберите строки для удаления.', 'error');
  editor.value = result.content;
  selectedListRows.clear();
  syncListLineNumbers(editor);
  toast(`Удалено строк: ${result.removed}. Изменён только черновик.`);
}

function renderListRows() {
  const root = $('#listRowsView');
  const editor = $('#listEditorTextarea');
  if (!root || !(editor instanceof HTMLTextAreaElement)) return;
  const search = $('#listSearchInput');
  const query = search instanceof HTMLInputElement ? search.value : '';
  const rows = listRowsFromContent(editor.value, query);
  root.replaceChildren();
  if (!rows.length) {
    const empty = document.createElement('p');
    empty.className = 'px-3 py-6 text-center text-sm text-slate-400';
    empty.textContent = query.trim() ? 'Совпадений не найдено.' : 'Список пуст.';
    root.append(empty);
  }
  for (const row of rows) {
    const item = document.createElement('div');
    item.className = `list-editor-row${row.comment ? ' is-comment' : ''}`;
    item.setAttribute('role', 'listitem');
    const checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.checked = selectedListRows.has(row.index);
    checkbox.setAttribute('aria-label', `Выбрать строку ${row.index + 1}: ${row.text}`);
    checkbox.addEventListener('change', () => {
      checkbox.checked ? selectedListRows.add(row.index) : selectedListRows.delete(row.index);
      const remove = $('[data-action-click="removeSelectedListLines"]');
      if (remove) remove.textContent = `Удалить выбранные (${selectedListRows.size})`;
    });
    const line = document.createElement('span');
    line.className = 'list-editor-row-number';
    line.textContent = String(row.index + 1);
    const text = document.createElement('span');
    text.className = 'min-w-0 flex-1 break-all font-mono';
    text.textContent = row.text;
    const remove = document.createElement('button');
    remove.type = 'button';
    remove.className = 'list-editor-row-remove';
    remove.setAttribute('aria-label', `Удалить строку ${row.index + 1}`);
    remove.textContent = 'Удалить';
    remove.addEventListener('click', () => removeListRows([row.index]));
    item.append(checkbox, line, text, remove);
    root.append(item);
  }
}

function renderCustomListTabs(files: readonly NfqwsListSummary[]) {
  const host = $('#customListTabs');
  if (!host) return;
  host.replaceChildren();
  const standardIds = new Set(['user.list', 'auto.list', 'exclude.list', 'ipset.list', 'ipset_exclude.list']);
  const fileById = new Map(files.map((file) => [file.id, file]));
  for (const id of listTabIdsFromInventory(files).filter((candidate) => !standardIds.has(candidate))) {
    const summary = fileById.get(id);
    if (!summary) continue;
    const group = document.createElement('div');
    group.className = 'list-custom-tab-group';
    const button = document.createElement('button');
    button.type = 'button';
    button.id = `listTab-${id}`;
    button.dataset.actionClick = 'selectListFile';
    button.dataset.args = JSON.stringify([id]);
    button.dataset.customListTab = 'true';
    button.className = 'list-file-tab flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-medium text-slate-400 hover:text-slate-200 border-b-2 border-transparent transition-all whitespace-nowrap';
    button.title = `${listDisplayName(id)} — ${summary.path}`;
    const label = document.createElement('span');
    label.textContent = id;
    button.append(label);
    const remove = document.createElement('button');
    remove.type = 'button';
    remove.dataset.actionClick = 'requestDeleteCustomList';
    remove.dataset.args = JSON.stringify([id]);
    remove.dataset.customListDelete = 'true';
    remove.dataset.actionTooltip = `Удалить пользовательский список ${id}`;
    remove.className = 'list-file-delete';
    remove.setAttribute('aria-label', `Удалить пользовательский список ${id}`);
    remove.innerHTML = '<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M8 6V4h8v2m-9 0 1 14h8l1-14M10 10v6m4-6v6"></path></svg>';
    group.append(button, remove);
    host.append(group);
  }
}
function requestDeleteCustomList(id: string) {
  const standardIds = new Set(['user.list', 'auto.list', 'exclude.list', 'ipset.list', 'ipset_exclude.list']);
  if (standardIds.has(id) || !/^[A-Za-z0-9_.-]{1,128}\.list$/.test(id) || id.includes('..')) return;
  const modal = $('#customListDeleteModal');
  const name = $('#customListDeleteName');
  const confirm = $('#confirmCustomListDelete');
  if (!modal || !name || !(confirm instanceof HTMLButtonElement)) return;
  name.textContent = id;
  confirm.dataset.args = JSON.stringify([id]);
  modal.classList.remove('hidden');
  modal.classList.add('flex');
}
function closeCustomListDeleteModal() {
  const modal = $('#customListDeleteModal');
  modal?.classList.add('hidden');
  modal?.classList.remove('flex');
}
async function deleteCustomList(id: string) {
  const standardIds = new Set(['user.list', 'auto.list', 'exclude.list', 'ipset.list', 'ipset_exclude.list']);
  if (standardIds.has(id) || !/^[A-Za-z0-9_.-]{1,128}\.list$/.test(id) || id.includes('..')) return;
  try {
    const current = await nfqwsLists.get(id);
    await nfqwsLists.delete(id, current.revision);
    closeCustomListDeleteModal();
    const index = await nfqwsLists.index();
    renderCustomListTabs(index.files);
    toast(`Список ${id} удалён.`);
    const nextId = index.files.some((file) => file.id === activeListId) ? activeListId : index.files[0]?.id;
    if (nextId) await loadList(nextId);
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось удалить пользовательский список', 'error');
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
    selectedListRows.clear();
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
  if (listEditorView === 'rows') {
    const remove = $('[data-action-click="removeSelectedListLines"]');
    if (remove) remove.textContent = `Удалить выбранные (${selectedListRows.size})`;
    renderListRows();
  }
}
function syncListScroll(editor: HTMLTextAreaElement) {
  const gutter = $('#listLineNumbers');
  if (gutter) gutter.scrollTop = editor.scrollTop;
}
function removeSelectedListLines() {
  if (!activeList) return toast('Сначала загрузите список из backend.', 'error');
  const editor = $('#listEditorTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return;
  if (listEditorView === 'rows') {
    if (!selectedListRows.size) return toast('Отметьте строки, которые нужно удалить.', 'error');
    return removeListRows([...selectedListRows]);
  }
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
function renderDiagnosisQueue(container: HTMLElement, stages: Array<{ id: string; label: string }>, activeIndex: number, results?: ReturnType<typeof diagnosisRows>) {
  const section = document.createElement('section'); section.className = 'diagnosis-queue ui-card rounded-2xl';
  section.setAttribute('aria-label', 'Этапы проверки сайта');
  const title = document.createElement('h3'); title.className = 'diagnosis-checks-title';
  title.textContent = activeIndex < 0 ? 'Этапы проверки' : activeIndex >= stages.length ? 'Проверки завершены · результаты по этапам' : 'Проверка сайта';
  section.append(title);
  const list = document.createElement('ol'); list.className = 'diagnosis-queue-list';
  stages.forEach((stage, index) => {
    const state = diagnosisQueueState(index, activeIndex);
    const result = results?.find((item) => item.id === stage.id);
    const row = document.createElement('li');
    row.className = `diagnosis-queue-step is-${state}${result ? ` is-result-${result.kind}` : ''}`;
    row.setAttribute('aria-current', state === 'active' ? 'step' : 'false');
    const marker = document.createElement('span'); marker.className = 'diagnosis-queue-marker'; marker.setAttribute('aria-hidden', 'true');
    marker.textContent = state === 'complete' ? (result?.kind === 'failure' ? '×' : result?.kind === 'unknown' ? '!' : '✓') : state === 'active' ? '…' : String(index + 1);
    const label = document.createElement('span'); label.textContent = stage.label;
    const status = document.createElement('small'); status.textContent = state === 'active' ? 'Выполняется' : state === 'pending' ? 'Ожидает' : result?.kind === 'failure' ? 'Ошибка' : result?.kind === 'unknown' ? 'Не определено' : 'Готово';
    row.append(marker, label, status); list.append(row);
  });
  section.append(list); container.replaceChildren(section);
}
const waitForDiagnosisAnimation = () => new Promise<void>((resolve) => window.setTimeout(resolve, 120));
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
  if (!input.value.trim()) return toast('Введите домен или публичный IPv4 для проверки.', 'error');
  diagnosisRunning = true; input.disabled = true; button.disabled = true;
  button.textContent = 'Проверяем…'; container.replaceChildren();
  renderDiagnosisQueue(container, SITE_DIAGNOSIS_STAGES.map(({ id, label }) => ({ id, label })), -1);
  const toastHost = $('#toastWrapper');
  const closeProgress = toastHost ? showDiagnosisProgress(toastHost, input.value.trim()) : () => undefined;
  const loading = document.createElement('p'); loading.className = 'col-span-full text-sm text-blue-300 animate-pulse';
  loading.setAttribute('role', 'status'); loading.textContent = 'Сравниваем доступ напрямую, текущий маршрут и QUIC с роутера…'; container.append(loading);
  try {
    const report = await nfqwsDiagnostics.site(input.value);
    const resultRows = diagnosisRows(report);
    const stages = resultRows.map(({ id, label }) => ({ id, label }));
    for (let index = 0; index < stages.length; index += 1) {
      renderDiagnosisQueue(container, stages, index, resultRows);
      await waitForDiagnosisAnimation();
      renderDiagnosisQueue(container, stages, index + 1, resultRows);
    }
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
    let renderedReport = '';
    for (let attempt = 0; attempt < 90 && !terminal.includes(current.state); attempt += 1) {
      if (operationStatus) operationStatus.textContent = current.message || `Этап: ${current.stage}`;
      if (current.report) {
        const signature = JSON.stringify(current.report);
        if (signature !== renderedReport) {
          renderTraceReport(container, current.report);
          renderedReport = signature;
        }
      }
      await new Promise((resolve) => window.setTimeout(resolve, 900));
      try { current = await nfqwsDiagnostics.traceStatus(started.id); }
      catch (error) {
        if (operationStatus) operationStatus.textContent = error instanceof Error ? `${current.message} · переподключение…` : 'Связь с роутером восстановится автоматически…';
      }
    }
    if (current.report && JSON.stringify(current.report) !== renderedReport) renderTraceReport(container, current.report);
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

const FOREIGN_CONFIG_CHECK_STORAGE = 'nfqws2-ui-foreign-config-check-id';

function showForeignConfigForm(kind: 'text' | 'url') {
  $('#foreignConfigTextForm')?.classList.toggle('hidden', kind !== 'text');
  $('#foreignConfigUrlForm')?.classList.toggle('hidden', kind !== 'url');
  $(`#foreignConfig${kind === 'text' ? 'TextContent' : 'Url'}`)?.focus();
}

function closeForeignConfigForms() {
  $('#foreignConfigTextForm')?.classList.add('hidden');
  $('#foreignConfigUrlForm')?.classList.add('hidden');
}

function renderForeignConfigInputs() {
  const list = $('#foreignConfigList');
  if (!list) return;
  list.replaceChildren();
  if (!foreignConfigInputs.length) {
    const empty = document.createElement('p'); empty.className = 'text-sm text-slate-500'; empty.textContent = 'Добавьте конфиг текстом, ссылкой или файлом .conf.'; list.append(empty); return;
  }
  foreignConfigInputs.forEach((item, index) => {
    const row = document.createElement('div'); row.className = 'flex min-w-0 items-center justify-between gap-3 rounded-lg border border-slate-800 bg-slate-950/35 px-3 py-2';
    const title = document.createElement('div'); title.className = 'min-w-0';
    const name = document.createElement('strong'); name.className = 'block truncate text-sm text-slate-200'; name.textContent = item.name;
    const source = document.createElement('span'); source.className = 'block truncate text-[11px] text-slate-500'; source.textContent = item.source === 'Ссылка' ? item.url ?? '' : `${item.source} · ${new TextEncoder().encode(item.content ?? '').length} байт`;
    title.append(name, source);
    const remove = document.createElement('button'); remove.type = 'button'; remove.className = 'shrink-0 rounded-md border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:border-rose-400 hover:text-rose-200'; remove.textContent = 'Убрать';
    remove.addEventListener('click', () => { foreignConfigInputs.splice(index, 1); renderForeignConfigInputs(); });
    row.append(title, remove); list.append(row);
  });
}

function addForeignConfig(input: ForeignConfigInput) {
  if (foreignConfigInputs.length >= 3) return toast('Можно сравнить не больше трёх конфигов за один запуск.', 'error');
  foreignConfigInputs.push(input);
  renderForeignConfigInputs();
  closeForeignConfigForms();
  toast(`Добавлен конфиг «${input.name}».`, 'success');
}

function addForeignConfigText() {
  const content = $('#foreignConfigTextContent');
  const nameInput = $('#foreignConfigTextName');
  if (!(content instanceof HTMLTextAreaElement)) return;
  const value = content.value.trim();
  if (!value) return toast('Вставьте содержимое nfqws2.conf.', 'error');
  if (new TextEncoder().encode(value).length > 65536) return toast('Размер конфига не должен превышать 64 КиБ.', 'error');
  const name = nameInput instanceof HTMLInputElement && nameInput.value.trim() ? nameInput.value.trim() : `Конфиг ${foreignConfigInputs.length + 1}`;
  addForeignConfig({ name, source: 'Текст', content: value });
  content.value = '';
  if (nameInput instanceof HTMLInputElement) nameInput.value = '';
}

function addForeignConfigUrl() {
  const urlInput = $('#foreignConfigUrl');
  const nameInput = $('#foreignConfigUrlName');
  if (!(urlInput instanceof HTMLInputElement) || !urlInput.value.trim()) return toast('Укажите HTTPS-ссылку на .conf.', 'error');
  const name = nameInput instanceof HTMLInputElement && nameInput.value.trim() ? nameInput.value.trim() : `Конфиг ${foreignConfigInputs.length + 1}`;
  addForeignConfig({ name, source: 'Ссылка', url: urlInput.value.trim() });
  urlInput.value = '';
  if (nameInput instanceof HTMLInputElement) nameInput.value = '';
}

async function addForeignConfigFile(file: File) {
  if (!file.name.toLowerCase().endsWith('.conf')) return toast('Выберите файл с расширением .conf.', 'error');
  if (file.size > 65536) return toast('Размер файла не должен превышать 64 КиБ.', 'error');
  try {
    const content = await file.text();
    if (!content.trim()) return toast('Файл пустой.', 'error');
    addForeignConfig({ name: file.name, source: 'Файл', content });
  } catch {
    toast('Не удалось прочитать файл конфигурации.', 'error');
  }
}

function foreignStatusLabel(value: boolean | null) {
  return value === true ? 'Доступен' : value === false ? 'Не отвечает' : 'Ожидает';
}

function renderForeignConfigCheck(operation: ForeignConfigCheckOperation) {
  const status = $('#foreignConfigStatus');
  const results = $('#foreignConfigResults');
  const progress = $('#foreignConfigProgress');
  const bar = $('#foreignConfigProgressBar');
  const active = ['queued', 'running', 'stopping'].includes(operation.state);
  if (status) { status.classList.remove('hidden'); status.textContent = operation.current ? `${operation.done}/${operation.total} · ${operation.current}` : operation.message; }
  progress?.classList.toggle('hidden', !active);
  if (bar) (bar as HTMLElement).style.width = `${operation.total ? Math.round(operation.done * 100 / operation.total) : 0}%`;
  const cancel = $('#cancelForeignConfigCheck');
  if (cancel instanceof HTMLButtonElement) {
    cancel.classList.toggle('hidden', !active);
    cancel.disabled = operation.state === 'stopping';
    cancel.textContent = operation.state === 'stopping' ? 'Останавливаю…' : 'Остановить';
  }
  const start = $('#runForeignConfigCheck');
  if (start instanceof HTMLButtonElement) { start.disabled = active; start.textContent = active ? 'Проверка…' : '▷ Проверить конфиги'; }
  if (!results) return;
  results.replaceChildren();
  for (const comparison of operation.comparisons) {
    const card = document.createElement('section'); card.className = 'ui-card min-w-0 rounded-xl border border-slate-800 p-3';
    const heading = document.createElement('div'); heading.className = 'mb-2 flex items-center justify-between gap-2';
    const title = document.createElement('h3'); title.className = 'truncate text-sm font-bold text-slate-100'; title.textContent = comparison.name;
    const source = document.createElement('span'); source.className = 'shrink-0 text-[10px] text-slate-500'; source.textContent = comparison.source;
    heading.append(title, source); card.append(heading);
    for (const site of comparison.sites) {
      const row = document.createElement('div'); row.className = 'grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-x-3 gap-y-0.5 border-t border-slate-800/70 py-2 text-xs';
      const host = document.createElement('strong'); host.className = 'truncate font-mono text-slate-300'; host.textContent = site.host;
      const current = document.createElement('span'); current.className = `whitespace-nowrap ${site.currentOk === true ? 'text-emerald-300' : site.currentOk === false ? 'text-rose-300' : 'text-slate-500'}`; current.textContent = `Текущий: ${foreignStatusLabel(site.currentOk)}`;
      const candidate = document.createElement('span'); candidate.className = `whitespace-nowrap ${site.candidateOk === true ? 'text-emerald-300' : site.candidateOk === false ? 'text-rose-300' : 'text-slate-500'}`; candidate.textContent = `Этот: ${foreignStatusLabel(site.candidateOk)}`;
      const detail = document.createElement('p'); detail.className = 'col-span-3 break-words text-[10px] leading-4 text-slate-500'; detail.textContent = site.candidateOk === null ? site.currentDetail : site.candidateDetail;
      row.append(host, current, candidate, detail); card.append(row);
    }
    results.append(card);
  }
}

async function pollForeignConfigCheck(id: string) {
  activeForeignConfigCheckId = id; foreignConfigCheckRunning = true;
  localStorage.setItem(FOREIGN_CONFIG_CHECK_STORAGE, id);
  const terminal = ['complete', 'cancelled', 'failed'];
  let finished = false;
  try {
    let current = await nfqwsDiagnostics.foreignConfigStatus(id);
    renderForeignConfigCheck(current);
    for (let attempt = 0; attempt < 300 && !terminal.includes(current.state); attempt += 1) {
      await new Promise((resolve) => window.setTimeout(resolve, 1000));
      current = await nfqwsDiagnostics.foreignConfigStatus(id);
      renderForeignConfigCheck(current);
    }
    if (!terminal.includes(current.state)) setText('#foreignConfigStatus', `${current.message} · проверка продолжится на роутере; можно вернуться позже.`);
    if (terminal.includes(current.state)) {
      finished = true;
      localStorage.removeItem(FOREIGN_CONFIG_CHECK_STORAGE);
      toast(current.message, current.state === 'failed' ? 'error' : 'success');
    }
  } catch (error) {
    setText('#foreignConfigStatus', error instanceof Error ? error.message : 'Не удалось получить состояние проверки конфигурации.');
  } finally {
    // Keep the operation addressable after a polling timeout or a temporary
    // connection error so the user can still cancel it from this page.
    foreignConfigCheckRunning = !finished;
    activeForeignConfigCheckId = finished ? null : id;
  }
}

async function runForeignConfigCheck() {
  if (foreignConfigCheckRunning) return;
  if (!foreignConfigInputs.length) return toast('Добавьте хотя бы один конфиг для проверки.', 'error');
  const hostsInput = $('#foreignConfigHosts');
  const hosts = hostsInput instanceof HTMLInputElement ? hostsInput.value.split(/[\s,;]+/).filter(Boolean) : [];
  if (!hosts.length || hosts.length > 3) return toast('Укажите от одного до трёх сайтов или публичных IPv4.', 'error');
  const button = $('#runForeignConfigCheck');
  if (button instanceof HTMLButtonElement) { button.disabled = true; button.textContent = 'Запуск…'; }
  try {
    const operation = await nfqwsDiagnostics.foreignConfigStart(foreignConfigInputs, hosts);
    renderForeignConfigCheck(operation);
    await pollForeignConfigCheck(operation.id);
  } catch (error) {
    if (button instanceof HTMLButtonElement) { button.disabled = false; button.textContent = '▷ Проверить конфиги'; }
    toast(error instanceof Error ? error.message : 'Не удалось запустить проверку конфигураций.', 'error');
  }
}

async function cancelForeignConfigCheck() {
  if (!activeForeignConfigCheckId) return;
  const button = $('#cancelForeignConfigCheck');
  if (button instanceof HTMLButtonElement) { button.disabled = true; button.textContent = 'Останавливаю…'; }
  try { renderForeignConfigCheck(await nfqwsDiagnostics.foreignConfigCancel(activeForeignConfigCheckId)); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось остановить проверку.', 'error'); }
}

const STRATEGY_PICK_STORAGE = 'nfqws2-ui-strategy-pick-id';
function renderStrategyPick(operation: StrategyPickOperation) {
  localStorage.setItem(STRATEGY_PICK_STORAGE, operation.id);
  setText('#scanProgressLabel', operation.current ? `${operation.stage === 'refining' ? 'Уточнение' : 'Проверка'} · ${operation.done}/${operation.total} · ${operation.current}` : operation.message);
  const bar = $('#scanProgressBar');
  if (bar) (bar as HTMLElement).style.width = `${operation.total ? Math.round(100 * operation.done / operation.total) : 0}%`;
  const container = $('#autotuneStrategiesList');
  if (!container) return;
  const expandedResults = new Set(
    [...container.querySelectorAll<HTMLDetailsElement>('details.strategy-pick-steps[open]')]
      .map((details) => details.dataset.candidateKey)
      .filter((key): key is string => Boolean(key)),
  );
  container.replaceChildren();
  const confirmed = operation.results.filter((result) => result.confirmed);
  const windowsTcpTimestampsCommand = 'netsh interface tcp set global timestamps=enabled';
  const resultSummary = document.createElement('section');
  resultSummary.className = `strategy-pick-summary ${confirmed.length ? 'has-confirmed' : operation.state === 'running' ? 'is-running' : 'has-no-confirmed'}`;
  resultSummary.setAttribute('role', operation.state === 'running' ? 'status' : 'region');
  const resultTitle = document.createElement('h3'); resultTitle.className = 'strategy-pick-summary-title';
  resultTitle.textContent = operation.state === 'running'
    ? 'Проверка стратегий выполняется'
    : confirmed.length
      ? `Подтверждено ${confirmed.length} из ${operation.total} стратегий`
      : operation.state === 'cancelled'
        ? 'Проверка остановлена'
        : operation.state === 'failed'
          ? 'Проверка прервана'
          : 'Рабочая стратегия не подтверждена';
  const resultMeta = document.createElement('p'); resultMeta.className = 'strategy-pick-summary-meta';
  resultMeta.textContent = `Проверено ${operation.done} из ${operation.total} стратегий · результатов: ${operation.results.length}`;
  const resultMessage = document.createElement('p'); resultMessage.className = 'strategy-pick-summary-message'; resultMessage.textContent = operation.message;
  resultSummary.append(resultTitle, resultMeta, resultMessage);
  if (!operation.results.length) {
    if (operation.state === 'site_unblocked') resultTitle.textContent = 'Без обхода сайт уже отвечает';
    resultMeta.textContent = operation.total ? `Проверено ${operation.done} из ${operation.total} стратегий` : 'Стратегии не запускались';
    container.append(resultSummary);
    return;
  }
  let fastest: StrategyPickOperation['results'][number] | undefined;
  if (confirmed.length) {
    fastest = [...confirmed].sort((a, b) => a.elapsedMs - b.elapsedMs)[0];
    const winner = document.createElement('section'); winner.className = 'strategy-pick-winner';
    const title = document.createElement('strong'); title.textContent = `Быстрее всех · ${fastest.candidate.name}`;
    const metric = document.createElement('span'); metric.className = 'strategy-pick-winner-time'; metric.textContent = `${fastest.elapsedMs} мс`;
    const source = document.createElement('p'); source.className = 'strategy-pick-winner-source'; source.textContent = `${fastest.successes}/${fastest.attempts} успешных проверок · ${fastest.candidate.source}`;
    const steps = document.createElement('code'); steps.className = 'strategy-pick-winner-steps'; steps.textContent = fastest.candidate.steps.join('\n');
    winner.append(title, metric, source, steps);
    if (appearanceSettings.remindWindowsTcpTimestamps && strategyNeedsWindowsTcpTimestamps(fastest.candidate.steps)) {
      const hint = document.createElement('p'); hint.className = 'strategy-pick-windows-hint'; hint.textContent = 'На Windows эта стратегия зависит от TCP timestamps. Если она не работает, включите timestamps в командной строке администратора:';
      const command = document.createElement('code'); command.textContent = windowsTcpTimestampsCommand;
      const copyCommand = document.createElement('button'); copyCommand.type = 'button'; copyCommand.className = 'strategy-copy-button rounded-md border border-slate-700 px-2.5 py-1 text-[11px] font-semibold'; copyCommand.textContent = 'Копировать команду'; copyCommand.dataset.actionClick = 'copySnippetToClipboard'; copyCommand.dataset.args = JSON.stringify([windowsTcpTimestampsCommand]);
      winner.append(hint, command, copyCommand);
    }
    container.append(resultSummary, winner);
  } else container.append(resultSummary);
  operation.results.forEach((result, index) => {
    const refined = result.candidate.source === 'Уточнение blockcheck2';
    const card = document.createElement('article');
    card.className = `strategy-pick-result-row strategy-result-enter ${result.confirmed ? 'is-confirmed' : 'is-unconfirmed'}`;
    const status = document.createElement('span'); status.className = 'strategy-pick-result-status'; status.setAttribute('aria-label', result.confirmed ? 'Подтверждена' : 'Не подтверждена');
    const statusIcon = document.createElementNS('http://www.w3.org/2000/svg', 'svg'); statusIcon.classList.add('strategy-pick-result-status-icon'); statusIcon.setAttribute('viewBox', '0 0 16 16'); statusIcon.setAttribute('aria-hidden', 'true');
    const statusPath = document.createElementNS('http://www.w3.org/2000/svg', 'path'); statusPath.setAttribute('d', result.confirmed ? 'm3.25 8.25 3 3 6.5-6.5' : 'm4.5 4.5 7 7m0-7-7 7'); statusPath.setAttribute('fill', 'none'); statusPath.setAttribute('stroke', 'currentColor'); statusPath.setAttribute('stroke-width', result.confirmed ? '1.8' : '1.6'); statusPath.setAttribute('stroke-linecap', 'round'); statusPath.setAttribute('stroke-linejoin', 'round'); statusIcon.append(statusPath); status.append(statusIcon);
    const body = document.createElement('div'); body.className = 'strategy-pick-result-main';
    const heading = document.createElement('h4'); heading.className = 'strategy-pick-result-title';
    heading.textContent = `${result.candidate.name}${refined ? ' · уточнённый вариант' : ''}`;
    const headingLine = document.createElement('div'); headingLine.className = 'strategy-pick-result-heading';
    const verdict = document.createElement('span'); verdict.className = `strategy-pick-result-badge ${result.confirmed ? 'is-confirmed' : 'is-unconfirmed'}`;
    verdict.textContent = result.confirmed ? 'Подтверждена' : 'Не подтверждена';
    headingLine.append(heading, verdict);
    if (appearanceSettings.remindWindowsTcpTimestamps && strategyNeedsWindowsTcpTimestamps(result.candidate.steps)) {
      const timestampBadge = document.createElement('span'); timestampBadge.className = 'strategy-pick-capability-badge'; timestampBadge.textContent = 'нужны TCP timestamps'; timestampBadge.title = 'На Windows включите временные метки TCP, если эта стратегия не работает.';
      headingLine.append(timestampBadge);
    }
    const meta = document.createElement('p'); meta.className = 'strategy-pick-result-meta'; meta.textContent = `${result.candidate.source} · ${result.successes}/${result.attempts} успешных · ${result.elapsedMs} мс`;
    const detail = document.createElement('p'); detail.className = 'strategy-pick-result-detail'; detail.textContent = result.detail;
    const details = document.createElement('details'); details.className = 'strategy-pick-steps';
    const candidateKey = JSON.stringify([result.candidate.name, result.candidate.steps]);
    details.dataset.candidateKey = candidateKey;
    details.open = expandedResults.has(candidateKey);
    const detailsSummary = document.createElement('summary'); detailsSummary.textContent = `Подробности и параметры · ${result.candidate.steps.length} шаг.`;
    const args = document.createElement('code'); args.textContent = result.candidate.steps.join('\n');
    const copy = document.createElement('button'); copy.type = 'button'; copy.className = 'strategy-copy-button mt-2 rounded-md border border-slate-700 px-2.5 py-1 text-[11px] font-semibold text-slate-300 transition-colors hover:border-blue-500 hover:text-blue-200'; copy.textContent = 'Копировать'; copy.dataset.actionClick = 'copyStrategyResult'; copy.dataset.args = JSON.stringify([result.candidate.steps.join('\n'), { source: 'self' }]);
    details.append(detailsSummary, args, copy); body.append(headingLine, meta, detail, details); card.append(status, body);
    const canApplyConfirmedResult = ['complete', 'partial', 'cancelled', 'failed'].includes(operation.state);
    if (canApplyConfirmedResult && result.confirmed) {
      const preview = document.createElement('button');
      preview.type = 'button';
      preview.className = 'rounded-lg border border-blue-400/30 bg-blue-500/15 px-3 py-2 text-sm font-semibold text-blue-100 transition hover:bg-blue-500/25 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300';
      preview.dataset.actionClick = 'previewStrategyPick';
      preview.dataset.arg = String(index);
      preview.textContent = 'Предпросмотр и применение Strategy #1';
      card.append(preview);
    }
    container.append(card);
  });
}

function renderStrategyPickPreview(preview: StrategyPickPreview, source: 'strategyPick' | 'freezeScan', candidateIndex: number, target: 'autotune' | 'freeze' = 'autotune') {
  const panel = $(`#${target === 'freeze' ? 'freezeApplyPreviewPanel' : 'autotunePreviewPanel'}`);
  const summary = $(`#${target === 'freeze' ? 'freezeApplyPreviewSummary' : 'autotunePreviewSummary'}`);
  const diff = $(`#${target === 'freeze' ? 'freezeApplyPreviewDiff' : 'autotunePreviewDiff'}`);
  if (!panel || !summary || !diff) return;
  panel.classList.remove('hidden');
  summary.textContent = `${preview.host} · ${preview.protocol.toUpperCase()} · ${preview.candidate.name}. ${preview.summary} Revision: ${preview.originalRevision.slice(0, 12)}.`;
  diff.replaceChildren();
  const before = preview.originalContent.split(/\r?\n/);
  const after = preview.proposedContent.split(/\r?\n/);
  const maxLines = Math.max(before.length, after.length);
  for (let index = 0; index < maxLines; index += 1) {
    if (before[index] === after[index]) continue;
    const row = document.createElement('div');
    row.className = 'grid gap-1 border-b border-slate-800 py-2 last:border-0 sm:grid-cols-2';
    const oldLine = document.createElement('code');
    oldLine.className = 'whitespace-pre-wrap break-all text-rose-200';
    oldLine.textContent = before[index] === undefined ? '− (нет строки)' : `− ${before[index]}`;
    const newLine = document.createElement('code');
    newLine.className = 'whitespace-pre-wrap break-all text-emerald-200';
    newLine.textContent = after[index] === undefined ? '+ (удалено)' : `+ ${after[index]}`;
    row.append(oldLine, newLine);
    diff.append(row);
  }
  if (!diff.childElementCount) diff.textContent = 'Изменений нет.';
  activeStrategyPreview = { source, operationId: preview.operationId, candidateIndex, revision: preview.originalRevision, panel: target };
  const apply = $(`#${target === 'freeze' ? 'applyFreezeStrategyButton' : 'applyStrategyPickButton'}`);
  if (apply instanceof HTMLButtonElement) {
    apply.disabled = false;
    apply.dataset.args = JSON.stringify([source, preview.operationId, String(candidateIndex), preview.originalRevision]);
  }
}

async function previewStrategyPick(candidateIndex: string) {
  const operationId = activeStrategyPickId ?? localStorage.getItem(STRATEGY_PICK_STORAGE);
  const index = Number(candidateIndex);
  if (!operationId || !Number.isInteger(index) || index < 0) return toast('Результат подбора не выбран.', 'error');
  const panel = $('#autotunePreviewPanel');
  panel?.classList.remove('hidden');
  setText('#autotunePreviewSummary', 'Проверяется подтверждённый результат и текущая конфигурация…');
  const diff = $('#autotunePreviewDiff');
  if (diff) diff.textContent = '';
  try {
    const preview = await nfqwsDiagnostics.strategyPreview(operationId, index);
    renderStrategyPickPreview(preview, 'strategyPick', index);
  } catch (error) {
    panel?.classList.add('hidden');
    toast(error instanceof Error ? error.message : 'Предпросмотр стратегии недоступен.', 'error');
  }
}

async function previewFreezeStrategy(operationId: string, resultIndex: string) {
  const index = Number(resultIndex);
  if (!Number.isInteger(index) || index < 0) return toast('Индекс проверенного домена некорректен.', 'error');
  const panel = $('#freezeApplyPreviewPanel');
  panel?.classList.remove('hidden');
  setText('#freezeApplyPreviewSummary', 'Формируется Strategy #1 в NFQWS_ARGS…');
  const diff = $('#freezeApplyPreviewDiff');
  if (diff) diff.textContent = '';
  try {
    const preview = await nfqwsDiagnostics.freezeStrategyPreview(operationId, index);
    renderStrategyPickPreview(preview, 'freezeScan', index, 'freeze');
  } catch (error) {
    panel?.classList.add('hidden');
    toast(error instanceof Error ? error.message : 'Предпросмотр стратегии недоступен.', 'error');
  }
}

async function applyFreezeStrategyOneClick(operationId: string, resultIndex: string) {
  const index = Number(resultIndex);
  const actionKey = `${operationId}:${resultIndex}`;
  if (freezeStrategyApplyPending.has(actionKey) || freezeStrategyApplyDone.has(actionKey)) return;
  const rowButton = [...document.querySelectorAll<HTMLButtonElement>('[data-action-click="applyFreezeStrategyOneClick"]')]
    .find((button) => button.dataset.args === JSON.stringify([operationId, resultIndex]));
  if (!Number.isInteger(index) || index < 0) return toast('Индекс проверенного домена некорректен.', 'error');
  const editor = $('#nfqwsConfTextarea');
  if (config && editor instanceof HTMLTextAreaElement && editor.value !== config.content) {
    return toast('Сначала примените или отмените черновик конфигурации.', 'error');
  }
  freezeStrategyApplyPending.add(actionKey);
  if (rowButton) { rowButton.disabled = true; rowButton.textContent = 'Проверяю и применяю…'; }
  let appliedSuccessfully = false;
  try {
    // The explicit row action is the user's approval. The backend still
    // verifies the persisted 3/3 result, target domain, config revision and
    // nfqws2 dry-run before writing; its transaction backs up and rolls back.
    const preview = await nfqwsDiagnostics.freezeStrategyPreview(operationId, index);
    const applied = await nfqwsDiagnostics.applyStrategy('freezeScan', operationId, index, preview.originalRevision);
    appliedSuccessfully = true;
    freezeStrategyApplyDone.add(actionKey);
    activeStrategyPreview = null;
    if (rowButton) { rowButton.textContent = 'Стратегия применена'; rowButton.title = `${preview.host}: профиль добавлен; резервная копия создана`; }
    toast(`${applied.message} Стратегия добавлена только для ${preview.host}; резервная копия сохранена.`, 'success');
    try { await loadConfig(); }
    catch { toast('Стратегия применена, но данные конфигурации на странице не обновились. Перезагрузите панель.', 'error'); }
  } catch (error) {
    if (rowButton) { rowButton.disabled = appliedSuccessfully; rowButton.textContent = appliedSuccessfully ? 'Стратегия применена' : 'Применить к сайту'; }
    toast(error instanceof Error ? error.message : 'Не удалось применить подтверждённую стратегию.', 'error');
  } finally {
    freezeStrategyApplyPending.delete(actionKey);
  }
}

async function applyStrategyPick(source: string, operationId: string, candidateIndex: string, expectedRevision: string) {
  if ((source !== 'strategyPick' && source !== 'freezeScan') || !activeStrategyPreview
    || activeStrategyPreview.source !== source || activeStrategyPreview.operationId !== operationId) {
    return toast('Сначала откройте актуальный предпросмотр стратегии.', 'error');
  }
  const summaryId = activeStrategyPreview.panel === 'freeze' ? '#freezeApplyPreviewSummary' : '#autotunePreviewSummary';
  const buttonId = activeStrategyPreview.panel === 'freeze' ? '#applyFreezeStrategyButton' : '#applyStrategyPickButton';
  if (activeStrategyPreview.candidateIndex !== Number(candidateIndex) || activeStrategyPreview.revision !== expectedRevision) return toast('Предпросмотр устарел. Откройте его снова.', 'error');
  const editor = $('#nfqwsConfTextarea');
  if (config && editor instanceof HTMLTextAreaElement && editor.value !== config.content) return toast('Сначала примените или отмените черновик конфигурации. Стратегия не применена.', 'error');
  const applyScope = source === 'strategyPick'
    ? 'Добавить подтверждённую стратегию как Strategy #1 в NFQWS_ARGS и сдвинуть прежние номера вниз? Это повлияет на все сайты; NFQWS_BASE_ARGS не меняется.'
    : `Добавить профиль только для ${$(summaryId)?.textContent?.split(' · ')[0] ?? 'проверенного домена'}?`;
  if (!window.confirm(`${applyScope}\n\nnfqws2 перезапустится. Будет создана резервная копия; при ошибке проверки конфигурация откатится.`)) return;
  const button = $(buttonId);
  if (button instanceof HTMLButtonElement) { button.disabled = true; button.textContent = 'Применение…'; }
  let wasApplied = false;
  try {
    const applied = await nfqwsDiagnostics.applyStrategy(source, operationId, Number(candidateIndex), expectedRevision);
    wasApplied = true;
    activeStrategyPreview = null;
    try { await loadConfig(); }
    catch { toast('Стратегия применена, но страницу конфигурации не удалось обновить. Перезагрузите панель.', 'error'); return; }
    toast(source === 'strategyPick'
      ? `${applied.message} Стратегия добавлена как Strategy #1; прежние номера сдвинуты. Резервная копия сохранена.`
      : `${applied.message} Доменный профиль добавлен; резервная копия сохранена.`, 'success');
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось применить профиль.', 'error');
  } finally {
    if (button instanceof HTMLButtonElement) {
      button.disabled = wasApplied;
      button.textContent = wasApplied ? 'Стратегия применена' : activeStrategyPreview?.panel === 'freeze' ? 'Применить и перезапустить' : 'Применить стратегию и перезапустить';
    }
  }
}

async function loadStrategyPickHistory() {
  const container = $('#autotuneConsoleBox');
  if (!container) return;
  try {
    const history = await nfqwsDiagnostics.strategyHistory();
    container.replaceChildren();
    if (!history.length) {
      const empty = document.createElement('p'); empty.className = 'text-slate-400'; empty.textContent = 'Завершённых подборов пока нет.'; container.append(empty);
      return;
    }
    for (const operation of history) {
      const button = document.createElement('button');
      button.type = 'button'; button.className = 'w-full rounded-lg border border-slate-800 bg-[#060a12] p-3 text-left hover:border-slate-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400';
      button.setAttribute('aria-label', `Открыть результаты подбора для ${operation.host}`);
      const title = document.createElement('span'); title.className = 'block truncate font-semibold text-slate-200'; title.textContent = operation.host;
      const result = operation.results.filter((item) => item.confirmed).length;
      const state = document.createElement('span'); state.className = `mt-1 block text-[11px] ${result ? 'text-emerald-300' : 'text-slate-400'}`;
      state.textContent = `${result} подтверждено · ${operation.done}/${operation.total} · ${new Date(operation.updatedAt * 1000).toLocaleString()}`;
      button.append(title, state);
      button.addEventListener('click', () => {
        if (strategyPickRunning) return;
        const input = $('#scanCustomHost');
        if (input instanceof HTMLInputElement) input.value = operation.host;
        localStorage.setItem(STRATEGY_PICK_STORAGE, operation.id);
        showRoute('autotune');
        renderStrategyPick(operation);
        setText('#scanProgressLabel', operation.message);
      });
      container.append(button);
    }
  } catch (error) {
    container.replaceChildren();
    const message = document.createElement('p'); message.className = 'text-xs text-amber-300';
    message.textContent = error instanceof Error ? `История недоступна: ${error.message}` : 'История подборов недоступна';
    container.append(message);
  }
}

async function pollStrategyPick(id: string, notifyOnCompletion = true) {
  activeStrategyPickId = id; strategyPickRunning = true;
  const input = $('#scanCustomHost'); const start = $('#startScanBtn'); const stop = $('#strategyPickCancel');
  if (input instanceof HTMLInputElement) input.disabled = true;
  if (start instanceof HTMLButtonElement) { start.disabled = true; start.textContent = 'Подбор…'; }
  stop?.classList.remove('hidden');
  try {
    let current = await nfqwsDiagnostics.strategyStatus(id);
    let consecutiveFailures = 0;
    renderStrategyPick(current);
    while (['queued', 'running', 'stopping'].includes(current.state)) {
      await new Promise((resolve) => window.setTimeout(resolve, 1000));
      try {
        current = await nfqwsDiagnostics.strategyStatus(id);
        consecutiveFailures = 0;
        renderStrategyPick(current);
      } catch (error) {
        consecutiveFailures += 1;
        setText('#scanProgressLabel', `${current.message} · переподключение…`);
        if (consecutiveFailures >= 10) throw error;
      }
    }
    renderStrategyPick(current);
    setText('#scanProgressLabel', current.message);
    if (['complete', 'partial', 'no_match', 'site_unblocked', 'cancelled', 'failed'].includes(current.state)) {
      void loadStrategyPickHistory();
      if (notifyOnCompletion) {
        if (current.state === 'failed') toast(current.message, 'error');
        else toast(current.message, ['complete', 'partial'].includes(current.state) ? 'success' : 'error');
      }
    }
  } catch (error) {
    setText('#scanProgressLabel', error instanceof Error ? error.message : 'Статус автоподбора пока недоступен');
  } finally {
    strategyPickRunning = false; activeStrategyPickId = null;
    if (input instanceof HTMLInputElement) input.disabled = false;
    if (start instanceof HTMLButtonElement) { start.disabled = false; start.textContent = 'Подобрать стратегию'; }
    stop?.classList.add('hidden');
  }
}

async function startStrategyPick() {
  if (strategyPickRunning) return;
  const input = $('#scanCustomHost'); const proto = $('#strategyProtocolSelect'); const mode = $('#strategyModeSelect'); const repeatsInput = $('#strategyRepeatsSelect'); const customInput = $('#strategyCustomSteps');
  if (!(input instanceof HTMLInputElement) || !input.value.trim()) return toast('Введите домен или публичный IPv4 для подбора.', 'error');
  const protocol = proto instanceof HTMLSelectElement && ['http', 'https'].includes(proto.value) ? proto.value as 'http' | 'https' : 'https';
  const selectedMode = mode instanceof HTMLSelectElement && ['extended', 'full', 'custom'].includes(mode.value) ? mode.value as 'extended' | 'full' | 'custom' : 'quick';
  const repeats = repeatsInput instanceof HTMLSelectElement ? Number(repeatsInput.value) : 3;
  const stopAfterFirst = ($('#strategyStopAfterFirst') as HTMLInputElement | null)?.checked ?? false;
  const includeCommunity = ($('#strategyIncludeCommunity') as HTMLInputElement | null)?.checked ?? false;
  const includeOtherHistory = ($('#strategyIncludeOtherHistory') as HTMLInputElement | null)?.checked ?? false;
  const refineLighter = ($('#strategyRefineLighter') as HTMLInputElement | null)?.checked ?? false;
  if (!Number.isInteger(repeats) || repeats < 2 || repeats > 5) return toast('Число повторов должно быть от 2 до 5.', 'error');
  let customSteps: string[][] = [];
  try { customSteps = customInput instanceof HTMLTextAreaElement ? parseCustomStrategies(customInput.value) : []; }
  catch (error) { return toast(error instanceof Error ? error.message : 'Не удалось прочитать собственные стратегии.', 'error'); }
  if (selectedMode === 'custom' && customSteps.length === 0) return toast('Добавьте хотя бы один параметр --lua-desync=…', 'error');
  const container = $('#autotuneStrategiesList'); container?.replaceChildren();
  $('#autotunePreviewPanel')?.classList.add('hidden');
  activeStrategyPreview = null;
  const applyButton = $('#applyStrategyPickButton'); if (applyButton instanceof HTMLButtonElement) applyButton.disabled = true;
  try {
    const operation = await nfqwsDiagnostics.strategyStart(input.value.trim(), protocol, selectedMode, repeats, selectedMode === 'custom' ? customSteps : undefined, stopAfterFirst, includeCommunity, includeOtherHistory, refineLighter);
    activeStrategyPickId = operation.id; localStorage.setItem(STRATEGY_PICK_STORAGE, operation.id);
    await pollStrategyPick(operation.id);
  } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось запустить автоподбор', 'error'); }
}

async function cancelStrategyPick() {
  if (!activeStrategyPickId) return;
  const stop = $('#strategyPickCancel'); if (stop instanceof HTMLButtonElement) { stop.disabled = true; stop.textContent = 'Останавливаю…'; }
  try { const operation = await nfqwsDiagnostics.strategyCancel(activeStrategyPickId); setText('#scanProgressLabel', operation.message); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось остановить автоподбор', 'error'); }
  finally { if (stop instanceof HTMLButtonElement) { stop.disabled = false; stop.textContent = 'Остановить'; } }
}

function freezeMeasurementLabel(measurement: FreezeScanOperation['results'][number]['current']) {
  if (measurement.ok) return `ответ получен · ${measurement.elapsedMs} мс`;
  if (measurement.reason.includes('TCP-соединение отдельно устанавливается')) return `тайм-аут · TCP доступен · ${measurement.elapsedMs} мс`;
  if (measurement.classification === 'PossibleFreeze') return `тайм-аут после ответа · ${measurement.elapsedMs} мс`;
  if (measurement.classification === 'PossibleBoundaryStall') return `объёмный тайм-аут · ${measurement.elapsedMs} мс`;
  if (measurement.classification === 'PossibleUploadStall') return `тайм-аут при отправке · ${measurement.elapsedMs} мс`;
  if (measurement.classification === 'TimeoutBeforeResponse' || measurement.classification === 'TimeoutBeforeBody') return `тайм-аут до тела ответа · ${measurement.elapsedMs} мс`;
  if (measurement.classification === 'ServerDoesNotKeepConnection') return 'сервер не удерживает соединение';
  if (measurement.classification === 'ConnectFailure') return 'TCP-соединение не установлено';
  if (measurement.classification === 'TlsFailure') return 'ошибка TLS';
  if (measurement.classification === 'Reset') return 'соединение сброшено';
  if (measurement.classification === 'dns_failure') return 'DNS не разрешил адрес';
  return measurement.reason || 'результат не определён';
}

function renderFreezeScan(operation: FreezeScanOperation) {
  const progress = $('#freezeScanProgress');
  const active = ['queued', 'running', 'stopping'].includes(operation.state);
  if (progress) {
    progress.classList.toggle('hidden', !active);
    if (active) {
      const view = freezeProgressPresentation(operation);
      setText('#freezeScanProgressTitle', view.headline);
      setText('#freezeScanProgressPercent', `${view.percent}%`);
      setText('#freezeScanProgressDetail', view.detail);
      const bar = $('#freezeScanProgressBar');
      if (bar instanceof HTMLElement) bar.style.width = `${view.percent}%`;
      const steps = $('#freezeScanProgressSteps');
      if (steps) {
        steps.replaceChildren();
        view.steps.forEach((step, index) => {
          const item = document.createElement('li');
          item.className = `freeze-progress-step rounded-lg border px-2.5 py-2 text-xs ${step.state}`;
          item.dataset.state = step.state;
          const number = document.createElement('span');
          number.className = 'mr-2 inline-flex h-5 w-5 items-center justify-center rounded-full border text-[10px] font-bold';
          number.textContent = step.state === 'complete' ? '✓' : String(index + 1);
          const label = document.createElement('span');
          label.textContent = step.label;
          item.append(number, label);
          steps.append(item);
        });
      }
    }
  }
  const status = $('#freezeScanStatus');
  if (status) {
    status.textContent = active
      ? `${operation.stage === 'strategy_search' ? 'Подбор имени' : 'Проверка'} · ${operation.current ?? 'подготовка'} · ${operation.done}/${operation.total}`
      : '';
    status.setAttribute('aria-busy', String(active));
    status.classList.toggle('hidden', !active);
  }
  const summary = $('#freezeScanSummary');
  const summaryValue = summarizeFreezeScan(operation);
  if (summary) {
    if (summaryValue) {
      summary.textContent = summaryValue.text;
      summary.className = `rounded-xl border px-4 py-3 text-sm font-bold leading-relaxed ${summaryValue.tone === 'warning' ? 'border-amber-400/25 bg-amber-500/15 text-amber-200' : summaryValue.tone === 'success' ? 'border-emerald-400/25 bg-emerald-500/15 text-emerald-200' : summaryValue.tone === 'error' ? 'border-rose-400/25 bg-rose-500/15 text-rose-200' : 'border-slate-600 bg-slate-800/70 text-slate-200'}`;
    } else {
      summary.classList.add('hidden');
    }
  }
  const container = $('#freezeScanResults');
  if (!container) return;
  const resultsSignature = JSON.stringify({ state: operation.state, results: operation.results });
  if (container.dataset.resultsSignature === resultsSignature) return;
  container.dataset.resultsSignature = resultsSignature;
  container.replaceChildren();
  if (!operation.results.length) { container.classList.add('hidden'); return; }
  container.classList.remove('hidden');
  const table = document.createElement('table'); table.className = 'freeze-scan-table min-w-[760px] w-full text-left text-sm';
  const head = document.createElement('thead'); head.className = 'bg-slate-900/80 text-xs uppercase text-slate-300';
  const headerRow = document.createElement('tr');
  const labels = ['Сеть / имя проверки', 'Проверенный IP / ASN', 'Напрямую', 'Текущий маршрут', 'Итог'];
  for (const title of labels) { const cell = document.createElement('th'); cell.className = 'px-3 py-3 font-bold'; cell.textContent = title; headerRow.append(cell); }
  head.append(headerRow); table.append(head);
  const body = document.createElement('tbody'); body.className = 'divide-y divide-slate-800 text-slate-200';
  for (const [resultIndex, result] of operation.results.entries()) {
    const presentation = freezeRowPresentation({ freezeDetected: result.freezeDetected, foundStrategy: result.foundStrategy, directOk: result.direct.ok, currentOk: result.current.ok });
    const row = document.createElement('tr'); row.className = `freeze-result-row freeze-result-${presentation.tone} align-top hover:bg-slate-900/50`;
    row.dataset.resultTone = presentation.tone;
    const cells = [
      freezeRowIdentity(result),
      `${result.address ?? 'адрес не найден'}\n${result.asn.state === 'verified' ? `AS${result.asn.asn} · подтверждён` : result.asn.state === 'changed' ? `ASN изменился: ожидался AS${result.asn.expected}, сейчас AS${result.asn.actual}` : 'ASN не подтверждён'}`,
      freezeMeasurementLabel(result.direct),
      freezeMeasurementLabel(result.current),
      result.foundStrategy ? `${freezeRowDetails(result).summary}${result.foundStrategy.reusedFrom ? ` · ASN совпадает с ${result.foundStrategy.reusedFrom}` : ''}` : result.freezeDetected ? 'Проверки показывают возможный обрыв' : result.direct.ok && result.current.ok ? 'Обрыв не обнаружен' : 'Результат не подтверждён',
    ];
    cells.forEach((value, index) => {
      const cell = document.createElement('td'); cell.dataset.label = labels[index];
      cell.className = `whitespace-pre-line px-3 py-2.5 leading-snug ${index === 4 ? 'freeze-result-summary font-bold' : 'font-semibold text-white'}`;
      if (index === 4) {
        const badge = document.createElement('span');
        badge.className = 'freeze-result-badge mb-1 inline-flex w-fit items-center rounded-full border px-2.5 py-1 text-xs font-bold';
        badge.textContent = presentation.label;
        badge.setAttribute('role', 'status');
        const detail = document.createElement('span');
        detail.className = 'block whitespace-pre-line';
        detail.textContent = value;
        const facts = freezeRowDetails(result);
        cell.append(badge, detail);
        if (facts.details) {
          const disclosure = document.createElement('details');
          disclosure.className = 'freeze-result-details mt-1 text-xs font-medium';
          const summary = document.createElement('summary');
          summary.textContent = 'Подробности проверки';
          const content = document.createElement('span');
          content.className = 'mt-1 block whitespace-pre-line';
          content.textContent = facts.details;
          disclosure.append(summary, content);
          cell.append(disclosure);
        }
      } else {
        cell.textContent = value;
      }
      const applyKey = `${operation.id}:${resultIndex}`;
      if (index === 4 && !freezeStrategyApplyDone.has(applyKey) && freezeCanApplyToHost({ state: operation.state, targetKind: result.targetKind, freezeDetected: result.freezeDetected, foundStrategy: result.foundStrategy })) {
        const button = document.createElement('button');
        button.type = 'button';
        button.className = 'mt-2 rounded-lg border border-emerald-400/40 bg-emerald-500/15 px-3 py-1.5 text-xs font-bold text-emerald-100 transition hover:bg-emerald-500/25 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-300';
        button.dataset.actionClick = 'applyFreezeStrategyOneClick';
        button.dataset.args = JSON.stringify([operation.id, String(resultIndex)]);
        button.disabled = freezeStrategyApplyPending.has(applyKey);
        button.textContent = button.disabled ? 'Проверяю и применяю…' : 'Применить к сайту';
        button.title = 'Проверенный вариант добавляется только этому домену; при ошибке конфигурация откатывается';
        cell.append(button);
      }
      row.append(cell);
    });
    body.append(row);
  }
  table.append(body); container.append(table);
}

async function pollFreezeScan(id: string) {
  activeFreezeScanId = id; freezeScanRunning = true;
  const input = $('#freezeScanHostsInput'); const start = $('[data-action-click="runFreezeScan"]'); const stop = $('#freezeScanCancel');
  if (input instanceof HTMLInputElement) input.disabled = true;
  if (start instanceof HTMLButtonElement) start.disabled = true;
  stop?.classList.remove('hidden');
  try {
    let current = await nfqwsDiagnostics.freezeScanStatus(id); renderFreezeScan(current);
    for (let index = 0; index < 920 && ['queued', 'running', 'stopping'].includes(current.state); index += 1) {
      await new Promise((resolve) => window.setTimeout(resolve, 1000));
      try { current = await nfqwsDiagnostics.freezeScanStatus(id); renderFreezeScan(current); }
      catch { setText('#freezeScanStatus', 'Связь с роутером восстанавливается…'); }
    }
    renderFreezeScan(current);
  } catch (error) { setText('#freezeScanStatus', error instanceof Error ? error.message : 'Статус проверки сетей недоступен'); }
  finally {
    freezeScanRunning = false; activeFreezeScanId = null;
    if (input instanceof HTMLInputElement) input.disabled = false;
    if (start instanceof HTMLButtonElement) start.disabled = false;
    stop?.classList.add('hidden');
  }
}

async function startFreezeScan() {
  if (freezeScanRunning) return;
  const input = $('#freezeScanHostsInput');
  if (!(input instanceof HTMLInputElement)) return;
  const extraHosts = input.value.split(',').map((host) => host.trim()).filter(Boolean);
  if (extraHosts.length > 20) return toast('Можно добавить не более 20 своих доменов.', 'error');
  const results = $('#freezeScanResults');
  results?.replaceChildren();
  $('#freezeApplyPreviewPanel')?.classList.add('hidden');
  activeStrategyPreview = null;
  const applyButton = $('#applyFreezeStrategyButton'); if (applyButton instanceof HTMLButtonElement) applyButton.disabled = true;
  if (results) delete results.dataset.resultsSignature;
  $('#freezeScanSummary')?.classList.add('hidden');
  try { const operation = await nfqwsDiagnostics.freezeScanStart(extraHosts); localStorage.setItem('nfqws2-ui-freeze-scan', operation.id); await pollFreezeScan(operation.id); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось запустить проверку сетей', 'error'); }
}

async function cancelFreezeScan() {
  if (!activeFreezeScanId) return;
  const stop = $('#freezeScanCancel'); if (stop instanceof HTMLButtonElement) { stop.disabled = true; stop.textContent = 'Останавливаю…'; }
  try { const operation = await nfqwsDiagnostics.freezeScanCancel(activeFreezeScanId); setText('#freezeScanStatus', operation.message); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось остановить проверку сетей', 'error'); }
  finally { if (stop instanceof HTMLButtonElement) { stop.disabled = false; stop.textContent = 'Остановить'; } }
}

export const DISABLED_PANEL_ACTIONS = ['startStrategyAutoScan','applyStrategyToConf','applyCommunityPreset','fetchAsnPrefixes','pushBackupToNas','exportFullBackup','excludeBrokenDomain'] as const;

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
    const expanded = expandedReleaseTag === release.tag;
    const card = document.createElement('article');
    card.className = `release-version-card rounded-xl border transition-colors ${selected ? 'is-selected' : ''}`;
    const header = document.createElement('button');
    header.type = 'button';
    header.dataset.actionClick = 'selectVersionItem';
    header.dataset.args = JSON.stringify([release.tag]);
    header.setAttribute('aria-pressed', String(selected));
    header.setAttribute('aria-expanded', String(expanded));
    header.setAttribute('aria-controls', `release-notes-${CSS.escape(release.tag)}`);
    header.className = 'release-version-header';
    const title = document.createElement('span');
    title.className = 'block text-sm font-bold text-slate-100';
    title.textContent = release.tag;
    header.append(title);
    if (updateTarget === 'core' && release.tag === installedCoreTag) {
      const installed = document.createElement('span');
      installed.className = 'ml-2 rounded bg-emerald-900/70 px-2 py-0.5 text-[10px] font-semibold text-emerald-300';
      installed.textContent = 'Установлена';
      title.append(installed);
    }
    const date = document.createElement('span');
    date.className = 'mt-1 block text-xs text-slate-400';
    date.textContent = release.publishedAt ? new Date(release.publishedAt).toLocaleDateString() : 'Дата публикации не указана';
    header.append(date);
    const radio = document.createElement('span');
    radio.setAttribute('aria-hidden', 'true');
    radio.className = `absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 rounded-full border-2 ${selected ? 'border-blue-300 bg-blue-500' : 'border-slate-500'}`;
    header.append(radio);
    card.append(header);
    const notes = document.createElement('div');
    notes.id = `release-notes-${CSS.escape(release.tag)}`;
    notes.className = 'release-markdown-scroll';
    notes.hidden = !expanded;
    notes.append(renderReleaseMarkdown(release.notes));
    card.append(notes);
    list.append(card);
  }
  if (install instanceof HTMLButtonElement) install.disabled = !selectedReleaseTag || updatePolling || (updateTarget === 'core' && selectedReleaseTag === installedCoreTag);
  setText('#updateModalTitle', updateTarget === 'core' ? 'Обновление nfqws2' : 'Обновление nfqws2-UI');
  setText('#updatePickerHint', updateTarget === 'core' ? 'Выберите версию ядра nfqws2' : 'Выберите версию панели');
  setText('#updateRepoSlug', updateTarget === 'core' ? 'nfqws/nfqws2-keenetic' : 'rangerxxyy/nfqws2-ui');
}

async function openUpdatePicker(target: 'ui' | 'core' = 'ui') {
  updateTarget = target;
  expandedReleaseTag = null;
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
      renderUiUpdateNotice(result.updateStatus, true);
    }
    selectedReleaseTag = availableReleases[0]?.tag ?? null;
    expandedReleaseTag = selectedReleaseTag;
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
    if (current.state === 'success') schedulePanelRefreshAfterUpdate(lifecycleMessage(core ? 'core-update' : 'panel-update', tag));
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

function updateStrategyMode(mode: string) {
  $('#strategyCustomInputPanel')?.classList.toggle('hidden', mode !== 'custom');
  $('#strategyModeSelect')?.closest('.strategy-mode-tile')?.classList.toggle('strategy-mode-tile-custom', mode === 'custom');
  const help = $('#strategyModeHelp');
  if (!help) return;
  help.textContent = ({
    quick: 'Текущий профиль и стандартный набор: до 12 HTTPS- или 4 HTTP-кандидатов до фильтра совместимости Lua. «Проверено X из Y» — стратегии, а не запросы; повторы указаны отдельно. Если первая попытка кандидата не удалась, лишние повторы пропускаются.',
    extended: 'Расширенный каталог blockcheck2: только совместимые с установленным nfqws2 варианты, проверка по одному.',
    full: 'Полный каталог blockcheck2 — около 3 000 вариантов. Проверка идёт по одному, занимает дольше; её можно остановить в любой момент.',
    custom: 'Будет проверена только ваша стратегия; backend заранее сверит Lua-функции и blob-файлы.',
  } as Record<string, string>)[mode] ?? 'Выберите режим тестирования.';
  help.classList.remove('mode-help-enter');
  void help.offsetWidth;
  help.classList.add('mode-help-enter');
}

function copyStrategyText(text: string, button: HTMLButtonElement) {
  writeTextToClipboard(text).then(() => {
    const original = 'Копировать';
    button.textContent = 'Скопировано';
    button.classList.add('strategy-copy-success');
    window.setTimeout(() => {
      button.textContent = original;
      button.classList.remove('strategy-copy-success');
    }, 1500);
  }).catch(() => toast('Буфер обмена недоступен', 'error'));
}

async function writeTextToClipboard(text: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const helper = document.createElement('textarea');
  helper.value = text;
  helper.setAttribute('readonly', '');
  helper.style.position = 'fixed';
  helper.style.opacity = '0';
  document.body.append(helper);
  helper.select();
  const copied = document.execCommand('copy');
  helper.remove();
  if (!copied) throw new Error('Буфер обмена недоступен');
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
    createBackup: () => void createSnapshot(),
    createSnapshot: () => void createSnapshot(),
    refreshBackups: () => void loadBackups(),
    restoreSnapshot: (id: string) => void restoreSnapshot(id),
    pinSnapshot: (id: string, pinned: string | boolean) => void (async () => { try { await nfqwsSnapshots.pin(id, pinned === true || pinned === 'true'); await loadBackups(); } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось закрепить снимок.', 'error'); } })(),
    saveSnapshotRetention: () => void saveSnapshotSettings().catch((error) => toast(error instanceof Error ? error.message : 'Не удалось сохранить срок хранения.', 'error')),
    saveSnapshotRemote: () => void saveSnapshotSettings(true).catch((error) => toast(error instanceof Error ? error.message : 'Не удалось сохранить настройки NAS.', 'error')),
    testSnapshotRemote: () => void runSnapshotRemote('test'),
    pushSnapshotRemote: () => void runSnapshotRemote('push'),
    createSnapshotKey: () => void runSnapshotRemote('key'),
    restoreSnapshotFile: (input: HTMLInputElement) => void restoreSnapshotFile(input),
    copySnapshotSetup: () => { const command = $<HTMLTextAreaElement>('#snapshotSetupCommand'); if (command) void navigator.clipboard.writeText(command.value).then(() => toast('Команда скопирована')).catch(() => toast('Буфер обмена недоступен', 'error')); },
    setAppearanceTheme: (theme: UiTheme) => updateAppearance({ theme }),
    setAppearanceLayout: (layout: UiLayout) => updateAppearance({ layout }),
    setAppearanceAccent: (accent: AppearanceSettings['accent']) => updateAppearance({ accent }),
    setAppearanceStatusBar: (statusBar: AppearanceSettings['statusBar']) => updateAppearance({ statusBar }),
    setAppearanceTimestampReminder: (remindWindowsTcpTimestamps: boolean) => updateAppearance({ remindWindowsTcpTimestamps }),
    checkUiUpdate: () => void checkUiUpdate(),
    showMonitorSection: (section: 'sites' | 'telegram') => showMonitorSection(section),
    checkMonitoringSite: (host: string) => void runMonitoring(host),
    saveMonitoringSettings: () => void saveMonitoringSettings().catch((e) => toast(e instanceof Error ? e.message : 'Не удалось сохранить расписание.', 'error')),
    toggleMonitoringCustom: (select: HTMLSelectElement) => { $('#monitoringCustom')?.classList.toggle('hidden', select.value !== 'custom'); $('#monitoringCustom')?.classList.toggle('inline-flex', select.value === 'custom'); },
    saveTelegramSettings: () => void saveTelegramSettings().catch((e) => toast(e instanceof Error ? e.message : 'Не удалось сохранить Telegram.', 'error')),
    toggleTelegramTransportFields: (via: string) => toggleTelegramTransportFields(via),
    sendTestTelegram: () => void sendTestTelegram().catch((e) => toast(e instanceof Error ? e.message : 'Не удалось отправить сообщение Telegram.', 'error')),
    findTelegramChatId: () => void findTelegramChatId().catch((e) => toast(e instanceof Error ? e.message : 'Не удалось найти чаты Telegram.', 'error')),
    addMonitoringSite: () => addMonitoringSite(),
    runMonitoring: () => runMonitoring(),
    removeMonitoringSite: (host: string) => {
      if (!monitoringState || monitoringRunning) return;
      monitoringState = { ...monitoringState, sites: monitoringState.sites.filter((site) => site.host !== host) };
      persistMonitoring();
    },
    switchConfigSection: (section: 'editor' | 'settings' | 'scripts') => switchConfigSection(section),
    refreshLuaScripts: () => void loadLuaScripts(),
    runSiteDiagnosis: () => void runSiteDiagnosis(),
    runSiteTrace: () => void runSiteTrace(),
    stopSiteTrace: () => void stopSiteTrace(),
    openForeignConfigText: () => showForeignConfigForm('text'),
    openForeignConfigUrl: () => showForeignConfigForm('url'),
    closeForeignConfigForms: () => closeForeignConfigForms(),
    addForeignConfigText: () => addForeignConfigText(),
    addForeignConfigUrl: () => addForeignConfigUrl(),
    runForeignConfigCheck: () => void runForeignConfigCheck(),
    cancelForeignConfigCheck: () => void cancelForeignConfigCheck(),
    startStrategyPick: () => void startStrategyPick(),
    cancelStrategyPick: () => void cancelStrategyPick(),
    toggleStrategyCustomInput: (mode: string) => updateStrategyMode(mode),
    copyStrategyParams: (button: HTMLButtonElement) => {
      const input = $('#strategyCustomSteps');
      if (!(input instanceof HTMLTextAreaElement)) return;
      copyStrategyText(input.value, button);
    },
    copyStrategyResult: (text: string, button: HTMLButtonElement) => copyStrategyText(text, button),
    previewStrategyPick: (index: string) => void previewStrategyPick(index),
    previewFreezeStrategy: (operationId: string, index: string) => void previewFreezeStrategy(operationId, index),
    applyFreezeStrategyOneClick: (operationId: string, index: string) => void applyFreezeStrategyOneClick(operationId, index),
    applyStrategyPick: (source: string, operationId: string, index: string, revision: string) => void applyStrategyPick(source, operationId, index, revision),
    runFreezeScan: () => void startFreezeScan(),
    cancelFreezeScan: () => void cancelFreezeScan(),
    setDiagPreset: (host: string) => { const input = $('#diagDomainInput'); if (input instanceof HTMLInputElement && !diagnosisRunning) { input.value = host; input.focus(); } },
    restartService: () => void serviceAction('restart'),
    toggleDaemonPower: () => void serviceAction(serviceControlPresentation(status?.service.state).action),
    applyConfigAndRestart: () => void saveConfigAndApply(),
    copyConfigText: () => { const editor = $('#nfqwsConfTextarea'); if (editor instanceof HTMLTextAreaElement) void navigator.clipboard.writeText(editor.value).then(() => toast('Конфигурация скопирована')).catch(() => toast('Буфер обмена недоступен', 'error')); },
    updateConfVariable: (key: string, value: string) => void updateConfigVariable(key, value),
    copySnippetToClipboard: (value: string) => void navigator.clipboard.writeText(String(value)).then(() => toast('Фрагмент скопирован')).catch(() => toast('Буфер обмена недоступен', 'error')),
    handleConfInput: () => { syncConfigLines(); renderConfigOutline(); },
    syncConfScroll: () => syncConfigLines(),
    toggleWordWrapConf: (enabled: boolean) => { const editor = $('#nfqwsConfTextarea'); if (editor instanceof HTMLTextAreaElement) { editor.style.whiteSpace = enabled ? 'pre-wrap' : 'pre'; syncConfigHighlight(editor); } },
    toggleConfigOutline: () => toggleConfigOutline(),
    scrollToConfMarker: (marker: string) => { const editor = $('#nfqwsConfTextarea'); if (!(editor instanceof HTMLTextAreaElement)) return; const offset = editor.value.toLowerCase().indexOf(String(marker).toLowerCase()); if (offset < 0) return toast(`Секция ${marker} не найдена`, 'error'); editor.scrollTop = editor.value.slice(0, offset).split('\n').length * 23; },
    toggleDiffModal: () => { const modal = $('#diffModalBackdrop'); if (modal && !modal.classList.contains('hidden')) { modal.classList.add('hidden'); modal.classList.remove('flex'); } else showDiff(); },
    resetDefaultConf: () => void loadConfig(),
    closeUpdateModal: () => { const modal = $('#updateModalBackdrop'); modal?.classList.add('hidden'); modal?.classList.remove('flex'); },
    openUpdateModal: (target: 'ui' | 'core') => void openUpdatePicker(target),
    openLatestUiUpdate: (tag: string) => { uiUpdateNoticeOpenedTag = tag; hideUiUpdateNotice(); void openUpdatePicker('ui'); },
    dismissUiUpdateNotice: (tag: string) => dismissUiUpdateNotice(tag),
    selectVersionItem: (tag: string) => { if (availableReleases.some((release) => release.tag === tag) && !updatePolling) { const toggling = expandedReleaseTag === tag; selectedReleaseTag = tag; expandedReleaseTag = toggling ? null : tag; renderReleases(); } },
    installSelectedRelease: () => void installSelectedRelease(),
    filterDocs: (query: string) => { const box = $('#docItemsContainer'); if (box) box.textContent = query ? 'Справочник флагов не подключён.' : 'Справочник флагов не загружен.'; },
    syncListLineNumbers: (editor?: HTMLTextAreaElement) => syncListLineNumbers(editor),
    syncListScroll: (editor: HTMLTextAreaElement) => syncListScroll(editor),
    syncCustomHostInput: () => undefined,
    selectListFile: (name: NfqwsListId) => void loadList(name),
    setListEditorView: (view: 'text' | 'rows') => setListEditorView(view),
    requestDeleteCustomList: (id: string) => requestDeleteCustomList(id),
    confirmDeleteCustomList: (id: string) => void deleteCustomList(id),
    cancelDeleteCustomList: () => closeCustomListDeleteModal(),
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
      selectedListRows.clear();
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
  initializeAppearanceSettings();
  setListEditorView(listEditorView);
  installStyledSelects(document);
  installActionTooltips(document);
  monitoringState = loadMonitoringState(getMonitoringStorage());
  renderMonitoringSites();
  void loadBackendMonitoring().catch(() => setText('#monitoringStatus', 'Не удалось загрузить настройки мониторинга с роутера.'));
  void loadTelegramSettings().catch(() => setText('#telegramStatus', 'Не удалось загрузить настройки Telegram с роутера.'));
  window.setInterval(() => {
    if (document.visibilityState === 'visible' && $('#tabContent-monitor:not(.hidden) #monitorSection-telegram:not(.hidden)')) {
      void loadTelegramNotifications().catch(() => {});
    }
  }, 30_000);
  bindPrototypeActions();
  const foreignFileInput = $('#foreignConfigFileInput');
  foreignFileInput?.addEventListener('change', () => {
    if (!(foreignFileInput instanceof HTMLInputElement)) return;
    const file = foreignFileInput.files?.[0];
    if (file) void addForeignConfigFile(file);
    foreignFileInput.value = '';
  });
  renderForeignConfigInputs();
  document.documentElement.classList.add('dark');
  const initialStrategyMode = ($('#strategyModeSelect') as HTMLSelectElement | null)?.value ?? 'quick';
  updateStrategyMode(initialStrategyMode);
  showPendingUpdateNotice();
  showRoute('conf');
  const savedPick = localStorage.getItem(STRATEGY_PICK_STORAGE);
  if (savedPick) void pollStrategyPick(savedPick, false);
  const savedForeignConfigCheck = localStorage.getItem(FOREIGN_CONFIG_CHECK_STORAGE);
  if (savedForeignConfigCheck) void pollForeignConfigCheck(savedForeignConfigCheck);
  const savedFreezeScan = localStorage.getItem('nfqws2-ui-freeze-scan');
  if (savedFreezeScan) void pollFreezeScan(savedFreezeScan);
    void loadStrategyPickHistory();
  const applyButton = $('[data-action-click="applyConfigAndRestart"]');
  if (applyButton instanceof HTMLButtonElement) applyButton.disabled = true;
  $('#nfqwsConfTextarea')?.addEventListener('input', () => {
    syncConfigLines();
    renderConfigOutline();
    const editor = $('#nfqwsConfTextarea');
    if (editor instanceof HTMLTextAreaElement) updateQuickParameters(editor.value);
  });
  $('#listEditorTextarea')?.addEventListener('keydown', (event) => {
    const shortcut = event as KeyboardEvent;
    if (!(shortcut.ctrlKey || shortcut.metaKey) || shortcut.altKey || !(shortcut.code === 'Slash' || shortcut.key === '/')) return;
    const editor = shortcut.currentTarget;
    if (!(editor instanceof HTMLTextAreaElement) || editor.disabled || editor.readOnly) return;
    shortcut.preventDefault();
    const result = toggleListComments(editor.value, editor.selectionStart, editor.selectionEnd);
    if (result.content === editor.value) return;
    editor.value = result.content;
    editor.setSelectionRange(result.selectionStart, result.selectionEnd);
    editor.dispatchEvent(new Event('input', { bubbles: true }));
  });
  $('#listSearchInput')?.addEventListener('input', () => { if (listEditorView === 'rows') renderListRows(); });
  $('#nfqwsConfTextarea')?.addEventListener('scroll', syncConfigLines);
  $('#refreshStatusBtn')?.addEventListener('click', () => void fetchStatus());
  void fetchStatus();
  void refreshUiUpdateStatus();
  window.setInterval(() => { if (document.visibilityState === 'visible') void refreshUiUpdateStatus(); }, 5 * 60 * 1000);
  document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') void refreshUiUpdateStatus(); });
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
