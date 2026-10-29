import { diagnosisRows } from './diagnosis-layout';
import { parseCustomStrategies } from './custom-strategy-input';
import { comparisonVerdict, presentTrace, type Presentation } from './diagnosis-presenter';
import { summarizeFreezeScan } from './freeze-summary';
import { freezeCanApplyToHost, freezeCanOfferProfileJoin, freezeInConfigNote, freezeMeasurementDetails, freezeRowPresentation } from './freeze-presentation';
import { freezeProgressPresentation } from './freeze-progress';
import { uiUpdateNoticePresentation } from './ui-update-notice';
import { captureForeignConfigDisclosureState, formatForeignConfigProbeSummary } from './disclosure-state';
import { freezeRowDetails, freezeRowIdentity } from './freeze-row-text';
import { configAssignmentValue, configDraftDisposition, configEditorLayout, configEditorViewportHeight, configOutlineEntries, sameConfigText } from './config-values';
import '@fontsource-variable/inter';
import '@fontsource-variable/jetbrains-mono';
import './panel.css';
import { api, nfqwsApply, nfqwsSnapshots, nfqwsProfiles, nfqwsChangeHistory, nfqwsCommunityStrategies, nfqwsLogs, nfqwsAppearance, nfqwsAuth, nfqwsConfig, nfqwsDiagnostics, nfqwsInstallation, nfqwsLists, nfqwsScripts, nfqwsUpdates, nfqwsMonitoring, nfqwsDpiLab, type CommunityStrategiesResponse, type ConfigResponse, type NfqwsArgumentProfile, type NfqwsProfilesResponse, type NfqwsLuaCatalogResponse, type NfqwsLuaFunctionInfo, type NfqwsLuaStandardGroup, type ChangeHistoryEntry, type ConfigValidationResponse, type ForeignConfigCheckOperation, type FreezeProfileJoinPreview, type FreezeScanOperation, type NfqwsListId, type NfqwsListResponse, type NfqwsListSummary, type NfqwsLuaScriptInfo, type PanelRelease, type SiteDiagnosticReport, type SiteDiagnosticStep, type SiteTraceOperation, type SiteTraceReport, type SnapshotOverview, type StatusResponse, type StrategyPickOperation, type StrategyPickPreview, type UiUpdateStatus, type LogSource } from './api';
import { dpiCaptureAvailability, dpiExperimentSaveAvailability, dpiEvidencePresentation, dpiFindingStatus, dpiStrategyCorrelation, dpiNextSteps, formatDpiByteDump, formatDpiFlowByteRows, formatDpiTcpFlags, type DpiLabAnalysis, type DpiLabFlow, type DpiLabPacket, type DpiLabSavedExperiment, type DpiLabStrategyAnalysis } from './dpi-lab';
import { appendListEntries, deduplicateListContent, isListTabSelected, listIdFromTabElementId, listRowsFromContent, listTabIdsFromInventory, removeListLines, removeListLinesByIndices, toggleListComments } from './list-editor';
import { availableNfqwsModes } from './config-facts';
import { resourceUsagePercent, usageTone } from './metrics';
import { installStyledSelects } from './styled-select';
import { installActionTooltips } from './action-tooltips';
import { strategyNeedsWindowsTcpTimestamps } from './strategy-capability';
import { serviceControlPresentation } from './service-control';
import { buildCommunityStrategyIssueUrl } from './community-share';
import { communityProviderLabel, visibleCommunityProtocols } from './community-presentation';
import { renderReleaseMarkdown } from './release-markdown';
import { isSupportedForeignConfigFileName } from './foreign-config-file';
import { decodeForeignConfigText } from './foreign-config-text';
import { diagnosisProgressState, isSkippedDiagnosisStep, SITE_DIAGNOSIS_STAGES } from './diagnosis-stages';
import { canShowToast, CORE_UPDATE_CHECK_INTERVAL_MS, UPDATE_STATUS_POLL_INTERVAL_MS, coreUpdateNoticePresentation, lifecycleMessage, shouldCheckCoreUpdate, toastCardsToDismiss, toastPresentation } from './notifications';
import { podkopWarningPresentation } from './podkop-warning';
import { shouldPollLogs } from './log-polling';
import { shouldFollowLogTail } from './log-scroll';
import { loadMonitoringState, normalizeMonitoringHost, normalizeMonitoringInterval, saveMonitoringState, type MonitoringState } from './monitoring';
import { copyProfileIntoDraft } from './copy-profile';
import { appendArgumentToCustomProfile, appendProfileFilterValue, appendProfileProtocol, appendProfileUdpFilter, diagnoseProfileLuaArguments, getLuaProfileParameterOptions, getRuntimeProfileConstructorTarget, getRuntimeProfileListTarget, getRuntimeProfileComponentArgs, getRuntimeProfileSharedComponentNote, getRussianLuaArgumentDescription, getRussianLuaFunctionDescription, filterProfileArgumentChoices, moveProfileStepTo, profileStrategyNumbers, nextProfileChoiceIndex, parseLuaParameterOption, prepareVisibleRuntimeProfiles, profileArgumentChoices, profileFilterValues, profileOrderWarnings, profileProtocolChoices, removeProfileFilterValue, reorderProfileSteps, replaceDesyncFunction, replaceDesyncParameters, replaceProfileArgument, replaceProfileName, serializeProfileForSharing, splitDesyncParameters, updateDesyncParameter, updateProfileFilterValue, removeDesyncParameter, appendDesyncParameter, parseProfilePaste } from './profile-editor';
import { applyAppearance, loadAppearance, saveAppearance, type AppearanceSettings, type UiLayout, type UiTheme } from './appearance';
import { configLuaArgumentEntries, diagnoseConfigLuaArguments } from './config-diagnostics';
import { getConfigCodeEditor, mountConfigArgumentEditor, mountConfigCodeEditor, setConfigCodeEditorContent, setConfigCodeEditorWrapping } from './config-code-editor';
import { EditorView } from '@codemirror/view';
import { navigationGroupForRoute } from './navigation';
import { isValidNetworkInterfaceSelection, networkInterfaceChoices } from './network-interface';
import { snapshotFreeSpaceState } from './snapshot-storage';
import { formatListProfileUsageStatus, listValidationNotice } from './list-profile-usage';

type Route = 'conf' | 'lists' | 'check' | 'trace' | 'autotune' | 'configCheck' | 'dpiLab' | 'monitor' | 'profiles' | 'community' | 'snapshots' | 'changes' | 'logs' | 'appearance' | 'about';
let status: StatusResponse | null = null;
let config: ConfigResponse | null = null;
let luaScripts: NfqwsLuaScriptInfo[] = [];
let activeLuaScript: string | null = null;
let discoveredNetworkInterfaces: string[] = [];
let defaultNetworkInterfaces: string[] = [];
let fullscreenPlaceholder: Comment | null = null;
let listFullscreenPlaceholder: Comment | null = null;
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
const monitoringHostsRunning = new Set<string>();
let monitoringState: MonitoringState | null = null;
let monitoringIntervalMinutes = 30;
let appearanceSettings: AppearanceSettings = { theme: 'dark', layout: 'standard', accent: 'blue', statusBar: 'standard', remindWindowsTcpTimestamps: true, showConfigWarnings: true };
let configWarningsTimer: number | null = null;
const configArgumentViews = new Map<string, EditorView>();
let configWarningsRequest = 0;
let configWarningsCatalogPromise: Promise<void> | null = null;
let traceRunning = false;
let activeTraceOperationId: string | null = null;
let dpiLabAnalysis: DpiLabAnalysis | null = null;
let dpiCaptureAnalyzing = false;
let dpiCaptureSaving = false;
let selectedDpiPacketNumber: number | null = null;
let selectedDpiFlowIndex: number | null = null;
let dpiLabExperiments: DpiLabSavedExperiment[] = [];
let dpiStrategyAnalysis: DpiLabStrategyAnalysis | null = null;
let dpiAnalyzedStrategySource: string | null = null;
let dpiLabProfiles: NfqwsProfilesResponse | null = null;
let dpiAnalyzedFileIdentity: string | null = null;
let strategyPickRunning = false;
let activeStrategyPickId: string | null = null;
let strategyTargetMode: 'single' | 'multi' = 'single';
let communityStrategies: CommunityStrategiesResponse | null = null;
let communityLoading = false;
let communityAutoDetectionAttempted = false;
let communityProviderEditing = false;
let pendingCommunityShare: { operationId: string; candidateIndex: number; host: string; protocol: 'http' | 'https'; candidate: StrategyPickOperation['results'][number]; targetAsn: number | null } | null = null;
let communityShareLookupGeneration = 0;
let communityShareLookupTimer: number | null = null;
type ForeignConfigInput = { name: string; source: 'Текст' | 'Файл' | 'Ссылка'; content?: string; url?: string };
let foreignConfigInputs: ForeignConfigInput[] = [];
const expandedForeignConfigPreviews = new WeakSet<ForeignConfigInput>();
let activeForeignConfigCheckId: string | null = null;
let foreignConfigCheckRunning = false;
let freezeScanRunning = false;
let activeFreezeScanId: string | null = null;
const freezeStrategyApplyPending = new Set<string>();
const freezeStrategyApplyDone = new Set<string>();
let activeStrategyPreview: { source: 'strategyPick' | 'freezeScan'; operationId: string; candidateIndex: number; revision: string; panel: 'autotune' | 'freeze' } | null = null;
let activeFreezeJoinPreview: FreezeProfileJoinPreview | null = null;
let restartInProgress = false;
let availableReleases: PanelRelease[] = [];
let selectedReleaseTag: string | null = null;
let expandedReleaseTag: string | null = null;
let updatePolling = false;
let updateTarget: 'ui' | 'core' = 'ui';
const UPDATE_PICKER_REFRESH_INTERVAL_MS = 2 * 60 * 1000;
let updatePickerRefreshTimer: number | null = null;
let updatePickerRequestPending = false;
let installedCoreTag: string | null = null;
let strategyCustomCodeEditor: ReturnType<typeof mountConfigArgumentEditor> | null = null;
let runtimeProfileTextCodeEditor: ReturnType<typeof mountConfigArgumentEditor> | null = null;
let uiUpdateStatusPolling = false;
let uiUpdateNoticeOpenedTag: string | null = null;
const UPDATE_NOTICE_KEY = 'nfqws2-ui-update-notice';
const UI_UPDATE_DISMISSED_TAG_KEY = 'nfqws2-ui-update-dismissed-tag';
const CORE_UPDATE_DISMISSED_TAG_KEY = 'nfqws2-core-update-dismissed-tag';
function strategyCustomText(): string {
  const input = $('#strategyCustomSteps');
  return strategyCustomCodeEditor?.state.doc.toString() ?? (input instanceof HTMLTextAreaElement ? input.value : '');
}
function setStrategyCustomText(value: string): void {
  const input = $('#strategyCustomSteps');
  if (input instanceof HTMLTextAreaElement) input.value = value;
  if (strategyCustomCodeEditor && strategyCustomCodeEditor.state.doc.toString() !== value) {
    strategyCustomCodeEditor.dispatch({ changes: { from: 0, to: strategyCustomCodeEditor.state.doc.length, insert: value } });
  }
}
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
function dismissToast(item: HTMLElement) {
  if (!item.isConnected || item.dataset.removing === 'true') return;
  item.dataset.removing = 'true';
  item.classList.add('is-removing');
  window.setTimeout(() => item.remove(), 220);
}
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
  copy.append(title, detail); item.append(icon, copy);
  const toastCards = [...host.querySelectorAll<HTMLElement>(':scope > .toast-card')];
  toastCardsToDismiss(toastCards.map((element) => ({ element, removing: element.dataset.removing === 'true' })))
    .forEach(({ element }) => dismissToast(element));
  host.append(item);
  window.setTimeout(() => dismissToast(item), 5200);
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
function hideCoreUpdateNotice() { $('#coreUpdateNotice')?.remove(); }
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
  const host = $('#updateNoticeWrapper');
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
function renderCoreUpdateNotice(status: { latestTag: string | null; installedTag: string | null; updateAvailable: boolean }, manualCheck = false) {
  let dismissedTag: string | null = null;
  try { dismissedTag = window.localStorage.getItem(CORE_UPDATE_DISMISSED_TAG_KEY); } catch { /* Treat storage failure as not dismissed. */ }
  const dismissedVersion = status.latestTag && (dismissedTag === status.latestTag && !manualCheck) ? dismissedTag : null;
  const presentation = coreUpdateNoticePresentation(status, dismissedVersion);
  const button = $('#updateNfqws2Button');
  button?.classList.toggle('update-available', presentation.highlight);
  button?.setAttribute('aria-label', presentation.highlight
    ? `Доступно обновление nfqws2: ${presentation.version}`
    : `Версия nfqws2 ${status.installedTag ?? 'не определена'}`);
  if (!presentation.notify || !presentation.version) { hideCoreUpdateNotice(); return; }
  const host = $('#updateNoticeWrapper');
  if (!host || $('#coreUpdateNotice')?.dataset.version === presentation.version) return;
  hideCoreUpdateNotice();
  const item = document.createElement('section');
  item.id = 'coreUpdateNotice'; item.dataset.version = presentation.version;
  item.className = 'ui-update-notice pointer-events-auto relative w-[22.5rem] max-w-[calc(100vw-2.5rem)] rounded-xl border px-4 py-3 text-slate-100 shadow-2xl';
  item.setAttribute('role', 'status'); item.setAttribute('aria-live', 'polite');
  const heading = document.createElement('div'); heading.className = 'flex min-w-0 items-center gap-2 text-sm font-bold text-blue-100';
  const icon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  icon.setAttribute('viewBox', '0 0 24 24'); icon.setAttribute('fill', 'none'); icon.setAttribute('stroke', 'currentColor');
  icon.setAttribute('stroke-width', '2'); icon.setAttribute('stroke-linecap', 'round'); icon.setAttribute('stroke-linejoin', 'round');
  icon.setAttribute('aria-hidden', 'true'); icon.classList.add('h-[18px]', 'w-[18px]', 'shrink-0', 'text-slate-100');
  icon.innerHTML = '<path d="M3 12a9 9 0 1 0 18 0a9 9 0 1 0 -18 0"/><path d="M9 12l2 2l4 -4"/>';
  const title = document.createElement('span'); title.textContent = 'Доступно обновление nfqws2';
  const actions = document.createElement('span'); actions.className = 'ml-auto flex shrink-0 items-center gap-1';
  const close = document.createElement('button'); close.type = 'button'; close.dataset.actionClick = 'dismissCoreUpdateNotice';
  close.dataset.args = JSON.stringify([presentation.version]); close.className = 'inline-flex h-7 w-7 items-center justify-center rounded-md text-[18px] leading-none text-slate-400 transition-colors hover:bg-slate-700/70 hover:text-white';
  close.setAttribute('aria-label', 'Скрыть уведомление об обновлении nfqws2'); close.textContent = '×'; actions.append(close);
  const body = document.createElement('button'); body.type = 'button'; body.dataset.actionClick = 'openUpdateModal'; body.dataset.args = JSON.stringify(['core']);
  body.className = 'mt-1 block w-full text-left text-sm leading-relaxed text-slate-300 hover:text-white';
  body.textContent = `Доступна новая версия nfqws2 ${presentation.version}`;
  heading.append(icon, title, actions); item.append(heading, body); host.append(item);
}
let coreUpdateStatusPolling = false;
let lastCoreUpdateCheckedAt = 0;
async function refreshCoreUpdateStatus(manualCheck = false) {
  if (coreUpdateStatusPolling || (!manualCheck && !shouldCheckCoreUpdate(Date.now(), lastCoreUpdateCheckedAt))) return;
  coreUpdateStatusPolling = true;
  lastCoreUpdateCheckedAt = Date.now();
  try { renderCoreUpdateNotice(await nfqwsUpdates.coreStatus(), manualCheck); }
  catch { /* Background update checks must not interrupt panel use. */ }
  finally { coreUpdateStatusPolling = false; }
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
  all<HTMLButtonElement>('button[data-action-click]:not([data-independent-action="true"])').forEach((button) => {
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
    const editor = field.closest<HTMLElement>('.config-argument-editor');
    if (editor) editor.inert = value;
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
  document.querySelectorAll<HTMLElement>('[id^="tabContent-"]').forEach((section) => section.classList.toggle('hidden', section.id !== `tabContent-${route}`));
  all<HTMLButtonElement>('.side-nav-btn').forEach((button) => {
    const active = button.id === `navBtn-${route}`;
    button.classList.toggle('active', active);
    button.classList.toggle('text-slate-400', !active);
  });
  syncNavigationLayout(route);
  if (route === 'lists') void loadList(activeListId);
  if (route === 'profiles') void loadProfiles();
  if (route === 'community') {
    const autoDetect = !communityAutoDetectionAttempted;
    if (autoDetect) communityAutoDetectionAttempted = true;
    void loadCommunityStrategies(false, false, autoDetect);
  }
  if (route === 'snapshots') void loadBackups();
  if (route === 'dpiLab') { switchDpiLabTab('visualizer'); void loadDpiStrategyProfiles(); }
  if (route === 'changes') void loadChangeHistory();
  if (route === 'logs') { void loadLogs(); startLogPolling(); } else stopLogPolling();
  if (route === 'about') void loadAboutStatus();
}

function dpiLabElement<K extends keyof HTMLElementTagNameMap>(tag: K, text = '', className = ''): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  if (className) element.className = className;
  if (text) element.textContent = text;
  return element;
}

function dpiFileIdentity(file: File): string {
  return `${file.name}\0${file.size}\0${file.lastModified}`;
}

function renderDpiExperiments(items: DpiLabSavedExperiment[]) {
  const host = $('#dpiLabExperiments');
  if (!host) return;
  host.replaceChildren();
  if (!items.length) {
    host.append(dpiLabElement('p', 'Сохранённых экспериментов пока нет.', 'text-sm text-slate-400'));
    return;
  }
  for (const item of items) {
    const row = dpiLabElement('div', '', 'dpi-lab-experiment-row');
    const details = dpiLabElement('div', '', 'min-w-0');
    details.append(dpiLabElement('strong', item.name, 'block truncate text-sm text-slate-100'));
    const date = new Date(item.savedAt * 1000);
    details.append(dpiLabElement('span', `${date.toLocaleString()} · ${item.packetCount} пак. · ${item.flowCount} TCP направл. · ${Math.ceil(item.sizeBytes / 1024)} КиБ`, 'block mt-1 text-xs text-slate-400'));
    const open = dpiLabElement('button', 'Открыть', 'about-check-button shrink-0');
    open.type = 'button';
    open.addEventListener('click', () => void loadDpiExperiment(item.id));
    row.append(details, open);
    host.append(row);
  }
}

async function loadDpiExperiments() {
  const host = $('#dpiLabExperiments');
  try {
    const response = await nfqwsDpiLab.list();
    dpiLabExperiments = response.items;
    renderDpiExperiments(dpiLabExperiments);
  } catch (error) {
    if (host) {
      host.replaceChildren();
      host.append(dpiLabElement('p', error instanceof Error ? error.message : 'Не удалось загрузить сохранённые эксперименты.', 'text-sm text-rose-300'));
    }
  }
}

function switchDpiLabTab(tab: 'visualizer' | 'live' | 'pcap') {
  for (const name of ['visualizer', 'live', 'pcap'] as const) {
    const pane = $(`#dpiLabPane-${name}`);
    pane?.classList.toggle('hidden', name !== tab);
    const button = [...document.querySelectorAll<HTMLButtonElement>('.dpi-lab-tab')].find((item) => item.dataset.args?.includes(name));
    button?.classList.toggle('active', name === tab);
    button?.setAttribute('aria-current', name === tab ? 'page' : 'false');
  }
  renderDpiExpectedModel();
  if (tab === 'pcap') void loadDpiExperiments();
}

function renderDpiExpectedModel() {
  const host = $('#dpiLabLiveModel');
  const note = $('#dpiLabPcapStrategyNote');
  if (host) {
    host.replaceChildren();
    if (!dpiStrategyAnalysis) host.textContent = 'Ожидаемая стратегия появится здесь после разбора в первой вкладке.';
    else {
      host.append(dpiLabElement('strong', `Ожидаемая схема · ${dpiStrategyAnalysis.stepCount} шага(ов)`, 'block text-slate-200'));
      if (dpiStrategyAnalysis.steps.length) {
        const list = dpiLabElement('ol', '', 'mt-2 grid gap-1');
        for (const step of dpiStrategyAnalysis.steps) list.append(dpiLabElement('li', `${step.index}. ${step.function}(${step.arguments.map((arg) => `${arg.name}${arg.value === null ? '' : `=${arg.value}`}`).join(', ')}) → ${step.modeling}`, 'break-words'));
        host.append(list);
      }
      host.append(dpiLabElement('p', 'Source metadata: подтверждённое наличие в Lua-каталоге; сетевой эффект пока не моделируется.', 'mt-2 text-xs'));
    }
  }
  if (note) note.textContent = dpiStrategyAnalysis?.steps.length
    ? `Визуализировано шагов: ${dpiStrategyAnalysis.stepCount}. Сопоставление по наблюдаемым байтам/пакетам пока невозможно: моделирование этих Lua-действий не реализовано.`
    : 'Сравнение PCAP со стратегией станет достоверным, когда для её Lua-функций будет реализована проверенная модель. Неизвестные действия не синтезируются.';
  renderDpiCorrelation();
}

function renderDpiCorrelation() {
  const host = $('#dpiLabCorrelation');
  if (!host) return;
  host.replaceChildren();
  const data = dpiStrategyCorrelation(dpiStrategyAnalysis, dpiLabAnalysis);
  for (const [title, detail] of [
    ['Ожидается по выбранной стратегии', data.expected.length ? data.expected.join(' · ') : 'Стратегия не выбрана или не содержит Lua-шагов.'],
    ['Модель', data.model], ['Наблюдается', data.observed], ['Неизвестно', data.confirmed],
  ]) {
    const row = dpiLabElement('div', '', 'rounded-lg border border-slate-700 p-3');
    row.append(dpiLabElement('strong', title, 'text-slate-100'), dpiLabElement('p', detail, 'mt-1 break-words text-slate-300')); host.append(row);
  }
}

function renderDpiJourney(packet: DpiLabPacket | undefined) {
  const host = $('#dpiLabJourney');
  if (!host) return;
  host.replaceChildren();
  const nodes = [packet ? `Отправитель: ${packet.source}:${packet.sourcePort ?? '—'}` : 'Отправитель неизвестен', 'Сетевой путь: неизвестен', 'nfqws2: участие не подтверждено', packet ? `Получатель: ${packet.destination}:${packet.destinationPort ?? '—'}` : 'Получатель неизвестен'];
  nodes.forEach((label, index) => {
    if (index) host.append(dpiLabElement('span', '→', 'text-slate-500'));
    host.append(dpiLabElement('span', label, 'rounded-lg border border-slate-700 px-3 py-2 break-all text-slate-200'));
  });
}

async function loadDpiStrategyProfiles() {
  const select = $('#dpiLabProfileSelect');
  if (!(select instanceof HTMLSelectElement)) return;
  try {
    dpiLabProfiles = await nfqwsProfiles.list();
    select.replaceChildren(new Option('Выберите текущий профиль…', ''));
    for (const profile of dpiLabProfiles.profiles) select.add(new Option(`${profile.index}. ${profile.name} · ${profile.source}`, profile.id));
    if (!dpiLabProfiles.profiles.length) select.replaceChildren(new Option('Профили не найдены', ''));
  } catch (error) {
    select.replaceChildren(new Option('Не удалось загрузить профили', ''));
    setText('#dpiLabStrategyStatus', error instanceof Error ? error.message : 'Не удалось загрузить профили из конфигурации nfqws2.');
  }
}

function selectDpiStrategyProfile() {
  const select = $('#dpiLabProfileSelect');
  const source = $('#dpiLabStrategySource');
  if (!(select instanceof HTMLSelectElement) || !(source instanceof HTMLTextAreaElement) || !dpiLabProfiles) return;
  const profile = dpiLabProfiles.profiles.find((item) => item.id === select.value);
  if (!profile) return;
  source.value = [...profile.filters, ...profile.protocols, ...profile.payloads, ...profile.steps].join('\n');
  invalidateDpiStrategyAnalysis();
  setText('#dpiLabStrategyStatus', `Загружен профиль «${profile.name}» из ${profile.source}. Нажмите «Визуализировать».`);
}

function invalidateDpiStrategyAnalysis() {
  dpiStrategyAnalysis = null;
  dpiAnalyzedStrategySource = null;
  $('#dpiLabStrategyResults')?.classList.add('hidden');
  renderDpiExpectedModel();
}

function renderDpiStrategyAnalysis(result: DpiLabStrategyAnalysis) {
  const region = $('#dpiLabStrategyResults');
  const facts = $('#dpiLabStrategyFacts');
  const steps = $('#dpiLabStrategySteps');
  if (!region || !facts || !steps) return;
  region.classList.remove('hidden');
  facts.replaceChildren();
  facts.append(dpiLabElement('span', `${result.stepCount} Lua-шаг(ов)`, 'dpi-lab-fact'));
  for (const profile of result.profiles) facts.append(dpiLabElement('span', `Профиль ${profile.index}${profile.name ? ` · ${profile.name}` : ''}: ${[...profile.filters, ...profile.controls].join(' · ') || 'фильтры не заданы'}`, 'dpi-lab-fact font-mono'));
  steps.replaceChildren();
  if (result.warnings.length) {
    const warningBox = dpiLabElement('div', '', 'grid gap-1 rounded-lg border border-amber-500/30 bg-amber-500/5 p-3 text-sm text-amber-200');
    result.warnings.forEach((warning) => warningBox.append(dpiLabElement('p', warning)));
    steps.append(warningBox);
  }
  for (const item of result.steps) {
    const card = dpiLabElement('article', '', 'dpi-lab-strategy-step');
    const heading = dpiLabElement('div', '', 'flex flex-wrap items-center justify-between gap-2');
    heading.append(dpiLabElement('h3', `${item.index}. ${item.function} · профиль ${item.profileIndex}`, 'font-semibold text-slate-100'));
    heading.append(dpiLabElement('span', item.installed ? `${item.evidence} · ${item.file}` : 'UNKNOWN · нет в каталоге установки', `evidence-badge ${item.installed ? 'evidence-derived' : 'evidence-unknown'}`));
    card.append(heading);
    card.append(dpiLabElement('code', `--lua-desync=${item.raw}`, 'mt-2 block overflow-x-auto rounded bg-slate-950/70 p-2 font-mono text-xs text-cyan-100'));
    if (item.arguments.length) {
      const argList = dpiLabElement('div', '', 'mt-2 flex flex-wrap gap-2');
      for (const argument of item.arguments) argList.append(dpiLabElement('span', `${argument.name}${argument.value === null ? '' : ` = ${argument.value}`}`, 'dpi-lab-fact font-mono'));
      card.append(argList);
    } else card.append(dpiLabElement('p', 'Аргументы не заданы.', 'mt-2 text-xs text-slate-400'));
    if (item.requiredArguments.length) card.append(dpiLabElement('p', `Функция требует: ${item.requiredArguments.join(', ')}. Сверьте с указанными аргументами.`, 'mt-2 text-xs text-amber-200'));
    if (item.unverifiedArguments.length) card.append(dpiLabElement('p', `Не найдены в документации установленной функции: ${item.unverifiedArguments.join(', ')}. Требует ручной проверки; поддержка не подтверждена.`, 'mt-2 text-xs text-amber-200'));
    for (const doc of item.docs) card.append(dpiLabElement('p', doc, 'mt-2 text-sm text-slate-300'));
    if (item.declaredArguments.length) card.append(dpiLabElement('p', `Параметры по установленному Lua-каталогу: ${item.declaredArguments.join(' · ')}`, 'mt-2 break-words text-xs text-slate-400'));
    card.append(dpiLabElement('p', `Packet Timeline: шаг ${item.index} следует за предыдущим. Синтетические пакеты, SEQ/ACK, TTL, checksum, replay/drop и точный split: ${item.modeling}.`, 'mt-2 border-t border-slate-800 pt-2 text-xs text-slate-400'));
    steps.append(card);
  }
  if (!result.luaExecuted) steps.prepend(dpiLabElement('p', 'Исходник прочитан как данные. Выполнение Lua отключено.', 'rounded-lg border border-emerald-500/20 bg-emerald-500/5 px-3 py-2 text-xs text-emerald-200'));
}

async function visualizeDpiStrategy() {
  const source = $('#dpiLabStrategySource');
  const button = $('[data-action-click="visualizeDpiStrategy"]');
  if (!(source instanceof HTMLTextAreaElement)) return;
  const text = source.value;
  if (!text.trim()) { setText('#dpiLabStrategyStatus', 'Выберите профиль, вставьте стратегию или загрузите текстовый файл.'); return; }
  if (new TextEncoder().encode(text).length > 256 * 1024) { setText('#dpiLabStrategyStatus', 'Текст больше лимита 256 КиБ.'); return; }
  invalidateDpiStrategyAnalysis();
  if (button instanceof HTMLButtonElement) button.disabled = true;
  setText('#dpiLabStrategyStatus', 'Разбираю стратегию и сверяю функции с Lua-файлами установленной версии…');
  try {
    const analysis = await nfqwsDpiLab.analyzeStrategy(text);
    if (source.value !== text) { setText('#dpiLabStrategyStatus', 'Текст изменился во время анализа. Нажмите «Визуализировать» повторно.'); return; }
    dpiStrategyAnalysis = analysis;
    dpiAnalyzedStrategySource = text;
    renderDpiStrategyAnalysis(dpiStrategyAnalysis);
    renderDpiExpectedModel();
    setText('#dpiLabStrategyStatus', `Разбор завершён: ${dpiStrategyAnalysis.stepCount} Lua-шаг(ов). Никакие сетевые пакеты не отправлялись.`);
  } catch (error) {
    setText('#dpiLabStrategyStatus', error instanceof Error ? error.message : 'Не удалось разобрать стратегию.');
  } finally { if (button instanceof HTMLButtonElement) button.disabled = false; }
}

async function runDpiLiveStrategyTest() {
  const host = $('#dpiLabLiveHost');
  const protocol = $('#dpiLabLiveProtocol');
  const output = $('#dpiLabLiveResults');
  if (!(host instanceof HTMLInputElement) || !(protocol instanceof HTMLSelectElement) || !output) return;
  if (!dpiStrategyAnalysis || $<HTMLTextAreaElement>('#dpiLabStrategySource')?.value !== dpiAnalyzedStrategySource) { setText('#dpiLabLiveStatus', 'Сначала выполните «Визуализировать» для текущего текста стратегии.'); return; }
  if (dpiStrategyAnalysis.profiles.length > 1) { setText('#dpiLabLiveStatus', 'В тексте несколько профилей. Выберите один текущий профиль в визуализаторе: объединять разные --new в одну тестовую цепочку нельзя.'); return; }
  const steps = dpiStrategyAnalysis.steps.map((step) => `--lua-desync=${step.raw}`);
  if (!steps.length) { setText('#dpiLabLiveStatus', 'В тексте нет шагов --lua-desync для сетевой проверки.'); return; }
  if (steps.length > 8) { setText('#dpiLabLiveStatus', 'Изолированная проверка ограничена 8 Lua-шагами за один запуск.'); return; }
  if (dpiStrategyAnalysis.otherOptions.some((option) => option.startsWith('--in-range=') || option.startsWith('--out-range='))) {
    setText('#dpiLabLiveStatus', 'Этот тест не поддерживает --in-range/--out-range. Цепочка с такими ограничениями не будет запущена с изменённой семантикой.'); return;
  }
  const button = $('[data-action-click="runDpiLiveStrategyTest"]');
  if (button instanceof HTMLButtonElement) button.disabled = true;
  output.replaceChildren();
  setText('#dpiLabLiveStatus', 'Запускаю прямую базовую проверку, затем отдельный тестовый процесс nfqws2…');
  try {
    const operation = await nfqwsDiagnostics.strategyStart(host.value.trim(), protocol.value as 'http' | 'https' | 'quic', 'custom', 2, [steps], true, false, false, false);
    let current = operation;
    for (let attempt = 0; attempt < 90 && ['queued', 'running', 'stopping'].includes(current.state); attempt += 1) {
      setText('#dpiLabLiveStatus', `${current.message} · ${current.done}/${current.total}`);
      await new Promise((resolve) => window.setTimeout(resolve, 1200));
      current = await nfqwsDiagnostics.strategyStatus(operation.id);
    }
    output.replaceChildren();
    output.append(dpiLabElement('p', 'Наблюдение: результат изолированного сетевого запроса NFQWS2. Отдельный захват пакетов и журнал фактических Lua-вызовов этот API не предоставляет.', 'rounded-lg border border-amber-500/30 bg-amber-500/5 p-3 text-sm text-amber-100'));
    output.append(dpiLabElement('p', 'Сначала операция выполняет прямой базовый запрос, затем запускает выбранные шаги в отдельном тестовом процессе.', 'text-sm text-slate-300'));
    const results = current.results ?? [];
    if (!results.length) output.append(dpiLabElement('p', `${current.state}: ${current.message}`, 'text-sm text-slate-200'));
    for (const result of results) {
      const row = dpiLabElement('article', '', 'dpi-lab-strategy-step');
      row.append(dpiLabElement('h3', result.candidate.name, 'font-semibold text-slate-100'));
      row.append(dpiLabElement('p', `${result.successes}/${result.attempts} успешных сетевых ответов · подтверждение: ${result.confirmed ? 'да по критерию проверки' : 'нет'} · ${result.elapsedMs} мс`, 'mt-1 text-sm text-slate-300'));
      row.append(dpiLabElement('p', result.detail, 'mt-1 text-xs text-slate-400'));
      output.append(row);
    }
    output.append(dpiLabElement('p', 'Неизвестно: какие именно пакеты увидел DPI и какие Lua-ветви реально отправили пакет. HTTP-код ответа не доказывает эффективность обхода.', 'text-sm text-slate-400'));
    setText('#dpiLabLiveStatus', ['queued', 'running', 'stopping'].includes(current.state)
      ? `Операция ещё выполняется (${current.state}). Откройте проверку позже, чтобы получить обновлённый результат.`
      : `Изолированная операция завершена: ${current.state}.`);
  } catch (error) {
    setText('#dpiLabLiveStatus', error instanceof Error ? error.message : 'Не удалось выполнить изолированную проверку.');
  } finally { if (button instanceof HTMLButtonElement) button.disabled = false; }
}

function renderDpiPacketBytes(packet: DpiLabPacket) {
  const caption = $('#dpiLabPacketCaption');
  const host = $('#dpiLabPacketBytes');
  if (!host || !caption) return;
  const info = $('#dpiLabPacketInfo');
  if (info) {
    const event = dpiLabAnalysis?.summary?.events.find((item) => item.packetNumbers[item.packetNumbers.length - 1] === packet.number);
    info.replaceChildren(dpiLabElement('h3', event?.title ?? 'Пакет из захвата', 'font-semibold text-white'), dpiLabElement('p', event?.detail ?? 'Пакет присутствует в файле; его назначение не определено.', 'mt-1'), dpiLabElement('p', `Откуда: ${packet.source}:${packet.sourcePort ?? '—'} → куда: ${packet.destination}:${packet.destinationPort ?? '—'} · данные: ${packet.payloadLen} Б · пакет №${packet.number}`, 'mt-2 break-all'));
  }
  caption.textContent = `Пакет ${packet.number} · ${new Date(packet.timestampMicros / 1000).toISOString()} · ${packet.source}:${packet.sourcePort ?? '—'} → ${packet.destination}:${packet.destinationPort ?? '—'} · ${packet.protocol}${packet.tcpSequence === null ? '' : ` · SEQ ${packet.tcpSequence} · ACK ${packet.tcpAcknowledgement} · flags ${formatDpiTcpFlags(packet.tcpFlags)}`}${packet.ttl === null ? '' : ` · TTL ${packet.ttl}`} · payload ${packet.payloadLen} Б · смещение ${packet.payloadOffset ?? '—'}`;
  host.replaceChildren();
  const dump = formatDpiByteDump(packet.frameHex, 4096);
  for (const row of dump.rows) {
    const line = dpiLabElement('div', '', 'dpi-lab-byte-row');
    line.append(dpiLabElement('span', row.offset, 'dpi-lab-byte-offset'), dpiLabElement('code', row.hex, 'dpi-lab-byte-hex'), dpiLabElement('code', row.ascii, 'dpi-lab-byte-ascii'));
    host.append(line);
  }
  if (dump.error) host.append(dpiLabElement('p', dump.error, 'text-sm text-rose-300'));
  if (dump.hiddenBytes) host.append(dpiLabElement('p', `Скрыто байтов после лимита просмотра: ${dump.hiddenBytes}. Кадр в захвате не изменён.`, 'mt-2 text-xs text-slate-400'));
}

function renderDpiPacketTimeline() {
  const host = $('#dpiLabPacketList');
  if (!host || !dpiLabAnalysis) return;
  const packets = dpiLabAnalysis.packets;
  host.replaceChildren();
  host.className = 'dpi-lab-packet-list';
  host.setAttribute('role', 'list');
  const spacer = dpiLabElement('div', '', 'dpi-lab-virtual-spacer');
  const window = dpiLabElement('div', '', 'dpi-lab-virtual-window');
  spacer.style.height = `${packets.length * 52}px`;
  spacer.append(window);
  host.append(spacer);
  const renderWindow = () => {
    const start = Math.max(0, Math.floor(host.scrollTop / 52) - 4);
    const end = Math.min(packets.length, start + 28);
    window.style.transform = `translateY(${start * 52}px)`;
    window.replaceChildren();
    for (const packet of packets.slice(start, end)) {
      const button = dpiLabElement('button', '', `dpi-lab-packet-row${packet.number === selectedDpiPacketNumber ? ' selected' : ''}`);
      button.type = 'button';
      button.setAttribute('role', 'listitem');
      button.setAttribute('aria-pressed', String(packet.number === selectedDpiPacketNumber));
      const time = new Date(packet.timestampMicros / 1000).toISOString().slice(11, 23);
      const title = `${packet.number}. ${time} ${packet.protocol} ${packet.source}:${packet.sourcePort ?? '—'} → ${packet.destination}:${packet.destinationPort ?? '—'}`;
      button.append(dpiLabElement('strong', title, 'block truncate text-xs text-slate-100'));
      button.append(dpiLabElement('span', `SEQ ${packet.tcpSequence ?? '—'} · ${formatDpiTcpFlags(packet.tcpFlags)} · ${packet.payloadLen} Б · ${dpiEvidencePresentation(packet.evidence).label}`, 'block truncate text-[11px] text-slate-400'));
      button.addEventListener('click', () => {
        selectedDpiPacketNumber = packet.number;
        renderDpiPacketTimeline();
        renderDpiPacketBytes(packet);
        renderDpiJourney(packet);
      });
      window.append(button);
    }
  };
  host.onscroll = renderWindow;
  renderWindow();
  if (selectedDpiPacketNumber !== null) {
    const selected = packets.find((packet) => packet.number === selectedDpiPacketNumber);
    if (selected) renderDpiPacketBytes(selected);
  }
}

function renderDpiFlowDetails(flow: DpiLabFlow) {
  const host = $('#dpiLabFlowDetails');
  if (!host) return;
  host.replaceChildren();
  const facts = dpiLabElement('div', '', 'grid gap-2 text-sm');
  facts.append(dpiLabElement('p', `${flow.source}:${flow.sourcePort} → ${flow.destination}:${flow.destinationPort}`, 'break-all font-mono text-slate-200'));
  facts.append(dpiLabElement('p', `Политика: ${flow.reassemblyPolicy} · старт SEQ ${flow.sequenceStart} · first-seen по порядку захвата · пакеты ${flow.packetNumbers.join(', ') || '—'}`, 'break-words text-xs text-slate-400'));
  facts.append(dpiLabElement('p', `Пропуски: ${flow.gapBytes} Б · повторные позиции: ${flow.overlapBytes} Б · различающиеся байты: ${flow.conflictingOverlapBytes} Б${flow.streamTruncated ? ' · поток ограничен 32 КиБ' : ''}`, 'text-sm text-slate-300'));
  facts.append(dpiLabElement('span', 'Реконструкция · SIMULATED', 'evidence-badge evidence-simulated w-fit'));
  const streamTitle = dpiLabElement('h3', 'Байты восстановленного потока · ?? означает пропуск · источник — первый пакет захвата', 'mt-3 text-xs font-semibold text-slate-200');
  const stream = dpiLabElement('div', '', 'dpi-lab-stream-bytes');
  const rows = formatDpiFlowByteRows(flow.dataHex, flow.byteSources, flow.sequenceStart);
  if (!rows.length) stream.append(dpiLabElement('p', flow.dataHex ? 'Не удалось проверить карту источников байтов.' : 'В потоке нет payload.'));
  for (const row of rows) {
    const line = dpiLabElement('div', '', 'dpi-lab-byte-row dpi-lab-stream-row');
    line.append(dpiLabElement('span', row.offset, 'dpi-lab-byte-offset'), dpiLabElement('span', row.sequence, 'dpi-lab-byte-sequence'), dpiLabElement('code', row.hex, 'dpi-lab-byte-hex'), dpiLabElement('span', row.sources, 'dpi-lab-byte-source'));
    stream.append(line);
  }
  if (flow.dataHex.split(' ').length > 4096) stream.append(dpiLabElement('p', 'Показаны первые 4096 байтов потока.', 'mt-2 text-xs text-slate-400'));
  const technical = dpiLabElement('details', '', 'dpi-lab-expert-details');
  technical.append(dpiLabElement('summary', 'Байты потока и источники', 'cursor-pointer text-sm text-slate-300'), streamTitle, stream);
  host.append(facts, technical);
  if (flow.tlsClientHello) {
    const hello = flow.tlsClientHello;
    host.append(dpiLabElement('h3', 'TLS ClientHello · PROTOCOL DERIVED', 'mt-4 text-sm font-semibold text-cyan-200'));
    host.append(dpiLabElement('p', `SNI: ${hello.serverName ?? 'не найдено'}`, 'mt-1 break-all font-mono text-sm text-slate-100'));
    const table = dpiLabElement('div', '', 'dpi-lab-tls-fields');
    for (const field of hello.fields) {
      const row = dpiLabElement('div', '', 'dpi-lab-tls-field');
      row.append(dpiLabElement('strong', field.name, 'text-slate-200'));
      row.append(dpiLabElement('span', `байт ${field.offset} · ${field.length} Б`, 'font-mono text-xs text-cyan-200'));
      row.append(dpiLabElement('span', field.value, 'min-w-0 break-words text-slate-300'));
      row.append(dpiLabElement('code', field.hex, 'min-w-0 break-all text-[11px] text-slate-500'));
      table.append(row);
    }
    const tlsDetails = dpiLabElement('details', '', 'dpi-lab-expert-details');
    tlsDetails.append(dpiLabElement('summary', 'Поля и байты ClientHello', 'cursor-pointer text-sm text-slate-300'), table);
    host.append(tlsDetails);
  } else if (flow.tlsParseError) {
    host.append(dpiLabElement('p', `TLS ClientHello не разобран: ${flow.tlsParseError}`, 'mt-3 text-sm text-amber-200'));
  }
}

let dpiDetailLevel: 'simple' | 'advanced' | 'expert' = 'simple';
function setDpiDetailLevel(level: 'simple' | 'advanced' | 'expert') {
  dpiDetailLevel = level;
  const analysisRegion = $('#dpiLabAnalysis');
  if (analysisRegion) analysisRegion.dataset.detailLevel = level;
  $('#dpiLabAdvancedPanel')?.classList.toggle('hidden', level === 'simple');
  $('#dpiLabExpertPanel')?.classList.toggle('hidden', level !== 'expert');
  $('#dpiLabPacketInfo')?.classList.toggle('hidden', level === 'simple');
  document.querySelectorAll<HTMLButtonElement>('#dpiLabLevels button').forEach((button) => button.setAttribute('aria-pressed', String(button.dataset.level === level)));
  if (level === 'expert' && dpiLabAnalysis && selectedDpiPacketNumber !== null) {
    const index = dpiLabAnalysis.packets.findIndex((packet) => packet.number === selectedDpiPacketNumber);
    const list = $('#dpiLabPacketList');
    if (list && index >= 0) { list.scrollTop = index * 52; renderDpiPacketTimeline(); }
  }
}

function renderDpiHumanSummary(analysis: DpiLabAnalysis) {
  const levels = $('#dpiLabLevels');
  if (levels) {
    levels.replaceChildren();
    for (const [level, label] of [['simple', 'Простой'], ['advanced', 'Продвинутый'], ['expert', 'Экспертный']] as const) {
      const button = dpiLabElement('button', label, 'about-check-button dpi-lab-level-option');
      button.type = 'button'; button.dataset.level = level;
      button.addEventListener('click', () => setDpiDetailLevel(level)); levels.append(button);
    }
  }
  const findings = $('#dpiLabFindings');
  if (findings) {
    findings.replaceChildren();
    for (const finding of analysis.summary.findings) {
      const card = dpiLabElement('div', '', 'rounded-lg border border-slate-700 p-3');
      const heading = dpiLabElement('div', '', 'flex flex-wrap items-center justify-between gap-2');
      const status = dpiFindingStatus(finding.state);
      heading.append(dpiLabElement('h3', finding.title, 'font-semibold text-slate-100'), dpiLabElement('span', status.label, `dpi-lab-finding-status dpi-lab-status-${status.tone}`));
      card.append(heading, dpiLabElement('p', finding.detail, 'mt-1 text-sm text-slate-300'));
      if (finding.packetNumbers.length || finding.flowIndices.length) {
        const evidence = dpiLabElement('details', '', 'mt-2');
        evidence.append(dpiLabElement('summary', `Показать подтверждения · пакеты: ${finding.packetNumbers.length}${finding.packetNumbers.length === 32 ? ' (первые, лимит 32)' : ''}, потоки: ${finding.flowIndices.length}`, 'cursor-pointer text-sm text-cyan-200'));
        const links = dpiLabElement('div', '', 'flex flex-wrap gap-1');
        for (const number of finding.packetNumbers) {
          const button = dpiLabElement('button', `Пакет №${number}`, 'about-check-button mt-2');
          button.type = 'button'; button.addEventListener('click', () => showDpiPacket(number)); links.append(button);
        }
        for (const index of finding.flowIndices) {
          const button = dpiLabElement('button', `Поток №${index}`, 'about-check-button mt-2');
          button.type = 'button'; button.addEventListener('click', () => showDpiFlow(index)); links.append(button);
        }
        evidence.append(links); card.append(evidence);
      }
      findings.append(card);
    }
  }
  const limitations = $('#dpiLabLimitations');
  if (limitations) { limitations.replaceChildren(); for (const line of analysis.summary.limitations) limitations.append(dpiLabElement('p', line)); }
  const nextSteps = $('#dpiLabNextSteps');
  if (nextSteps) { nextSteps.replaceChildren(); for (const tip of dpiNextSteps(analysis)) nextSteps.append(dpiLabElement('li', tip)); }
  const events = $('#dpiLabEvents');
  if (events) {
    events.replaceChildren();
    if (!analysis.summary.events.length) events.append(dpiLabElement('p', 'Распознанных событий нет. Захват может быть неполным.', 'text-sm text-slate-400'));
    let count = 0;
    const more = dpiLabElement('button', 'Показать ещё события', 'about-check-button w-fit'); more.type = 'button';
    const appendEvents = () => {
      more.remove();
      for (const event of analysis.summary.events.slice(count, count + 12)) {
        const row = dpiLabElement('div', '', 'rounded-lg border border-slate-700 p-3');
        row.append(dpiLabElement('strong', event.title, 'text-slate-100'), dpiLabElement('p', event.detail, 'mt-1 text-sm text-slate-300'));
        for (const number of event.packetNumbers) { const button = dpiLabElement('button', `Пакет №${number}`, 'about-check-button mt-2 mr-2'); button.type = 'button'; button.addEventListener('click', () => showDpiPacket(number)); row.append(button); }
        if (event.flowIndex !== null) { const button = dpiLabElement('button', `Поток №${event.flowIndex}`, 'about-check-button mt-2 mr-2'); button.type = 'button'; button.addEventListener('click', () => showDpiFlow(event.flowIndex!)); row.append(button); }
        events.append(row);
      }
      count += 12;
      if (count < analysis.summary.events.length) events.append(more);
      else if (analysis.summary.eventsTruncated) events.append(dpiLabElement('p', 'Анализатор ограничил временную шкалу первыми событиями. Полный список кадров доступен в экспертном режиме.', 'rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-400'));
    };
    more.addEventListener('click', appendEvents); appendEvents();
  }
  setDpiDetailLevel(dpiDetailLevel);
}

function showDpiPacket(number: number) {
  const packet = dpiLabAnalysis?.packets.find((item) => item.number === number);
  if (!packet) return;
  selectedDpiPacketNumber = number; setDpiDetailLevel('advanced'); renderDpiPacketBytes(packet);
  renderDpiJourney(packet);
  $('#dpiLabPacketInfo')?.scrollIntoView({ block: 'nearest' });
}

function showDpiFlow(index: number) {
  if (!dpiLabAnalysis || index < 1 || index > dpiLabAnalysis.flows.length) return;
  setDpiDetailLevel('advanced');
  const button = document.querySelectorAll<HTMLButtonElement>('#dpiLabFlowList .dpi-lab-flow-row')[index - 1];
  button?.click();
  $('#dpiLabFlowDetails')?.scrollIntoView({ block: 'nearest' });
}

function renderDpiLabAnalysis(analysis: DpiLabAnalysis) {
  const region = $('#dpiLabAnalysis');
  if (!region) return;
  region.classList.remove('hidden');
  dpiDetailLevel = 'simple';
  renderDpiHumanSummary(analysis);
  renderDpiCorrelation();
  renderDpiJourney(analysis.packets.find((packet) => packet.sourcePort !== null && packet.destinationPort !== null));
  setText('#dpiLabCaptureName', analysis.capture.name);
  setText('#dpiLabCaptureSummary', `${analysis.capture.packetCount} пак. · ${analysis.flows.length} TCP направл. · ${Math.ceil(analysis.capture.sizeBytes / 1024)} КиБ · ${analysis.capture.timestampResolution} · link type ${analysis.capture.linkType}`);
  const warnings = $('#dpiLabWarnings');
  if (warnings) {
    warnings.replaceChildren();
    for (const warning of analysis.warnings) warnings.append(dpiLabElement('p', warning, 'rounded-md border border-amber-500/20 bg-amber-500/5 px-3 py-2'));
    if (!analysis.warnings.length) warnings.append(dpiLabElement('p', 'Неподдержанных или пропущенных кадров нет.', 'text-xs text-slate-400'));
  }
  selectedDpiPacketNumber = analysis.packets[0]?.number ?? null;
  selectedDpiFlowIndex = analysis.flows.length ? 0 : null;
  renderDpiPacketTimeline();
  if (!analysis.packets.length) {
    setText('#dpiLabPacketCaption', 'Кадры в PCAP отсутствуют.');
    $('#dpiLabPacketBytes')?.replaceChildren();
  }
  const flows = $('#dpiLabFlowList');
  if (flows) {
    flows.replaceChildren();
    if (!analysis.flows.length) flows.append(dpiLabElement('p', 'TCP payload-потоки не найдены.', 'text-sm text-slate-400'));
    analysis.flows.forEach((flow, index) => {
      const button = dpiLabElement('button', '', `dpi-lab-flow-row${index === selectedDpiFlowIndex ? ' selected' : ''}`);
      button.type = 'button';
      button.append(dpiLabElement('strong', `${flow.source}:${flow.sourcePort} → ${flow.destination}:${flow.destinationPort}`, 'block break-all text-xs text-slate-100'));
      button.append(dpiLabElement('span', `${flow.packetNumbers.length} пак. · ${flow.gapBytes} Б пропусков · ${flow.overlapBytes} Б повторов · ${flow.conflictingOverlapBytes} Б различий${flow.tlsClientHello?.serverName ? ` · SNI ${flow.tlsClientHello.serverName}` : ''}`, 'mt-1 block text-[11px] text-slate-400'));
      button.addEventListener('click', () => {
        selectedDpiFlowIndex = index;
        flows.querySelectorAll('.dpi-lab-flow-row').forEach((row, rowIndex) => row.classList.toggle('selected', rowIndex === index));
        renderDpiFlowDetails(flow);
      });
      flows.append(button);
    });
  }
  const selectedFlow = selectedDpiFlowIndex === null ? undefined : analysis.flows[selectedDpiFlowIndex];
  if (selectedFlow) renderDpiFlowDetails(selectedFlow);
}

function syncDpiCaptureControls() {
  const input = $('#dpiLabCaptureFile');
  const button = $('#dpiLabAnalyzeButton');
  if (!(input instanceof HTMLInputElement) || !(button instanceof HTMLButtonElement)) return;
  const file = input.files?.[0];
  const availability = dpiCaptureAvailability(file?.size ?? null, dpiCaptureAnalyzing || dpiCaptureSaving);
  button.disabled = availability.disabled;
  button.title = availability.reason;
  button.textContent = dpiCaptureAnalyzing ? 'Анализирую…' : 'Анализировать PCAP';
  button.setAttribute('aria-busy', String(dpiCaptureAnalyzing));
  input.disabled = dpiCaptureAnalyzing || dpiCaptureSaving;
  const save = $('#dpiLabSaveButton');
  const saveState = dpiExperimentSaveAvailability(!!file, !!file && !!dpiLabAnalysis && dpiFileIdentity(file) === dpiAnalyzedFileIdentity, dpiCaptureAnalyzing, dpiCaptureSaving);
  if (save instanceof HTMLButtonElement) {
    save.disabled = saveState.disabled;
    save.title = saveState.reason;
    save.textContent = dpiCaptureSaving ? 'Сохраняю…' : 'Сохранить эксперимент';
  }
  setText('#dpiLabSaveHint', saveState.reason);
}

async function analyzeDpiCapture() {
  const input = $('#dpiLabCaptureFile');
  const status = $('#dpiLabStatus');
  if (!(input instanceof HTMLInputElement) || !input.files?.[0]) { setText('#dpiLabStatus', 'Сначала выберите файл classic PCAP.'); return; }
  const file = input.files[0];
  const availability = dpiCaptureAvailability(file.size, dpiCaptureAnalyzing);
  if (availability.disabled) { setText('#dpiLabStatus', availability.reason); return; }
  dpiCaptureAnalyzing = true;
  dpiAnalyzedFileIdentity = null;
  syncDpiCaptureControls();
  if (status) status.textContent = 'Проверяю границы PCAP и реконструирую потоки…';
  try {
    dpiLabAnalysis = await nfqwsDpiLab.analyze(file);
    dpiAnalyzedFileIdentity = dpiFileIdentity(file);
    renderDpiLabAnalysis(dpiLabAnalysis);
    setText('#dpiLabStatus', `Анализ завершён: ${dpiLabAnalysis.packets.length} кадров, ${dpiLabAnalysis.flows.length} направлений TCP.`);
  } catch (error) {
    if (status) status.textContent = error instanceof Error ? error.message : 'Не удалось разобрать PCAP.';
  } finally {
    dpiCaptureAnalyzing = false;
    syncDpiCaptureControls();
  }
}

async function saveDpiExperiment() {
  if (dpiCaptureSaving || dpiCaptureAnalyzing) return;
  const input = $('#dpiLabCaptureFile');
  if (!(input instanceof HTMLInputElement) || !input.files?.[0] || !dpiLabAnalysis || dpiFileIdentity(input.files[0]) !== dpiAnalyzedFileIdentity) {
    setText('#dpiLabStatus', 'Сначала проанализируйте выбранный PCAP.');
    return;
  }
  dpiCaptureSaving = true;
  syncDpiCaptureControls();
  try {
    const saved = await nfqwsDpiLab.save(input.files[0]);
    setText('#dpiLabStatus', `Эксперимент «${saved.name}» сохранён.`);
    await loadDpiExperiments();
  } catch (error) {
    setText('#dpiLabStatus', error instanceof Error ? error.message : 'Не удалось сохранить эксперимент.');
  } finally {
    dpiCaptureSaving = false;
    syncDpiCaptureControls();
  }
}

async function loadDpiExperiment(id: string) {
  if (dpiCaptureAnalyzing || dpiCaptureSaving) { setText('#dpiLabStatus', 'Дождитесь завершения текущего анализа или сохранения.'); return; }
  setText('#dpiLabStatus', 'Загружаю сохранённый эксперимент…');
  try {
    dpiLabAnalysis = await nfqwsDpiLab.load(id);
    dpiAnalyzedFileIdentity = null;
    const input = $('#dpiLabCaptureFile');
    if (input instanceof HTMLInputElement) input.value = '';
    syncDpiCaptureControls();
    renderDpiLabAnalysis(dpiLabAnalysis);
    setText('#dpiLabStatus', `Эксперимент «${dpiLabAnalysis.capture.name}» открыт из локального сохранения.`);
  } catch (error) {
    setText('#dpiLabStatus', error instanceof Error ? error.message : 'Не удалось открыть эксперимент.');
  }
}

function renderCommunityStrategies(data: CommunityStrategiesResponse) {
  const provider = $('#communityProviderSummary');
  if (provider) provider.textContent = communityProviderLabel(data.asn, data.provider);
  const asnInput = $('#communityProviderAsn');
  const nameInput = $('#communityProviderName');
  if (asnInput instanceof HTMLInputElement) asnInput.value = data.asn ? String(data.asn) : '';
  if (nameInput instanceof HTMLInputElement) nameInput.value = data.provider;
  const editor = $('#communityProviderEditor');
  const editButton = $('#communityEditProvider');
  const editing = data.asn === null || communityProviderEditing;
  editor?.classList.toggle('hidden', !editing);
  editButton?.classList.toggle('hidden', editing);
  const summary = $('#communityDatabaseSummary');
  if (summary) summary.textContent = `${data.total ?? '—'} стратегий · ${data.providers ?? '—'} провайдеров${data.fetchedAt ? ` · база обновлена ${new Date(data.fetchedAt * 1000).toLocaleString()}` : ''}`;
  const error = $('#communityDatabaseError');
  if (error) { error.textContent = data.error ?? ''; error.classList.toggle('hidden', !data.error); }
  const host = $('#communityStrategiesList');
  if (!host) return;
  host.replaceChildren();
  const groups = visibleCommunityProtocols(data.items);
  for (const group of groups) {
    const items = group.items;
    const section = document.createElement('section'); section.className = 'community-protocol-group';
    const title = document.createElement('h2'); title.className = 'community-protocol-title'; title.textContent = `${group.title} · ${items.length}`; section.append(title);
    for (const item of items) {
      const row = document.createElement('article'); row.className = 'community-strategy-row';
      const details = document.createElement('div'); details.className = 'community-strategy-details';
      const nameLine = document.createElement('div'); nameLine.className = 'community-strategy-name-line';
      const name = document.createElement('strong'); name.className = 'community-strategy-name'; name.textContent = item.name || item.steps.map((step) => step.replace('--lua-desync=', '').split(':')[0]).join(' + '); nameLine.append(name);
      if (item.skipped) { const badge = document.createElement('span'); badge.className = 'community-strategy-badge community-strategy-badge-skipped'; badge.textContent = 'Подбор пропускает'; nameLine.append(badge); }
      if (!item.fresh) { const badge = document.createElement('span'); badge.className = 'community-strategy-badge community-strategy-badge-stale'; badge.textContent = 'Давно не подтверждалась'; nameLine.append(badge); }
      const meta = document.createElement('p'); meta.className = 'community-strategy-meta';
      const allTargets = item.targets.map((target) => [target.host, target.asn ? `AS${target.asn}` : ''].filter(Boolean).join(' · ')).filter(Boolean);
      const targets = allTargets.slice(0, 6);
      meta.textContent = `${targets.length ? targets.join(' · ') : 'сайт не указан'}${allTargets.length > targets.length ? ` · и ещё ${allTargets.length - targets.length}` : ''} · подтверждений ${item.reports} · не сработала ${item.fails}${item.last ? ` · проверена ${item.last}` : ''}`;
      details.append(nameLine, meta);
      const code = document.createElement('code'); code.className = 'community-strategy-code'; code.textContent = item.steps.join('\n'); details.append(code);
      const actions = document.createElement('div'); actions.className = 'community-strategy-actions';
      const test = document.createElement('button'); test.type = 'button'; test.className = 'backup-action'; test.dataset.actionClick = 'testCommunityStrategy'; test.dataset.args = JSON.stringify([item.id]); test.textContent = 'Проверить';
      const copy = document.createElement('button'); copy.type = 'button'; copy.className = 'backup-action'; copy.dataset.actionClick = 'copyCommunityStrategy'; copy.dataset.args = JSON.stringify([item.id]); copy.textContent = 'Копировать';
      actions.append(test, copy); row.append(details, actions); section.append(row);
    }
    host.append(section);
  }
}

async function loadCommunityStrategies(detect = false, force = false, autoDetect = false) {
  if (communityLoading) return;
  communityLoading = true;
  const refresh = $('[data-action-click="refreshCommunityStrategies"]');
  if (refresh instanceof HTMLButtonElement) { refresh.disabled = true; refresh.textContent = 'Обновление…'; }
  setText('#communityDatabaseError', '');
  setText('#communityDatabaseSummary', detect || autoDetect ? 'Определяю провайдера и загружаю базу…' : 'Загружаю базу…');
  try { communityStrategies = await nfqwsCommunityStrategies.list(detect, force, autoDetect); renderCommunityStrategies(communityStrategies); }
  catch (error) { setText('#communityDatabaseError', error instanceof Error ? error.message : 'Не удалось загрузить стратегии сообщества.'); $('#communityDatabaseError')?.classList.remove('hidden'); }
  finally { communityLoading = false; if (refresh instanceof HTMLButtonElement) { refresh.disabled = false; refresh.textContent = 'Обновить'; } }
}

async function saveCommunityProvider() {
  const asn = $('#communityProviderAsn'); const name = $('#communityProviderName');
  if (!(asn instanceof HTMLInputElement) || !(name instanceof HTMLInputElement)) return;
  const parsed = asn.value.trim() ? Number(asn.value.trim()) : null;
  if (parsed !== null && (!Number.isSafeInteger(parsed) || parsed < 1 || parsed > 4_294_967_295)) return toast('Укажите корректный номер ASN.', 'error');
  try { await nfqwsCommunityStrategies.setProvider(parsed, name.value.trim()); communityProviderEditing = false; communityAutoDetectionAttempted = true; await loadCommunityStrategies(false, true); toast('Провайдер сохранён.'); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось сохранить провайдера.', 'error'); }
}

function findCommunityStrategy(id: string) { return communityStrategies?.items.find((item) => item.id === id); }
async function testCommunityStrategy(id: string) {
  const item = findCommunityStrategy(id); if (!item) return toast('Стратегия больше не доступна. Обновите список.', 'error');
  const target = item.targets.find((candidate) => candidate.host)?.host;
  const host = target ?? '';
  const input = $('#scanCustomHost'); const protocol = $('#strategyProtocolSelect'); const mode = $('#strategyModeSelect'); const custom = $('#strategyCustomSteps'); const repeats = $('#strategyRepeatsSelect');
  if (input instanceof HTMLInputElement) input.value = host;
  if (protocol instanceof HTMLSelectElement) protocol.value = item.proto === 'http' ? 'http' : 'https';
  if (mode instanceof HTMLSelectElement) { mode.value = 'custom'; mode.dispatchEvent(new Event('change', { bubbles: true })); }
  if (custom instanceof HTMLTextAreaElement) setStrategyCustomText(item.steps.join(' '));
  if (repeats instanceof HTMLSelectElement) repeats.value = '3';
  strategyTargetMode = 'single';
  setStrategyTargetMode('single');
  showRoute('autotune');
  if (!host) {
    toast('Укажите домен для проверки этой стратегии в поле «Сайт».');
    if (input instanceof HTMLInputElement) input.focus();
    return;
  }
  await startStrategyPick();
}
async function copyCommunityStrategy(id: string) {
  const item = findCommunityStrategy(id); if (!item) return;
  try { await writeTextToClipboard(item.steps.join('\n')); toast('Стратегия скопирована.'); }
  catch { toast('Буфер обмена недоступен.', 'error'); }
}

async function openCommunityShare(operationId: string, candidateIndex: number) {
  try {
    const operation = await nfqwsDiagnostics.strategyStatus(operationId);
    const candidate = operation.results[candidateIndex];
    if (!candidate || (!candidate.confirmed && !candidate.candidate.source.includes('Стратегии сообщества'))) return toast('Публиковать можно только подтверждённую стратегию или результат теста стратегии сообщества.', 'error');
    if (operation.hosts && operation.hosts.length > 1) return toast('Общий результат для нескольких сайтов нельзя отправить как проверку одного сайта.', 'error');
    pendingCommunityShare = { operationId, candidateIndex, host: operation.host, protocol: operation.protocol === 'http' ? 'http' : 'https', candidate, targetAsn: null };
    let provider = communityStrategies;
    if (!provider?.asn) {
      try { provider = await nfqwsCommunityStrategies.list(true, false); communityStrategies = provider; }
      catch { /* Provider fields remain editable in the share form. */ }
    }
    const asn = $('#communityShareAsn'); const name = $('#communityShareProvider'); const host = $('#communityShareHost');
    if (asn instanceof HTMLInputElement) asn.value = provider?.asn ? String(provider.asn) : '';
    if (name instanceof HTMLInputElement) name.value = provider?.provider ?? '';
    if (host instanceof HTMLInputElement) host.value = operation.host;
    const dialog = $('#communityShareModal');
    renderCommunitySharePreview();
    for (const field of ['#communityShareAsn', '#communityShareProvider']) {
      const input = $(field);
      if (input instanceof HTMLInputElement) input.oninput = renderCommunitySharePreview;
    }
    const hostInput = $('#communityShareHost');
    if (hostInput instanceof HTMLInputElement) hostInput.oninput = () => scheduleCommunityShareTargetAsn();
    setText('#communityShareStatus', 'Публичный issue содержит выбранные ниже ASN, провайдера, сайт, протокол и параметры стратегии. IP не передаётся.');
    if (dialog) openAppModal(dialog, '#communityShareAsn');
    scheduleCommunityShareTargetAsn(true);
  } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось загрузить результат проверки.', 'error'); }
}

function scheduleCommunityShareTargetAsn(immediate = false) {
  const host = $<HTMLInputElement>('#communityShareHost')?.value.trim() ?? '';
  if (!pendingCommunityShare) return;
  const generation = ++communityShareLookupGeneration;
  if (communityShareLookupTimer !== null) window.clearTimeout(communityShareLookupTimer);
  communityShareLookupTimer = null;
  pendingCommunityShare.targetAsn = null;
  setText('#communityShareTargetAsn', host ? 'Определяю ASN целевой сети…' : 'ASN целевой сети не указан.');
  renderCommunitySharePreview();
  if (!host) return;
  communityShareLookupTimer = window.setTimeout(() => {
    communityShareLookupTimer = null;
    void lookupCommunityShareTargetAsn(host, generation);
  }, immediate ? 0 : 500);
}

function clearCommunityShare() {
  communityShareLookupGeneration++;
  if (communityShareLookupTimer !== null) window.clearTimeout(communityShareLookupTimer);
  communityShareLookupTimer = null;
  pendingCommunityShare = null;
}

async function lookupCommunityShareTargetAsn(host: string, generation: number) {
  try {
    const result = await nfqwsCommunityStrategies.targetAsn(host);
    if (generation !== communityShareLookupGeneration || !pendingCommunityShare) return;
    pendingCommunityShare.targetAsn = result.asn;
    setText('#communityShareTargetAsn', result.asn ? `Целевая сеть: AS${result.asn}` : 'ASN целевой сети не определён; сайт будет указан без привязки к сети.');
    renderCommunitySharePreview();
  } catch {
    if (generation !== communityShareLookupGeneration || !pendingCommunityShare) return;
    setText('#communityShareTargetAsn', 'ASN целевой сети не удалось определить; сайт будет указан без привязки к сети.');
  }
}

function renderCommunitySharePreview() {
  if (!pendingCommunityShare) return;
  const asn = $<HTMLInputElement>('#communityShareAsn')?.value.trim() || '—';
  const provider = $<HTMLInputElement>('#communityShareProvider')?.value.trim() || '';
  const host = $<HTMLInputElement>('#communityShareHost')?.value.trim() || 'не указан';
  const candidate = pendingCommunityShare.candidate;
  const proto = pendingCommunityShare.protocol === 'http' ? 'http' : 'tls';
  const preview = $('#communitySharePreview');
  if (preview) preview.textContent = [`Провайдер: AS${asn}${provider ? ` · ${provider}` : ''}`, `Протокол: ${proto}`, `Сайт: ${host}`, `Целевая сеть: ${pendingCommunityShare.targetAsn ? `AS${pendingCommunityShare.targetAsn}` : 'не определена'}`, `Результат: ${candidate.successes}/${candidate.attempts}`, '', ...candidate.candidate.steps].join('\n');
}

function submitCommunityShare() {
  if (!pendingCommunityShare) return;
  const asn = $('#communityShareAsn'); const provider = $('#communityShareProvider'); const host = $('#communityShareHost');
  if (!(asn instanceof HTMLInputElement) || !(provider instanceof HTMLInputElement) || !(host instanceof HTMLInputElement)) return;
  const asnValue = asn.value.trim().replace(/^AS/i, '');
  if (!/^\d{1,10}$/.test(asnValue) || Number(asnValue) < 1 || Number(asnValue) > 4_294_967_295) { setText('#communityShareStatus', 'Укажите корректный ASN провайдера.'); asn.focus(); return; }
  const share = pendingCommunityShare;
  const candidate = share.candidate;
  const proto = share.protocol === 'http' ? 'http' : 'tls';
  const outcome = candidate.confirmed ? 'works' : 'fails';
  const tries = candidate.confirmed ? candidate.attempts : Math.max(candidate.attempts, candidate.successes + 1);
  const url = buildCommunityStrategyIssueUrl({
    title: `${outcome === 'fails' ? 'Не сработала' : candidate.candidate.source.includes('Стратегии сообщества') ? 'Подтверждаю' : 'Новая стратегия'} · AS${asnValue} · ${proto} · ${host.value.trim() || candidate.candidate.name}`,
    outcome, asn: asnValue, provider: provider.value.trim(), proto,
    targetHost: host.value.trim(), targetAsn: share.targetAsn, strategy: candidate.candidate.steps.join('\n'),
    result: `${candidate.successes}/${tries}`, nfqws2: status?.service.version ?? '', ui: status?.panel.version ?? '',
  });
  void nfqwsCommunityStrategies.setProvider(Number(asnValue), provider.value.trim()).catch(() => {});
  window.open(url, '_blank', 'noopener,noreferrer');
  const modal = $('#communityShareModal'); if (modal) closeAppModal(modal);
  clearCommunityShare();
}
let snapshotOverview: SnapshotOverview | null = null;
let logPollingTimer: number | null = null;
let currentLogSource: LogSource = 'nfqws2-ui';
let logRequestPending = false;
let logRequestGeneration = 0;
let currentLogContent = '';
let pendingConfirmation: (() => Promise<void>) | null = null;
let pendingConfirmationCancel: (() => void) | null = null;
const appModalReturnFocus = new WeakMap<HTMLElement, HTMLElement | null>();
function openAppModal(root: HTMLElement, focusSelector?: string) {
  if (!root.dataset.modalOpen) appModalReturnFocus.set(root, document.activeElement instanceof HTMLElement ? document.activeElement : null);
  root.dataset.modalOpen = 'true'; root.setAttribute('aria-hidden', 'false');
  root.classList.remove('hidden'); root.classList.add('flex');
  const dialog = root.querySelector<HTMLElement>('[aria-modal="true"]');
  const focusTarget = (focusSelector ? root.querySelector<HTMLElement>(focusSelector) : null)
    ?? dialog?.querySelector<HTMLElement>('button:not(:disabled),input:not(:disabled),textarea:not(:disabled),select:not(:disabled),[tabindex="0"]')
    ?? dialog;
  focusTarget?.focus();
}
function closeAppModal(root: HTMLElement | null) {
  if (!root) return;
  root.dataset.modalOpen = 'false'; root.setAttribute('aria-hidden', 'true');
  root.classList.add('hidden'); root.classList.remove('flex');
  const previous = appModalReturnFocus.get(root); appModalReturnFocus.delete(root);
  if (previous?.isConnected) previous.focus();
}
function closeAppModalRoot(root: HTMLElement) {
  switch (root.id) {
    case 'configClearModal': setConfigClearModal(false); break;
    case 'listClearModal': setListClearModal(false); break;
    case 'strategyHistoryClearModal': setStrategyPickHistoryClearModal(false); break;
    case 'customListDeleteModal': closeCustomListDeleteModal(); break;
    case 'operationConfirmModal': closeDangerousConfirmation(); break;
    case 'profileImportDialog': cancelProfileImport(); break;
    case 'updateModalBackdrop': closeUpdateModal(); break;
    case 'diffModalBackdrop': closeAppModal(root); break;
    case 'communityShareModal': closeAppModal(root); clearCommunityShare(); break;
  }
}
function trapAppModalFocus(event: KeyboardEvent) {
  const dialog = [...document.querySelectorAll<HTMLElement>('[aria-modal="true"]')].find((item) => {
    const root = item.closest<HTMLElement>('[data-app-modal-root]');
    return Boolean(root?.dataset.modalOpen === 'true');
  });
  const root = dialog?.closest<HTMLElement>('[data-app-modal-root]');
  if (!dialog || !root) return;
  if (event.key === 'Escape') { event.preventDefault(); closeAppModalRoot(root); return; }
  if (event.key !== 'Tab') return;
  const focusable = [...dialog.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),textarea:not(:disabled),select:not(:disabled),a[href],[tabindex]:not([tabindex="-1"])')].filter((item) => item.getClientRects().length > 0);
  const first = focusable[0]; const last = focusable.at(-1);
  if (!first || !last) { event.preventDefault(); dialog.focus(); return; }
  if (event.shiftKey && (document.activeElement === first || !dialog.contains(document.activeElement))) { event.preventDefault(); last.focus(); }
  else if (!event.shiftKey && (document.activeElement === last || !dialog.contains(document.activeElement))) { event.preventDefault(); first.focus(); }
}
function closeAppModalOnBackdrop(event: MouseEvent) {
  if (!(event.target instanceof HTMLElement) || !event.target.hasAttribute('data-app-modal-root')) return;
  const root = event.target;
  if (root?.dataset.modalOpen === 'true') closeAppModalRoot(root);
}
let changeHistoryCache: ChangeHistoryEntry[] = [];
let changeHistoryCanRestoreInitial = false;
let runtimeProfilesState: NfqwsProfilesResponse | null = null;
let runtimeProfileLuaCatalog: NfqwsLuaFunctionInfo[] = [];
let runtimeProfileLuaStandardGroups: NfqwsLuaStandardGroup[] = [];
let runtimeProfileCliCatalog: NfqwsLuaCatalogResponse['cli'] | undefined;
let draggedRuntimeProfileStep: { profileId: string; index: number } | null = null;
let runtimeProfileDraft: string[][] = [];
let profileListInventory: NfqwsListSummary[] = [];
let selectedRuntimeProfileId = '';
let runtimeProfilesDirty = false;
let runtimeProfileManagedDraft: Record<string, string[]> = {};
let runtimeProfileManagedDirty = false;
let runtimeProfileEditorMode: 'builder' | 'text' = 'builder';
const RUNTIME_PROFILE_LABELS_KEY = 'nfqws2-ui-profile-labels';
function runtimeProfileLabelOverrides(): Record<string, string> {
  try {
    const value = JSON.parse(window.localStorage.getItem(RUNTIME_PROFILE_LABELS_KEY) ?? '{}') as unknown;
    return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, string> : {};
  } catch { return {}; }
}
function setRuntimeProfileLabelOverride(id: string, label: string) {
  try {
    const labels = runtimeProfileLabelOverrides(); labels[id] = label;
    window.localStorage.setItem(RUNTIME_PROFILE_LABELS_KEY, JSON.stringify(labels));
    return true;
  } catch { return false; }
}
function renderBackups(data: SnapshotOverview) {
  const host = $('#backupList'); if (!host) return;
  snapshotOverview = data;
  setText('#snapshotCount', `${data.localCount} ${pluralize(data.localCount, 'снимок', 'снимка', 'снимков')} · ${formatBytes(data.totalBytes)} из ${data.settings.maxLocalMib} МиБ`);
  const snapshotLimitBytes = data.settings.maxLocalMib * 1024 * 1024;
  setText('#snapshotStorageStatus', data.totalBytes > snapshotLimitBytes
    ? `Лимит превышен на ${formatBytes(data.totalBytes - snapshotLimitBytes)}. Закреплённые копии и копии, ожидающие отправки на NAS, автоматически не удаляются.`
    : `Лимит ${data.settings.maxLocalMib} МиБ. При превышении удаляются старейшие незакреплённые копии.`);
  const freeSpaceState = snapshotFreeSpaceState(data.availableBytes, data.maxArchiveBytes);
  setText('#snapshotDiskSpaceStatus', freeSpaceState === 'unknown'
    ? 'Свободное место на разделе снимков определить не удалось.'
    : freeSpaceState === 'low'
      ? `На разделе свободно ${formatBytes(data.availableBytes!)} — меньше максимального размера архива (${formatBytes(data.maxArchiveBytes)}). Новый снимок может не поместиться.`
      : `Свободно на разделе снимков: ${formatBytes(data.availableBytes!)}.`);
  $('#snapshotDiskSpaceStatus')?.classList.toggle('text-amber-300', freeSpaceState === 'low');
  $('#snapshotDiskSpaceStatus')?.classList.toggle('text-slate-400', freeSpaceState !== 'low');
  setText('#snapshotDirectory', data.directory);
  setValue('#snapshotAutoEnabled', String(data.settings.autoEnabled), true);
  const automaticSnapshots = $<HTMLInputElement>('#snapshotAutoEnabled'); if (automaticSnapshots) automaticSnapshots.disabled = false;
  setText('#backupStatus', data.settings.autoEnabled
    ? 'Автокопии включены: перед изменениями в панели и ежедневно в 04:05.'
    : 'Автокопии выключены. Восстановление снимка всё ещё создаёт страховочную копию.');
  setText('#snapshotRemoteStatus', !data.remote.tool ? 'На роутере нет SSH-клиента' : data.remote.error ? `Ошибка отправки: ${data.remote.error}` : data.remote.on ? `Включено · на NAS ${data.remote.stored} · ждут отправки ${data.remote.pending}` : 'Отправка выключена — снимки хранятся на роутере');
  setValue('#snapshotDays', String(data.settings.maxDays)); setValue('#snapshotMaxCount', String(data.settings.maxCount)); setValue('#snapshotMaxLocalMib', String(data.settings.maxLocalMib));
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
    const title = document.createElement('strong'); title.className = 'block text-xs font-semibold text-white'; title.textContent = `${snapshotReason(item.reason)}${item.pinned ? ' · закреплён' : ''}`;
    const date = document.createElement('span'); date.className = 'text-[11px] text-slate-400'; date.textContent = new Date(item.ts * 1000).toLocaleString();
    const note = document.createElement('span'); note.className = 'ml-2 break-words text-[11px] text-slate-500'; note.textContent = item.note || `${item.local && item.remote ? 'Роутер и NAS' : item.local ? 'На роутере' : 'На NAS'} · ${formatBytes(item.size)}`;
    const meta = document.createElement('div'); meta.className = 'mt-0.5'; meta.append(date, note);
    details.append(title, meta);
    const actions = document.createElement('div'); actions.className = 'flex flex-wrap gap-2';
    const download = document.createElement('a'); download.className = 'backup-action'; download.href = `/api/snapshots/${encodeURIComponent(item.id)}/archive`; download.textContent = 'Скачать';
    const restore = document.createElement('button'); restore.type = 'button'; restore.className = 'backup-action'; restore.dataset.actionClick = 'restoreSnapshot'; restore.dataset.args = JSON.stringify([item.id]); restore.textContent = 'Восстановить';
    const pin = document.createElement('button'); pin.type = 'button'; pin.className = 'backup-action'; pin.dataset.actionClick = 'pinSnapshot'; pin.dataset.args = JSON.stringify([item.id, !item.pinned]); pin.textContent = item.pinned ? 'Открепить' : 'Закрепить';
    pin.disabled = !item.local;
    const remove = document.createElement('button'); remove.type = 'button'; remove.className = 'backup-action text-rose-200'; remove.dataset.actionClick = 'deleteSnapshot'; remove.dataset.args = JSON.stringify([item.id]); remove.textContent = 'Удалить'; remove.disabled = !item.local || item.pinned;
    actions.append(download, restore, pin, remove); row.append(details, actions); host.append(row);
  }
}
async function loadBackups() {
  const label = $('#backupStatus'); if (label) label.textContent = 'Загружаем список снимков с роутера…';
  try { renderBackups(await nfqwsSnapshots.get()); }
  catch (error) { if (label) label.textContent = error instanceof Error ? `Не удалось загрузить снимки: ${error.message}` : 'Не удалось загрузить резервные копии.'; }
}
function setValue(selector: string, value: string, checked = false) { const element = $(selector); if (element instanceof HTMLInputElement) { if (checked) element.checked = value === 'true'; else element.value = value; } }
function formatBytes(value: number): string { if (value < 1024) return `${value} Б`; if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} КБ`; return `${(value / (1024 * 1024)).toFixed(1)} МБ`; }
function pluralize(value: number, one: string, few: string, many: string): string { const n = Math.abs(value) % 100; const last = n % 10; return n > 10 && n < 20 ? many : last > 1 && last < 5 ? few : last === 1 ? one : many; }
function snapshotReason(reason: string): string { return ({ 'вручную': 'Вручную', 'перед изменением': 'Перед изменением', 'ежедневный': 'Ежедневный', 'перед восстановлением': 'Перед восстановлением', 'из файла': 'Из файла' } as Record<string, string>)[reason] ?? reason; }
async function createSnapshot() {
  try { const item = await nfqwsSnapshots.create(); toast(`Снимок создан · ${new Date(item.ts * 1000).toLocaleString()}`); await loadBackups(); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось создать снимок.', 'error'); }
}
async function restoreSnapshot(id: string) {
  requestDangerousConfirmation('Восстановить резервную копию?', 'Конфигурация, списки и настройки будут восстановлены из снимка. Перед записью панель сохранит текущее состояние.', async () => {
    try { const result = await nfqwsSnapshots.restore(id); toast(result.message); await Promise.all([loadBackups(), loadConfig(), fetchStatus(), syncAppearanceFromRouter()]); }
    catch (error) { toast(error instanceof Error ? error.message : 'Не удалось восстановить снимок.', 'error'); }
  });
}
async function restoreSnapshotFile(input: HTMLInputElement) {
  const file = input.files?.[0]; if (!file) return;
  requestDangerousConfirmation('Восстановить архив?', `Будут заменены файлы панели и nfqws2 из «${file.name}». Перед восстановлением создастся снимок текущего состояния.`, async () => {
    try { const result = await nfqwsSnapshots.restoreFile(file); toast(result.message ?? `Восстановлено файлов: ${result.changed?.length ?? 0}`); await Promise.all([loadBackups(), loadConfig(), fetchStatus(), syncAppearanceFromRouter()]); }
    catch (error) { toast(error instanceof Error ? error.message : 'Не удалось восстановить архив.', 'error'); }
    finally { input.value = ''; }
  }, () => { input.value = ''; });
}
function readSnapshotSettings() {
  const current = snapshotOverview?.settings; if (!current) throw new Error('Сначала загрузите настройки снимков.');
  const number = (selector: string, fallback: number) => Number.parseInt(($<HTMLInputElement>(selector)?.value ?? ''), 10) || fallback;
  return { ...current, autoEnabled: ($<HTMLInputElement>('#snapshotAutoEnabled')?.checked ?? current.autoEnabled), maxDays: number('#snapshotDays', current.maxDays), maxCount: number('#snapshotMaxCount', current.maxCount), maxLocalMib: number('#snapshotMaxLocalMib', current.maxLocalMib), remote: { ...current.remote, enabled: ($<HTMLInputElement>('#snapshotRemoteEnabled')?.checked ?? false), host: $<HTMLInputElement>('#snapshotRemoteHost')?.value.trim() ?? '', port: number('#snapshotRemotePort', current.remote.port), user: $<HTMLInputElement>('#snapshotRemoteUser')?.value.trim() ?? '', dir: $<HTMLInputElement>('#snapshotRemoteDir')?.value.trim() ?? '', keepLocal: number('#snapshotRemoteKeep', current.remote.keepLocal) } };
}
async function saveSnapshotSettings(remoteOnly = false) {
  const settings = readSnapshotSettings();
  if (remoteOnly && snapshotOverview) { settings.maxDays = snapshotOverview.settings.maxDays; settings.maxCount = snapshotOverview.settings.maxCount; settings.maxLocalMib = snapshotOverview.settings.maxLocalMib; }
  await nfqwsSnapshots.saveSettings(settings); toast('Настройки резервных копий сохранены.'); await loadBackups();
}
async function toggleAutomaticSnapshots(enabled: boolean) {
  const input = $<HTMLInputElement>('#snapshotAutoEnabled');
  try { await saveSnapshotSettings(); }
  catch (error) {
    if (input && snapshotOverview) input.checked = snapshotOverview.settings.autoEnabled;
    toast(error instanceof Error ? error.message : `Не удалось ${enabled ? 'включить' : 'выключить'} автокопии.`, 'error');
  }
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
async function deleteSnapshot(id: string) {
  requestDangerousConfirmation('Удалить локальную копию?', 'Архив будет удалён с роутера. Если копия уже отправлена на NAS, она там останется.', async () => {
    try { await nfqwsSnapshots.delete(id); toast('Локальная копия удалена.'); await loadBackups(); }
    catch (error) { toast(error instanceof Error ? error.message : 'Не удалось удалить снимок.', 'error'); }
  });
}

function profileFromArgs(index: number, args: string[], editable: boolean): NfqwsArgumentProfile {
  const source = editable ? 'NFQWS_ARGS_CUSTOM' : 'Управляемая группа nfqws2';
  const explicitName = args.find((argument) => argument.startsWith('--name='))?.slice('--name='.length);
  const name = explicitName || (editable ? `Свой профиль ${index}` : `Профиль ${index}`);
  return {
    id: editable ? `custom-${index}` : `draft-managed-${index}`, index, name, source, editable, hasConstructor: editable, effectiveArgs: args, args,
    filters: args.filter((argument) => argument.startsWith('--filter-') && !argument.startsWith('--filter-7=') && !argument.startsWith('--filter-l7=')),
    protocols: args.filter((argument) => argument.startsWith('--filter-7=') || argument.startsWith('--filter-l7=')),
    payloads: args.filter((argument) => argument.startsWith('--payload=')),
    lists: args.filter((argument) => argument.startsWith('--hostlist') || argument.startsWith('--ipset') || argument.startsWith('--autohostlist')),
    steps: args.filter((argument) => argument.startsWith('--lua-desync=') || argument.startsWith('--payload=') || argument.startsWith('--in-range=') || argument.startsWith('--out-range=')),
  };
}
function visibleRuntimeProfiles(): NfqwsArgumentProfile[] {
  if (!runtimeProfilesState) return [];
  const managed = runtimeProfilesState.profiles.filter((profile) => !profile.editable).map((profile) => {
    const target = getRuntimeProfileConstructorTarget(profile);
    const args = target ? [...(runtimeProfileManagedDraft[target.source] ?? target.args)] : profile.args;
    const linkedArgs = target?.linkedComponents.flatMap((component) => runtimeProfileManagedDraft[component.source] ?? component.args) ?? [];
    const syntheticArgs = target ? profile.args.slice(target.args.length + target.linkedComponents.reduce((total, component) => total + component.args.length, 0)) : [];
    const effectiveArgs = target ? [...args, ...linkedArgs, ...syntheticArgs] : profile.args;
    return {
      ...profile, hasConstructor: Boolean(target), effectiveArgs, args,
      filters: args.filter((argument) => argument.startsWith('--filter-') && !argument.startsWith('--filter-7=') && !argument.startsWith('--filter-l7=')),
      protocols: args.filter((argument) => argument.startsWith('--filter-7=') || argument.startsWith('--filter-l7=')),
      payloads: args.filter((argument) => argument.startsWith('--payload=')),
      steps: args.filter((argument) => argument.startsWith('--lua-desync=') || argument.startsWith('--payload=') || argument.startsWith('--in-range=') || argument.startsWith('--out-range=')),
    };
  });
  const custom = runtimeProfileDraft.map((args, index) => profileFromArgs(index + 1, args, true));
  return prepareVisibleRuntimeProfiles([...custom, ...managed]);
}
function appendProfileBuilderSectionHeading(parent: HTMLElement, text: string) {
  const tone = text === 'Когда срабатывает' ? 'profile-builder-heading-condition' : text === 'Для кого' ? 'profile-builder-heading-scope' : 'profile-builder-heading-action';
  const heading = document.createElement('h3'); heading.className = `profile-builder-section-heading ${tone}`; heading.textContent = text; parent.append(heading);
}
function openProfileChoiceDialog(choices: Array<{ group: string; value: string; label: string }>, headingText: string, onSelect: (value: string) => void) {
  const dialog = document.createElement('dialog'); dialog.className = 'profile-choice-dialog'; dialog.setAttribute('aria-labelledby', 'profileChoiceTitle');
  const panel = document.createElement('div'); panel.className = 'profile-choice-panel';
  const heading = document.createElement('div'); heading.className = 'profile-choice-heading';
  const title = document.createElement('h3'); title.id = 'profileChoiceTitle'; title.textContent = headingText;
  const close = document.createElement('button'); close.type = 'button'; close.className = 'backup-action'; close.textContent = 'Закрыть'; close.addEventListener('click', () => dialog.close()); heading.append(title, close);
  const search = document.createElement('input'); search.type = 'search'; search.className = 'profile-choice-search'; search.placeholder = 'Найти по названию или назначению'; search.setAttribute('aria-label', `Поиск: ${headingText.toLocaleLowerCase()}`);
  const results = document.createElement('div'); results.className = 'profile-choice-results'; results.setAttribute('role', 'listbox'); results.setAttribute('aria-label', headingText);
  const drawChoices = () => {
    results.replaceChildren();
    const matches = filterProfileArgumentChoices(choices, search.value);
    if (!matches.length) { const empty = document.createElement('p'); empty.className = 'profile-choice-empty'; empty.textContent = 'Ничего не найдено. Попробуйте другое название.'; results.append(empty); return; }
    const groups = new Map<string, typeof matches>();
    for (const match of matches) groups.set(match.group, [...(groups.get(match.group) ?? []), match]);
    let groupIndex = 0; let optionIndex = 0;
    for (const [groupName, groupChoices] of groups) {
      const section = document.createElement('section'); section.className = 'profile-choice-group'; section.setAttribute('role', 'group');
      const groupTitle = document.createElement('h4'); groupTitle.id = `profileChoiceGroup${groupIndex++}`; groupTitle.textContent = groupName; section.setAttribute('aria-labelledby', groupTitle.id); section.append(groupTitle);
      for (const choice of groupChoices) {
        const item = document.createElement('button'); item.type = 'button'; item.className = 'profile-choice-option'; item.setAttribute('role', 'option'); item.setAttribute('aria-selected', 'false'); item.tabIndex = optionIndex++ === 0 ? 0 : -1;
        const label = document.createElement('strong'); label.textContent = choice.label;
        const syntax = document.createElement('code'); syntax.textContent = choice.value;
        item.append(label, syntax); item.addEventListener('click', () => { dialog.close(); onSelect(choice.value); }); section.append(item);
      }
      results.append(section);
    }
  };
  search.addEventListener('input', drawChoices);
  search.addEventListener('keydown', (event) => {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    const options = [...results.querySelectorAll<HTMLButtonElement>('[role="option"]')]; if (!options.length) return;
    event.preventDefault(); options[event.key === 'ArrowDown' ? 0 : options.length - 1]?.focus();
  });
  results.addEventListener('keydown', (event) => {
    const option = event.target instanceof Element ? event.target.closest<HTMLButtonElement>('[role="option"]') : null; if (!option) return;
    const options = [...results.querySelectorAll<HTMLButtonElement>('[role="option"]')]; const nextIndex = nextProfileChoiceIndex(options.indexOf(option), event.key, options.length); if (nextIndex === null) return;
    event.preventDefault(); options.forEach((item, index) => { item.tabIndex = index === nextIndex ? 0 : -1; }); options[nextIndex]?.focus();
  });
  drawChoices(); panel.append(heading, search, results); dialog.append(panel);
  dialog.addEventListener('click', (event) => { if (event.target === dialog) dialog.close(); });
  document.body.append(dialog); dialog.addEventListener('close', () => dialog.remove(), { once: true }); dialog.showModal(); search.focus();
}
function runtimeProfileDraftTarget(id: string): { source: string; args: string[]; managed: boolean; customIndex: number | null } | null {
  const customMatch = /^custom-(\d+)$/.exec(id);
  if (customMatch) {
    const customIndex = Number(customMatch[1]) - 1;
    const args = runtimeProfileDraft[customIndex];
    return args ? { source: 'NFQWS_ARGS_CUSTOM', args: [...args], managed: false, customIndex } : null;
  }
  const profile = runtimeProfilesState?.profiles.find((item) => item.id === id);
  if (!profile) return null;
  const target = getRuntimeProfileConstructorTarget(profile);
  if (!target) return null;
  return { source: target.source, args: [...(runtimeProfileManagedDraft[target.source] ?? target.args)], managed: true, customIndex: null };
}
function storeRuntimeProfileArgs(id: string, args: string[], redraw = true) {
  const target = runtimeProfileDraftTarget(id);
  if (!target) return false;
  if (target.managed) runtimeProfileManagedDraft[target.source] = [...args];
  else if (target.customIndex !== null) runtimeProfileDraft[target.customIndex] = [...args];
  markRuntimeProfilesDirty(id, redraw);
  return true;
}
function markRuntimeProfilesDirty(id?: string, redraw = true) {
  if (id && !id.startsWith('custom-')) runtimeProfileManagedDirty = true;
  else runtimeProfilesDirty = true;
  const status = $('#profileStatus');
  if (status) status.textContent = 'Есть несохранённые изменения. Применение сохранит снимок, проверит конфиг и перезапустит nfqws2.';
  if (redraw) renderRuntimeProfiles();
}
function renderRuntimeProfiles() {
  const list = $('#profileList'); const editor = $('#profileEditor');
  if (!list || !editor || !runtimeProfilesState) return;
  runtimeProfileTextCodeEditor?.destroy(); runtimeProfileTextCodeEditor = null;
  const previousEditorMode = editor.querySelector<HTMLElement>('#runtimeProfileModeToggle')?.dataset.mode as 'builder' | 'text' | undefined;
  const profiles = visibleRuntimeProfiles().map((profile) => ({ ...profile, name: profile.editable ? profile.name : runtimeProfileLabelOverrides()[profile.id] ?? profile.name }));
  const orderWarnings = profileOrderWarnings(profiles.map((profile) => profile.effectiveArgs ?? profile.args));
  if (!profiles.some((profile) => profile.id === selectedRuntimeProfileId)) selectedRuntimeProfileId = profiles[0]?.id ?? '';
  list.replaceChildren();
  const sideHeading = document.createElement('p'); sideHeading.className = 'px-2 pb-1 text-[10px] font-bold text-slate-400'; sideHeading.textContent = 'Профили'; list.append(sideHeading);
  for (const [profileIndex, profile] of profiles.entries()) {
    const button = document.createElement('button'); button.type = 'button'; button.dataset.actionClick = 'selectRuntimeProfile'; button.dataset.args = JSON.stringify([profile.id]);
    button.setAttribute('aria-current', profile.id === selectedRuntimeProfileId ? 'true' : 'false');
    button.className = `flex w-full items-start gap-2 rounded-lg px-2 py-2 text-left text-sm transition-colors ${profile.id === selectedRuntimeProfileId ? 'bg-blue-500/20 text-blue-200' : 'text-slate-300 hover:bg-slate-800'}`;
    const num = document.createElement('span'); num.className = 'mt-0.5 grid h-5 w-5 shrink-0 place-items-center rounded-full border border-emerald-400 text-[11px] font-bold text-emerald-300'; num.textContent = String(profile.index);
    const nameWrap = document.createElement('span'); nameWrap.className = 'grid min-w-0 flex-1';
    const name = document.createElement('span'); name.className = 'truncate font-medium'; name.textContent = profile.name;
    nameWrap.append(name);
    const warnings = diagnoseProfileLuaArguments(profile.effectiveArgs ?? profile.args, runtimeProfileLuaCatalog, runtimeProfileLuaStandardGroups);
    const ruleWarnings = orderWarnings[profileIndex] ?? [];
    const warningCount = warnings.length + ruleWarnings.length;
    const badge = document.createElement('span'); badge.className = warningCount ? 'rounded-full bg-amber-500/15 px-1.5 text-[10px] font-semibold text-amber-300' : 'sr-only'; badge.textContent = warningCount ? String(warningCount) : profile.editable ? 'Пользовательский профиль' : 'Штатный профиль'; badge.setAttribute('aria-label', warningCount ? `Предупреждений профиля: ${[...warnings.map((item) => item.message), ...ruleWarnings].join('. ')}` : profile.editable ? 'Пользовательский профиль' : 'Штатный профиль');
    button.append(num, nameWrap, badge); list.append(button);
  }
  editor.replaceChildren();
  const profile = profiles.find((item) => item.id === selectedRuntimeProfileId);
  if (!profile) {
    const empty = document.createElement('p'); empty.className = 'text-sm text-slate-400'; empty.textContent = 'Профили не найдены в текущей конфигурации. Можно добавить пользовательский профиль.'; editor.append(empty); return;
  }
  const heading = document.createElement('div'); heading.className = 'flex flex-wrap items-start justify-between gap-3 border-b border-slate-800 pb-3';
  const title = document.createElement('div');
  const name = document.createElement('h2'); name.className = 'font-bold text-white'; name.textContent = `#${profile.index} ${profile.name}`;
  const profileWarnings = diagnoseProfileLuaArguments(profile.effectiveArgs ?? profile.args, runtimeProfileLuaCatalog, runtimeProfileLuaStandardGroups);
  if (profileWarnings.length) { const warningBadge = document.createElement('span'); warningBadge.className = 'ml-2 inline-flex rounded-full bg-amber-500/15 px-2 py-0.5 align-middle text-[10px] font-semibold text-amber-300'; warningBadge.textContent = `${profileWarnings.length} предупреждений`; warningBadge.setAttribute('aria-label', profileWarnings.map((item) => item.message).join('. ')); name.append(warningBadge); }
  const argumentVariable = profile.editable ? 'NFQWS_ARGS_CUSTOM' : getRuntimeProfileConstructorTarget(profile)?.source ?? profile.source;
  const source = document.createElement('p'); source.className = 'mt-1 text-xs text-slate-400'; source.textContent = `${profile.editable ? `Пользовательский профиль · ${argumentVariable}` : `Штатная группа · ${argumentVariable}`} · параметры передаются в nfqws2`;
  const titleLine = document.createElement('div'); titleLine.className = 'flex flex-wrap items-center gap-2'; titleLine.append(name);
  title.append(titleLine, source); heading.append(title);
  const headerControls = document.createElement('div'); headerControls.className = 'flex flex-wrap items-center gap-2';
  if (profile.hasConstructor) {
    const modeButtons = document.createElement('div'); modeButtons.id = 'runtimeProfileModeToggle'; modeButtons.className = 'runtime-profile-mode-toggle';
    modeButtons.dataset.mode = previousEditorMode ?? runtimeProfileEditorMode;
    for (const [mode, label] of [['builder', 'Конструктор'], ['text', 'Текст']] as const) {
      const button = document.createElement('button'); button.type = 'button'; button.dataset.actionClick = 'setRuntimeProfileMode'; button.dataset.args = JSON.stringify([mode]); button.dataset.modeChoice = mode; button.setAttribute('aria-pressed', String(runtimeProfileEditorMode === mode)); button.className = 'runtime-profile-mode-option'; button.textContent = label; modeButtons.append(button);
    }
    headerControls.append(modeButtons);
    if (previousEditorMode && previousEditorMode !== runtimeProfileEditorMode) {
      requestAnimationFrame(() => { if (document.contains(modeButtons)) modeButtons.dataset.mode = runtimeProfileEditorMode; });
    }
  }
  if (profile.editable) {
    const actions = document.createElement('div'); actions.className = 'flex flex-wrap gap-1';
    for (const [action, label, accessible] of [['shareRuntimeProfile', 'Для чата', 'Скопировать профиль для отправки в чат'], ['copyRuntimeProfile', 'Копия', 'Создать копию профиля'], ['moveRuntimeProfileUp', '↑', 'Переместить профиль выше'], ['moveRuntimeProfileDown', '↓', 'Переместить профиль ниже'], ['deleteRuntimeProfile', 'Удалить', 'Удалить профиль из черновика']] as const) {
      const button = document.createElement('button'); button.type = 'button'; button.dataset.actionClick = action; button.dataset.args = JSON.stringify([profile.id]); button.className = 'backup-action'; button.textContent = label; button.setAttribute('aria-label', accessible); if (action === 'copyRuntimeProfile') button.dataset.actionTooltip = accessible; actions.append(button);
    }
    headerControls.append(actions);
  } else {
    const actions = document.createElement('div'); actions.className = 'flex flex-wrap items-center gap-1.5';
    const copy = document.createElement('button'); copy.type = 'button'; copy.dataset.actionClick = 'copyRuntimeProfile'; copy.dataset.args = JSON.stringify([profile.id]); copy.dataset.actionTooltip = 'Скопировать в пользовательские профили для редактирования'; copy.className = 'backup-action'; copy.textContent = 'Копия';
    const share = document.createElement('button'); share.type = 'button'; share.dataset.actionClick = 'shareRuntimeProfile'; share.dataset.args = JSON.stringify([profile.id]); share.dataset.actionTooltip = 'Скопировать профиль вместе с подписью версии nfqws2'; share.className = 'backup-action'; share.textContent = 'Для чата';
    actions.append(share, copy); headerControls.append(actions);
  }
  heading.append(headerControls);
  editor.append(heading);
  const nameField = document.createElement('label'); nameField.className = 'runtime-profile-name-field';
  const nameLabel = document.createElement('span'); nameLabel.textContent = 'Имя';
  const nameInput = document.createElement('input'); nameInput.type = 'text'; nameInput.name = 'runtimeProfileName'; nameInput.value = profile.name; nameInput.maxLength = 32; nameInput.autocomplete = 'off'; nameInput.spellcheck = false; nameInput.className = 'runtime-profile-name-input'; nameInput.setAttribute('aria-label', 'Имя профиля');
  const saveProfileName = () => {
    const nextName = nameInput.value.trim();
    if (!/^[\p{L}\p{N}_.-]{1,32}$/u.test(nextName)) { toast('Имя: буквы, цифры, точка, дефис или подчёркивание; не больше 32 символов.', 'error'); nameInput.value = profile.name; nameInput.focus(); return; }
    if (nextName === profile.name) return;
    if (profile.editable) renameRuntimeProfile(profile.id, nextName);
    else if (!setRuntimeProfileLabelOverride(profile.id, nextName)) { toast('Не удалось сохранить название в хранилище браузера.', 'error'); nameInput.value = profile.name; return; }
    renderRuntimeProfiles(); toast('Название профиля сохранено.');
  };
  nameInput.addEventListener('change', saveProfileName);
  nameInput.addEventListener('keydown', (event) => { if (event.key === 'Enter') { event.preventDefault(); saveProfileName(); nameInput.blur(); } });
  nameField.append(nameLabel, nameInput); editor.append(nameField);
  const profileOrderIssues = orderWarnings[profiles.findIndex((item) => item.id === profile.id)] ?? [];
  for (const message of profileOrderIssues) { const notice = document.createElement('p'); notice.className = 'mt-2 rounded-md border border-amber-500/25 bg-amber-500/10 px-2.5 py-2 text-xs leading-5 text-amber-200'; notice.textContent = message; editor.append(notice); }
  if (profile.hasConstructor && runtimeProfileEditorMode === 'builder') {
    const builder = document.createElement('div'); builder.className = 'mt-2 grid gap-2';
    if (!profile.editable) {
      const target = runtimeProfilesState?.profiles.find((item) => item.id === profile.id);
      const source = target && getRuntimeProfileConstructorTarget(target)?.source;
      const shared = source === 'NFQWS_ARGS_QUIC' ? 'Эта переменная общая для «QUIC по IP» и «QUIC по сайтам».' : source === 'NFQWS_ARGS' ? 'Эта переменная общая для «HTTP/HTTPS по IP» и «HTTP/HTTPS по сайтам».' : null;
      if (shared) { const note = document.createElement('p'); note.className = 'text-xs leading-5 text-slate-400'; note.textContent = shared; builder.append(note); }
    }
    const groups: Array<[string, string, (argument: string) => boolean]> = [
      ['TCP-порты', 'condition', (arg) => arg.startsWith('--filter-tcp=')],
      ['UDP-порты', 'condition', (arg) => arg.startsWith('--filter-udp=')],
      ['Другие фильтры', 'condition', (arg) => arg.startsWith('--filter-') && !arg.startsWith('--filter-tcp=') && !arg.startsWith('--filter-udp=') && !arg.startsWith('--filter-l7=') && !arg.startsWith('--filter-7=')],
      ['Протоколы', 'condition', (arg) => arg.startsWith('--filter-l7=') || arg.startsWith('--filter-7=')],
      ['Списки доменов и IP', 'scope', (arg) => arg.startsWith('--hostlist') || arg.startsWith('--ipset') || arg.startsWith('--autohostlist')],
      ['Упорядоченные шаги', 'action', (arg) => arg.startsWith('--lua-desync=') || arg.startsWith('--payload=') || arg.startsWith('--in-range=') || arg.startsWith('--out-range=')],
      ['Другие параметры', 'other', (arg) => !arg.startsWith('--filter-') && !arg.startsWith('--filter-7=') && !arg.startsWith('--payload=') && !arg.startsWith('--in-range=') && !arg.startsWith('--out-range=') && !arg.startsWith('--hostlist') && !arg.startsWith('--ipset') && !arg.startsWith('--autohostlist') && !arg.startsWith('--lua-desync=') && !arg.startsWith('--name=')],
    ];
    let conditionsHeadingAdded = false; let listsHeadingAdded = false; let stepsHeadingAdded = false;
    for (const [groupName, groupTone, predicate] of groups) {
      const indexes = profile.args.map((arg, index) => predicate(arg) ? index : -1).filter((index) => index >= 0);
      if (!indexes.length && groupName === 'UDP-порты' && profile.hasConstructor) {
        if (!conditionsHeadingAdded) { appendProfileBuilderSectionHeading(builder, 'Когда срабатывает'); conditionsHeadingAdded = true; }
        const row = document.createElement('div'); row.className = 'profile-filter-empty-row flex flex-wrap items-center gap-2 px-1 py-1';
        const label = document.createElement('span'); label.className = 'w-28 text-xs text-slate-400'; label.textContent = 'UDP-порты';
        const addUdp = document.createElement('button'); addUdp.type = 'button'; addUdp.className = 'profile-filter-add-text'; addUdp.textContent = '+ добавить фильтр UDP-порты';
        addUdp.addEventListener('click', () => {
          const port = document.createElement('input'); port.type = 'text'; port.inputMode = 'numeric'; port.className = 'profile-filter-add-input min-w-36 rounded-md border bg-slate-950 px-2 py-1 font-mono text-xs'; port.placeholder = '443 или 5000-5010'; port.setAttribute('aria-label', 'UDP-порт или диапазон');
          const commit = () => {
            const target = runtimeProfileDraftTarget(profile.id);
            const updated = appendProfileUdpFilter(target?.args ?? profile.args, port.value);
            if (!updated) { toast('Введите UDP-порт от 1 до 65535 или корректный диапазон.', 'error'); port.focus(); return; }
            if (target) storeRuntimeProfileArgs(profile.id, updated);
          };
          const cancel = document.createElement('button'); cancel.type = 'button'; cancel.className = 'grid h-8 w-8 place-items-center rounded-md border border-slate-700 text-slate-400 hover:border-rose-500 hover:text-rose-300'; cancel.setAttribute('aria-label', 'Отменить добавление UDP-порта'); cancel.title = 'Отменить добавление UDP-порта';
          const cancelIcon = document.createElementNS('http://www.w3.org/2000/svg', 'svg'); cancelIcon.setAttribute('viewBox', '0 0 24 24'); cancelIcon.setAttribute('width', '16'); cancelIcon.setAttribute('height', '16'); cancelIcon.setAttribute('fill', 'none'); cancelIcon.setAttribute('stroke', 'currentColor'); cancelIcon.setAttribute('stroke-width', '2'); cancelIcon.setAttribute('stroke-linecap', 'round'); cancelIcon.setAttribute('aria-hidden', 'true');
          const cancelPath = document.createElementNS('http://www.w3.org/2000/svg', 'path'); cancelPath.setAttribute('d', 'M18 6 6 18M6 6l12 12'); cancelIcon.append(cancelPath); cancel.append(cancelIcon);
          cancel.addEventListener('click', () => { row.replaceChildren(label, addUdp); addUdp.focus(); });
          port.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') { event.preventDefault(); commit(); }
            else if (event.key === 'Escape') { event.preventDefault(); cancel.click(); }
          });
          const confirm = document.createElement('button'); confirm.type = 'button'; confirm.className = 'profile-filter-add-text'; confirm.textContent = 'Добавить'; confirm.addEventListener('click', commit);
          row.replaceChildren(label, port, confirm, cancel); port.focus();
        });
        row.append(label, addUdp); builder.append(row); continue;
      }
      if (!indexes.length && groupName === 'Протоколы' && profile.hasConstructor) {
        if (!conditionsHeadingAdded) { appendProfileBuilderSectionHeading(builder, 'Когда срабатывает'); conditionsHeadingAdded = true; }
        const row = document.createElement('div'); row.className = 'profile-filter-empty-row flex flex-wrap items-center gap-2 px-1 py-1';
        const label = document.createElement('span'); label.className = 'w-28 text-xs text-slate-400'; label.textContent = 'Протоколы';
        const select = document.createElement('select'); select.className = 'profile-protocol-select min-h-8 min-w-40 max-w-64 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-200'; select.setAttribute('aria-label', 'Добавить протокол'); select.dataset.menuMinWidth = '220';
        const choices = profileProtocolChoices(runtimeProfileCliCatalog?.l7 ?? [], profile.args);
        const placeholder = document.createElement('option'); placeholder.value = ''; placeholder.textContent = choices.length ? '+ протокол' : 'Каталог протоколов недоступен'; select.append(placeholder);
        for (const protocol of choices) { const option = document.createElement('option'); option.value = protocol; option.textContent = protocol; select.append(option); }
        select.disabled = choices.length === 0;
        select.addEventListener('change', () => {
          if (!select.value) return;
          const target = runtimeProfileDraftTarget(profile.id);
          const updated = appendProfileProtocol(target?.args ?? profile.args, select.value);
          if (updated && target) storeRuntimeProfileArgs(profile.id, updated);
        });
        row.append(label, select); builder.append(row); continue;
      }
      if (!indexes.length && groupName !== 'Упорядоченные шаги') continue;
      if (!conditionsHeadingAdded && ['TCP-порты', 'UDP-порты', 'Другие фильтры', 'Протоколы'].includes(groupName)) { appendProfileBuilderSectionHeading(builder, 'Когда срабатывает'); conditionsHeadingAdded = true; }
      if (!listsHeadingAdded && groupName === 'Списки доменов и IP') { appendProfileBuilderSectionHeading(builder, 'Для кого'); listsHeadingAdded = true; }
      if (!stepsHeadingAdded && groupName === 'Упорядоченные шаги') { appendProfileBuilderSectionHeading(builder, 'Что делает'); stepsHeadingAdded = true; }
      if (groupName === 'Упорядоченные шаги') {
        const section = document.createElement('div'); section.className = 'profile-builder-step-list grid gap-2 rounded-lg border border-slate-800 bg-slate-950/30 p-2';
        const heading = document.createElement('p'); heading.className = 'text-xs text-slate-400'; heading.textContent = 'Шаги выполняются сверху вниз; их порядок влияет на обработку пакетов.'; section.append(heading);
        const functions = runtimeProfileLuaCatalog.map((item) => item.name);
        const strategyNumbers = profileStrategyNumbers(profile.args);
        indexes.forEach((argumentIndex, stepIndex) => {
          const argument = profile.args[argumentIndex] ?? '';
          if (!argument.startsWith('--lua-desync=')) {
            const label = argument.startsWith('--payload=') ? 'Типы пакетов' : argument.startsWith('--out-range=') ? 'Исходящие пакеты' : 'Входящие пакеты';
            const row = document.createElement('div'); row.className = `profile-step-row ${argument.startsWith('--payload=') ? 'profile-step-payload' : 'profile-step-range'} grid gap-2 rounded-lg border border-slate-800 bg-slate-900/70 p-2 sm:grid-cols-[1rem_2rem_minmax(0,1fr)_auto] sm:items-center`; row.dataset.profileStepKind = argument.startsWith('--payload=') ? 'payload' : 'range';
            const grip = createProfileStepGrip(); bindProfileStepDrop(row, profile.id, stepIndex);
            const badge = document.createElement('span'); badge.className = 'profile-step-number grid h-6 w-6 place-items-center rounded-full border text-xs font-bold'; badge.textContent = label === 'Типы пакетов' ? 'P' : 'R';
            const input = document.createElement('input'); input.type = 'text'; input.className = 'min-w-0 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-xs text-blue-200'; input.value = argument; input.setAttribute('aria-label', `${label}, шаг ${stepIndex + 1}`); input.dataset.actionChange = 'updateRuntimeProfileArgument'; input.dataset.args = JSON.stringify([profile.id, String(argumentIndex), { source: 'value' }]);
            const controls = document.createElement('div'); controls.className = 'flex items-center gap-1';
            for (const [direction, text, accessible] of [['up', '↑', 'Переместить шаг выше'], ['down', '↓', 'Переместить шаг ниже']] as const) { const button = document.createElement('button'); button.type = 'button'; button.dataset.actionClick = 'moveRuntimeProfileStep'; button.dataset.args = JSON.stringify([profile.id, String(stepIndex), direction]); button.className = 'backup-action'; button.textContent = text; button.setAttribute('aria-label', accessible); button.disabled = direction === 'up' ? stepIndex === 0 : stepIndex === indexes.length - 1; controls.append(button); }
            const remove = document.createElement('button'); remove.type = 'button'; remove.dataset.actionClick = 'removeRuntimeProfileArgument'; remove.dataset.args = JSON.stringify([profile.id, String(argumentIndex)]); remove.className = 'backup-action'; remove.textContent = '×'; remove.setAttribute('aria-label', `Удалить шаг ${stepIndex + 1}`); controls.append(remove);
            row.append(grip, badge, input, controls); section.append(row); return;
          }
          const body = argument.slice('--lua-desync='.length);
          const [functionName = ''] = body.split(':');
          const strategyNumber = strategyNumbers[argumentIndex] ?? 0;
          const row = document.createElement('div'); row.className = 'profile-step-row profile-step-desync grid gap-2 rounded-lg border border-slate-800 bg-slate-900/70 p-2 sm:grid-cols-[1rem_2rem_minmax(0,1fr)_auto] sm:items-start'; row.dataset.profileStepKind = 'desync'; row.style.setProperty('--profile-step-hue', String((stepIndex * 47 + 166) % 360));
          const grip = createProfileStepGrip(); bindProfileStepDrop(row, profile.id, stepIndex);
          const badge = document.createElement('span'); badge.className = 'profile-step-number grid h-6 w-6 place-items-center rounded-full border text-xs font-bold'; badge.textContent = String(strategyNumber); badge.setAttribute('aria-label', `Стратегия ${strategyNumber}`); badge.title = `Стратегия ${strategyNumber}`;
          const content = document.createElement('div'); content.className = 'profile-step-content';
          const functionSelect = document.createElement('select'); functionSelect.className = 'profile-function-select min-h-8 w-full max-w-2xl rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-200'; functionSelect.dataset.menuMinWidth = '360';
          functionSelect.dataset.actionTooltip = `Стратегия ${strategyNumber}. Выберите Lua-функцию шага; доступность проверит nfqws2 при применении.`;
          functionSelect.addEventListener('change', () => updateRuntimeProfileStep(profile.id, String(argumentIndex), (current) => replaceDesyncFunction(current, functionSelect.value)));
          [...new Set([...functions, functionName])].sort().forEach((name) => { const option = document.createElement('option'); option.value = name; option.textContent = name; option.selected = name === functionName; functionSelect.append(option); });
          const functionInfo = runtimeProfileLuaCatalog.find((item) => item.name === functionName);
          const description = document.createElement('span'); description.className = 'text-xs leading-5 text-slate-400'; description.textContent = getRussianLuaFunctionDescription(functionName);
          const parameterList = document.createElement('div'); parameterList.className = 'profile-step-parameters';
          const parameterParts = splitDesyncParameters(argument);
          diagnoseProfileLuaArguments([argument], runtimeProfileLuaCatalog, runtimeProfileLuaStandardGroups).forEach((item) => { const warning = document.createElement('span'); warning.className = 'rounded-md border border-amber-500/30 bg-amber-500/10 px-2 py-1 text-xs text-amber-200'; warning.textContent = item.message; content.append(warning); });
          parameterParts.forEach((parameter, parameterIndex) => {
            const [key, ...valueParts] = parameter.split('='); const hasValue = parameter.includes('=');
            const chip = document.createElement('span'); chip.className = 'profile-parameter-chip';
            const keyLabel = document.createElement('span'); keyLabel.className = 'profile-parameter-key'; keyLabel.textContent = hasValue ? key : parameter; chip.append(keyLabel);
            if (hasValue) { const valueInput = document.createElement('input'); valueInput.type = 'text'; valueInput.className = 'profile-parameter-value'; valueInput.value = valueParts.join('='); valueInput.size = Math.min(20, Math.max(3, valueInput.value.length)); valueInput.setAttribute('aria-label', `Значение параметра ${key}`); valueInput.dataset.luaArgumentKey = key; valueInput.dataset.actionChange = 'setRuntimeProfileStepParameter'; valueInput.dataset.args = JSON.stringify([profile.id, String(argumentIndex), String(parameterIndex), { source: 'value' }]); chip.append(valueInput); }
            const removeParameter = document.createElement('button'); removeParameter.type = 'button'; removeParameter.dataset.actionClick = 'removeRuntimeProfileStepParameter'; removeParameter.dataset.args = JSON.stringify([profile.id, String(argumentIndex), String(parameterIndex)]); removeParameter.className = 'profile-parameter-remove'; removeParameter.textContent = '×'; removeParameter.setAttribute('aria-label', `Убрать параметр ${key}`); chip.append(removeParameter); parameterList.append(chip);
          });
          const documentedOptions = getLuaProfileParameterOptions(functionInfo, runtimeProfileLuaStandardGroups, functionName);
          const knownParameterSelect = document.createElement('select'); knownParameterSelect.className = 'min-h-8 w-full max-w-2xl rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-200'; knownParameterSelect.setAttribute('aria-label', 'Добавить документированный параметр');
          knownParameterSelect.dataset.menuMinWidth = '360';
          const knownPlaceholder = document.createElement('option'); knownPlaceholder.value = ''; knownPlaceholder.textContent = '+ параметр из каталога'; knownParameterSelect.append(knownPlaceholder);
          documentedOptions.forEach((definition) => { const parsed = parseLuaParameterOption(definition); if (!parsed || parameterParts.some((part) => part.split('=', 1)[0] === parsed.key)) return; const option = document.createElement('option'); option.value = definition; option.textContent = getRussianLuaArgumentDescription(definition); knownParameterSelect.append(option); });
          knownParameterSelect.addEventListener('change', () => { const parsed = parseLuaParameterOption(knownParameterSelect.value); if (parsed) addRuntimeProfileStepParameter(profile.id, String(argumentIndex), parsed.value === null ? parsed.key : `${parsed.key}=${parsed.value}`); });
          const addParameterInput = document.createElement('input'); addParameterInput.type = 'text'; addParameterInput.className = 'min-h-7 min-w-36 max-w-52 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-xs text-slate-200'; addParameterInput.placeholder = '+ вручную'; addParameterInput.setAttribute('aria-label', 'Новый параметр шага в формате key=value или флаг');
          const addParameter = document.createElement('button'); addParameter.type = 'button'; addParameter.className = 'backup-action'; addParameter.textContent = '+ параметр'; addParameter.addEventListener('click', () => { const value = addParameterInput.value.trim(); if (value) addRuntimeProfileStepParameter(profile.id, String(argumentIndex), value); });
          const parameterPicker = document.createElement('div'); parameterPicker.className = 'profile-parameter-picker'; parameterPicker.append(knownParameterSelect);
          const manualParameter = document.createElement('details'); manualParameter.className = 'profile-manual-parameter';
          const manualSummary = document.createElement('summary'); manualSummary.textContent = 'Свой параметр'; manualParameter.append(manualSummary);
          const manualControls = document.createElement('div'); manualControls.className = 'flex flex-wrap items-center gap-1.5'; manualControls.append(addParameterInput, addParameter); manualParameter.append(manualControls);
          parameterList.append(parameterPicker, manualParameter);
          content.append(functionSelect, description, parameterList);
          const controls = document.createElement('div'); controls.className = 'profile-step-controls';
          for (const [direction, label, accessible] of [['up', '↑', 'Переместить шаг выше'], ['down', '↓', 'Переместить шаг ниже']] as const) {
            const button = document.createElement('button'); button.type = 'button'; button.dataset.actionClick = 'moveRuntimeProfileStep'; button.dataset.args = JSON.stringify([profile.id, String(stepIndex), direction]); button.className = 'backup-action'; button.textContent = label; button.setAttribute('aria-label', accessible); button.dataset.actionTooltip = accessible; button.disabled = direction === 'up' ? stepIndex === 0 : stepIndex === indexes.length - 1; controls.append(button);
          }
          const remove = document.createElement('button'); remove.type = 'button'; remove.dataset.actionClick = 'removeRuntimeProfileArgument'; remove.dataset.args = JSON.stringify([profile.id, String(argumentIndex)]); remove.className = 'backup-action'; remove.textContent = '×'; remove.setAttribute('aria-label', `Удалить шаг ${stepIndex + 1}`); remove.dataset.actionTooltip = 'Удалить этот шаг desync'; controls.append(remove);
          row.append(grip, badge, content, controls); section.append(row);
        });
        const addStrategy = document.createElement('button'); addStrategy.type = 'button'; addStrategy.className = 'profile-builder-add-select w-full rounded-lg border border-cyan-500/40 bg-cyan-950/30 px-3 py-2.5 text-left text-sm font-semibold text-cyan-100 transition hover:border-cyan-300 hover:bg-cyan-900/40'; addStrategy.textContent = '＋ Добавить стратегию';
        addStrategy.setAttribute('aria-haspopup', 'dialog');
        addStrategy.addEventListener('click', () => {
          const choices = runtimeProfileLuaCatalog.length
            ? runtimeProfileLuaCatalog.map(({ name }) => ({ group: 'Стратегии', value: `--lua-desync=${name}`, label: `${name} · ${getRussianLuaFunctionDescription(name)}` }))
            : [{ group: 'Стратегии', value: '--lua-desync=fake', label: `fake · ${getRussianLuaFunctionDescription('fake')}` }];
          openProfileChoiceDialog(choices, 'Выберите стратегию', appendRuntimeProfileArgument);
        });
        const target = runtimeProfileDraftTarget(profile.id);
        const helper = document.createElement('p'); helper.className = 'text-xs leading-5 text-slate-400';
        helper.textContent = profile.editable
          ? 'Выберите действие из списка. Оно добавится только в этот профиль NFQWS_ARGS_CUSTOM; параметры можно настроить после добавления.'
          : target?.source === 'NFQWS_ARGS_QUIC'
            ? 'Выберите действие. NFQWS_ARGS_QUIC общий для профилей «QUIC по IP» и «QUIC по сайтам»; параметры можно настроить после добавления.'
            : target?.source === 'NFQWS_ARGS'
              ? 'Выберите действие. NFQWS_ARGS общий для профилей «HTTP/HTTPS по IP» и «HTTP/HTTPS по сайтам»; параметры можно настроить после добавления.'
              : 'Выберите действие из списка. Оно добавится в настройки этого профиля; параметры можно настроить после добавления.';
        section.append(addStrategy, helper); builder.append(section); continue;
      }
      const row = document.createElement('div'); row.className = `profile-builder-argument-group profile-builder-argument-group-${groupTone} flex flex-wrap items-center gap-1.5 rounded-lg border border-dashed border-slate-700 bg-slate-950/40 p-2`; row.dataset.profileGroup = groupTone;
      const title = document.createElement('span'); title.className = 'mr-1 text-xs font-semibold text-slate-300'; title.textContent = groupName; row.append(title);
      for (const index of indexes) {
        const argument = profile.args[index]!;
        if (!profile.hasConstructor) { const chip = document.createElement('span'); chip.className = 'max-w-full break-all rounded-md bg-slate-800 px-2 py-1 font-mono text-xs text-blue-200'; chip.textContent = argument; row.append(chip); continue; }
        if (argument.startsWith('--filter-tcp=') || argument.startsWith('--filter-udp=') || argument.startsWith('--filter-l7=') || argument.startsWith('--filter-7=')) {
          const filterType = argument.startsWith('--filter-tcp=') ? 'tcp' : argument.startsWith('--filter-udp=') ? 'udp' : 'protocol';
          row.classList.add('profile-builder-filter-group'); title.classList.remove('mr-1');
          const filterEditor = document.createElement('div'); filterEditor.className = `profile-filter-editor profile-filter-editor-${filterType} flex min-w-0 flex-wrap items-center gap-1.5`;
          const filterValues = profileFilterValues(argument);
          filterValues.forEach((value, valueIndex) => {
            const chip = document.createElement('span'); chip.className = 'profile-filter-token inline-flex items-center gap-1 rounded-full border px-2 py-1';
            const token = document.createElement('input'); token.type = 'text'; token.value = value; token.size = Math.min(18, Math.max(3, value.length)); token.className = 'min-w-3 bg-transparent font-mono text-xs focus:outline-none'; token.setAttribute('aria-label', `${groupName}: значение ${value}`);
            token.addEventListener('change', () => updateRuntimeProfileFilterValue(profile.id, String(index), String(valueIndex), token.value));
            const removeValue = document.createElement('button'); removeValue.type = 'button'; removeValue.className = 'profile-filter-remove'; removeValue.textContent = '×'; removeValue.setAttribute('aria-label', `Убрать ${value}`); removeValue.addEventListener('click', () => removeRuntimeProfileFilterValue(profile.id, String(index), String(valueIndex)));
            chip.append(token, removeValue); filterEditor.append(chip);
          });
          if (filterType === 'protocol') {
            const addProtocol = document.createElement('select'); addProtocol.className = 'profile-protocol-select min-h-8 min-w-36 max-w-56 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-200'; addProtocol.setAttribute('aria-label', 'Добавить протокол'); addProtocol.dataset.menuMinWidth = '220';
            const choices = profileProtocolChoices(runtimeProfileCliCatalog?.l7 ?? [], profile.args);
            const placeholder = document.createElement('option'); placeholder.value = ''; placeholder.textContent = choices.length ? '+ протокол' : 'Все протоколы выбраны'; addProtocol.append(placeholder);
            for (const protocol of choices) { const option = document.createElement('option'); option.value = protocol; option.textContent = protocol; addProtocol.append(option); }
            addProtocol.disabled = choices.length === 0;
            addProtocol.addEventListener('change', () => {
              if (!addProtocol.value) return;
              if (!filterValues.length) updateRuntimeProfileArgument(profile.id, String(index), `${argument.slice(0, argument.indexOf('=') + 1)}${addProtocol.value}`);
              else appendRuntimeProfileFilterValue(profile.id, String(index), addProtocol.value);
            });
            filterEditor.append(addProtocol);
          } else {
            const addValue = document.createElement('input'); addValue.type = 'text'; addValue.className = 'profile-filter-add-input min-w-24 rounded-md border bg-slate-950 px-2 py-1 font-mono text-xs'; addValue.placeholder = '+ порт / диапазон'; addValue.setAttribute('aria-label', `Добавить значение: ${groupName}`);
            const commitValue = () => { const value = addValue.value.trim(); if (!value) return; if (appendRuntimeProfileFilterValue(profile.id, String(index), value)) addValue.value = ''; };
            addValue.addEventListener('keydown', (event) => { if (event.key === 'Enter') { event.preventDefault(); commitValue(); } });
            const addValueButton = document.createElement('button'); addValueButton.type = 'button'; addValueButton.className = 'profile-filter-add'; addValueButton.textContent = '+'; addValueButton.setAttribute('aria-label', `Добавить значение в ${groupName}`); addValueButton.addEventListener('click', commitValue);
            filterEditor.append(addValue, addValueButton);
          }
          const removeArgument = document.createElement('button'); removeArgument.type = 'button'; removeArgument.className = 'profile-filter-remove-argument'; removeArgument.textContent = '×'; removeArgument.setAttribute('aria-label', `Удалить условие ${groupName}`); removeArgument.addEventListener('click', () => removeRuntimeProfileArgument(profile.id, String(index)));
          row.append(filterEditor, removeArgument); continue;
        }
        const field = document.createElement('div'); field.className = 'flex min-w-0 items-center gap-1 rounded-md bg-slate-800 p-1';
        const input = document.createElement('input'); input.type = 'text'; input.className = 'min-w-0 max-w-full bg-transparent px-1 font-mono text-xs text-blue-200 focus:outline-none'; input.value = argument; input.setAttribute('aria-label', `Параметр ${groupName}: ${argument}`); input.dataset.actionChange = 'updateRuntimeProfileArgument'; input.dataset.args = JSON.stringify([profile.id, String(index), { source: 'value' }]); input.size = Math.min(48, Math.max(16, argument.length));
        const remove = document.createElement('button'); remove.type = 'button'; remove.dataset.actionClick = 'removeRuntimeProfileArgument'; remove.dataset.args = JSON.stringify([profile.id, String(index)]); remove.className = 'rounded px-1 text-slate-400 hover:bg-slate-700 hover:text-white'; remove.textContent = '×'; remove.setAttribute('aria-label', `Удалить параметр ${argument}`);
        field.append(input, remove); row.append(field);
      }
      builder.append(row);
    }
    if (!profile.args.length) { const empty = document.createElement('p'); empty.className = 'rounded-lg border border-dashed border-slate-700 p-3 text-xs text-slate-400'; empty.textContent = 'Профиль пока пустой. Начните с кнопки «Добавить стратегию» или задайте фильтр, для какого трафика она сработает.'; builder.append(empty); }
    const addLabel = document.createElement('div'); addLabel.className = 'mt-1 grid gap-1 text-xs text-slate-400';
    const addTitle = document.createElement('span'); addTitle.textContent = 'Добавить фильтр или параметр';
    const argumentChoices = profileArgumentChoices(profile.args, profileListInventory, runtimeProfileCliCatalog)
      .filter((choice) => {
        if (profile.editable || !choice.group.startsWith('Для кого ·')) return true;
        return getRuntimeProfileListTarget(profile, choice.value)?.editable === true;
      })
      .filter((choice) => !choice.value.startsWith('--lua-desync='));
    const groupedChoices = new Map<string, Array<{ group: string; value: string; label: string }>>();
    for (const choice of argumentChoices) groupedChoices.set(choice.group, [...(groupedChoices.get(choice.group) ?? []), choice]);
    const addChoice = document.createElement('button'); addChoice.type = 'button'; addChoice.className = 'profile-builder-add-select mt-1 w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-left text-sm font-semibold text-slate-100 hover:border-blue-400 hover:bg-slate-800'; addChoice.textContent = '＋ Выбрать фильтр или параметр';
    addChoice.setAttribute('aria-haspopup', 'dialog');
    addChoice.addEventListener('click', () => openProfileChoiceDialog([...groupedChoices.values()].flat(), 'Добавить фильтр или параметр', appendRuntimeProfileArgument));
    addLabel.append(addTitle, addChoice); builder.append(addLabel); editor.append(builder);
  }
  if (!profile.editable) {
    const components = profile.components ?? [];
    const editorComponents = runtimeProfileEditorMode === 'text' || !profile.hasConstructor ? components : components.slice(1);
    if (editorComponents.length) {
      const intro = document.createElement('p'); intro.className = 'mt-3 text-xs leading-5 text-slate-400'; intro.textContent = runtimeProfileEditorMode === 'text' ? 'Текстовый режим показывает исходные переменные профиля. Общие переменные могут менять сразу несколько профилей.' : 'Связанные списки и дополнительные настройки. Они могут использоваться несколькими профилями.'; editor.append(intro);
      const componentEditors = document.createElement('div'); componentEditors.className = 'grid gap-3';
      for (const component of editorComponents) {
        const componentArgs = runtimeProfileManagedDraft[component.source] ?? component.args;
        const splitArgs = getRuntimeProfileComponentArgs(componentArgs);
        const canEditListComponent = runtimeProfileEditorMode === 'builder'
          && component.editable
          && ['NFQWS_ARGS_IPSET', 'NFQWS_EXTRA_ARGS'].includes(component.source)
          && splitArgs.other.length === 0;
        if (canEditListComponent) {
          const section = document.createElement('section'); section.className = 'grid gap-2 rounded-lg border border-slate-800 bg-slate-950/40 p-3';
          const title = document.createElement('h4'); title.className = 'text-xs font-semibold text-slate-200';
          title.textContent = component.source === 'NFQWS_ARGS_IPSET' ? 'IP-списки' : 'Списки сайтов'; section.append(title);
          if (splitArgs.lists.length) {
            splitArgs.lists.forEach((argument, index) => {
              const row = document.createElement('div'); row.className = 'flex min-w-0 items-center gap-2';
              const input = document.createElement('input'); input.type = 'text'; input.value = argument; input.className = 'min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 font-mono text-xs text-blue-200'; input.setAttribute('aria-label', `${title.textContent}: ${argument}`);
              input.addEventListener('change', () => {
                const updated = input.value.trim();
                if (!updated || /\s/.test(updated)) { input.value = argument; toast('Одна строка списка должна быть одним аргументом nfqws2.', 'error'); return; }
                const next = [...(runtimeProfileManagedDraft[component.source] ?? component.args)];
                const currentIndex = next.indexOf(argument, index);
                if (currentIndex >= 0) next[currentIndex] = updated;
                runtimeProfileManagedDraft[component.source] = next; markRuntimeProfilesDirty(profile.id);
              });
              const remove = document.createElement('button'); remove.type = 'button'; remove.className = 'backup-action'; remove.textContent = 'Удалить'; remove.setAttribute('aria-label', `Удалить список ${argument}`);
              remove.addEventListener('click', () => {
                const next = [...(runtimeProfileManagedDraft[component.source] ?? component.args)];
                const currentIndex = next.indexOf(argument, index);
                if (currentIndex >= 0) next.splice(currentIndex, 1);
                runtimeProfileManagedDraft[component.source] = next; markRuntimeProfilesDirty(profile.id);
              });
              row.append(input, remove); section.append(row);
            });
          } else {
            const empty = document.createElement('p'); empty.className = 'text-xs leading-5 text-slate-400'; empty.textContent = 'Списки пока не добавлены. Используйте «Выбрать параметр» ниже, чтобы подключить список.'; section.append(empty);
          }
          componentEditors.append(section);
        } else {
          const label = document.createElement('label'); label.className = 'mt-3 block text-xs font-semibold text-slate-300'; label.textContent = component.source;
          const textarea = document.createElement('textarea'); textarea.rows = Math.max(3, Math.min(10, componentArgs.length + 1)); textarea.spellcheck = false;
          textarea.className = 'mt-1.5 w-full resize-y rounded-lg border border-slate-700 bg-slate-950 p-3 font-mono text-xs leading-5 text-slate-100 focus:border-blue-500 focus:outline-none';
          textarea.readOnly = !component.editable;
          if (!component.editable) textarea.classList.add('text-slate-400');
          textarea.value = componentArgs.join('\n');
          textarea.setAttribute('aria-label', `Настройка ${component.source}`);
          if (component.editable) textarea.addEventListener('input', () => {
            runtimeProfileManagedDraft[component.source] = textarea.value.split(/\s+/).map((arg) => arg.trim()).filter(Boolean);
            runtimeProfileManagedDirty = true;
            const status = $('#profileStatus'); if (status) status.textContent = `Изменена ${component.source}. Применение проверит конфигурацию и перезапустит nfqws2 с откатом при ошибке.`;
          });
          label.append(textarea); componentEditors.append(label);
        }
        const sharedNote = getRuntimeProfileSharedComponentNote(component.source);
        if (component.note || sharedNote) { const note = document.createElement('p'); note.className = 'mt-1 text-xs leading-5 text-slate-400'; note.textContent = [component.note, sharedNote].filter(Boolean).join(' '); componentEditors.append(note); }
      }
      if (runtimeProfileEditorMode === 'builder' && editorComponents.length) {
        const details = document.createElement('details'); details.className = 'mt-3 rounded-lg border border-slate-800 px-3 py-2';
        const summary = document.createElement('summary'); summary.className = 'cursor-pointer text-sm font-semibold text-slate-300'; summary.textContent = 'Связанные списки и дополнительные настройки';
        details.open = true;
        details.append(summary, componentEditors); editor.append(details);
      } else editor.append(componentEditors);
    }
  } else if (runtimeProfileEditorMode === 'text') {
    const label = document.createElement('label'); label.className = 'mt-3 block text-xs font-semibold text-slate-300'; label.textContent = profile.editable ? 'Аргументы профиля · по одному на строку' : 'Аргументы штатной группы';
    const textEditor = document.createElement('div'); textEditor.id = 'runtimeProfileArgs'; textEditor.className = 'config-argument-code-editor mt-1.5 w-full resize-y overflow-auto rounded-lg border border-slate-700 bg-[#05080f]'; textEditor.style.height = `${Math.max(5, Math.min(12, profile.args.length + 2)) * 1.25}rem`;
    runtimeProfileTextCodeEditor = mountConfigArgumentEditor(textEditor, profile.args.join('\n'), (value) => storeRuntimeProfileArgs(profile.id, value.split(/\s+/).map((arg) => arg.trim()).filter(Boolean), false), 'Аргументы профиля nfqws2', 'по одному аргументу на строку');
    label.append(textEditor); editor.append(label);
  }
  if (profile.editable) {
    const tip = document.createElement('p'); tip.className = 'mt-2 text-xs leading-5 text-slate-500'; tip.textContent = '--new нельзя вводить в редактор отдельного профиля. Для named-профиля используйте --name=имя. Сохранение создаст резервную копию и перезапустит nfqws2 с проверкой и откатом при ошибке.'; editor.append(tip);
  }
  const headingElement = editor.firstElementChild;
  const bodyNodes = Array.from(editor.childNodes).slice(1);
  const main = document.createElement('div'); main.className = 'profile-editor-main'; main.append(...bodyNodes);
  if (headingElement) editor.replaceChildren(headingElement);
  editor.append(main);
  installStyledSelects(editor);
}
async function fetchRuntimeProfileCatalog(forceRefresh = false) {
  const catalog = await nfqwsProfiles.catalog(forceRefresh);
  runtimeProfileLuaCatalog = catalog.functions;
  runtimeProfileLuaStandardGroups = catalog.standardGroups ?? [];
  runtimeProfileCliCatalog = catalog.cli;
}
function renderRuntimeProfileCatalogStatus() {
  const label = $('#profileStatus');
  if (!label || !runtimeProfilesState) return;
  if (runtimeProfilesDirty || runtimeProfileManagedDirty) {
    label.textContent = 'Есть несохранённые изменения. Применение сохранит снимок, проверит конфиг и перезапустит nfqws2.';
    return;
  }
  const cliStatus = runtimeProfileCliCatalog
    ? runtimeProfileCliCatalog.full
      ? `справка nfqws2 полная · ${runtimeProfileCliCatalog.options.length} параметров, ${runtimeProfileCliCatalog.l7.length} протоколов, ${runtimeProfileCliCatalog.payloads.length} типов пакетов`
      : `справка nfqws2 неполная · найдено ${runtimeProfileCliCatalog.options.length} параметров`
    : 'справка параметров nfqws2 недоступна';
  label.replaceChildren(document.createTextNode(`${visibleRuntimeProfiles().length} профилей nfqws2 · пользовательских ${runtimeProfileDraft.length}; штатные UDP, HTTP/HTTPS и QUIC доступны для настройки · ${cliStatus}`));
  if (!runtimeProfileCliCatalog?.full) {
    const retry = document.createElement('button'); retry.type = 'button'; retry.className = 'ml-1 inline-flex min-h-7 items-center rounded-md border border-slate-700 px-2 text-xs font-medium text-cyan-200 hover:border-cyan-400 hover:bg-cyan-950/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400'; retry.textContent = '⟳ Повторить'; retry.setAttribute('aria-label', 'Повторить загрузку справки nfqws2'); retry.title = 'Повторно запросить каталоги параметров nfqws2';
    retry.addEventListener('click', () => void refreshRuntimeProfileCatalog(true));
    label.append(' ', retry);
  }
}
async function refreshRuntimeProfileCatalog(forceRefresh = false) {
  const label = $('#profileStatus');
  if (label) label.textContent = 'Обновляем каталоги параметров nfqws2…';
  try {
    await fetchRuntimeProfileCatalog(forceRefresh);
    renderRuntimeProfiles();
    renderRuntimeProfileCatalogStatus();
  } catch (error) {
    if (label) {
      label.textContent = error instanceof Error ? `Не удалось обновить каталог: ${error.message}` : 'Не удалось обновить каталог параметров nfqws2.';
      const retry = document.createElement('button'); retry.type = 'button'; retry.className = 'ml-1 inline-flex min-h-7 items-center rounded-md border border-slate-700 px-2 text-xs font-medium text-cyan-200 hover:border-cyan-400 hover:bg-cyan-950/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400'; retry.textContent = '⟳ Повторить'; retry.setAttribute('aria-label', 'Повторить загрузку справки nfqws2');
      retry.addEventListener('click', () => void refreshRuntimeProfileCatalog(true));
      label.append(' ', retry);
    }
  }
}
async function loadProfiles() {
  const label = $('#profileStatus'); if (label) label.textContent = 'Загружаем профильные группы nfqws2…';
  try {
    const result = await nfqwsProfiles.list(); runtimeProfilesState = result;
    try { await fetchRuntimeProfileCatalog(); } catch { runtimeProfileLuaCatalog = []; runtimeProfileLuaStandardGroups = []; runtimeProfileCliCatalog = undefined; }
    try { profileListInventory = (await nfqwsLists.index()).files; } catch { profileListInventory = []; }
    runtimeProfileDraft = result.profiles.filter((profile) => profile.editable).map((profile) => [...profile.args]);
    runtimeProfileManagedDraft = {}; runtimeProfileManagedDirty = false;
    runtimeProfilesDirty = false; renderRuntimeProfiles();
    renderRuntimeProfileCatalogStatus();
  } catch (error) { if (label) label.textContent = error instanceof Error ? error.message : 'Не удалось загрузить профили nfqws2.'; }
}
function addRuntimeProfile() {
  runtimeProfileDraft.push(['--filter-tcp=443', '--payload=tls_client_hello', '--lua-desync=fake']);
  selectedRuntimeProfileId = `custom-${runtimeProfileDraft.length}`;
  markRuntimeProfilesDirty();
}
function copyRuntimeProfile(id: string) {
  const source = visibleRuntimeProfiles().find((profile) => profile.id === id);
  if (!source) return toast('Профиль для копирования не найден. Обновите список профилей.', 'error');
  const result = copyProfileIntoDraft(runtimeProfileDraft, source.effectiveArgs ?? source.args);
  if (!result.ok) return toast(result.reason === 'profile-limit' ? 'Нельзя создать больше 64 пользовательских профилей.' : 'Копия превысит ограничение в 8192 аргумента.', 'error');
  runtimeProfileDraft = result.profiles;
  selectedRuntimeProfileId = `custom-${runtimeProfileDraft.length}`;
  markRuntimeProfilesDirty();
  toast(`Создана копия профиля «${source.name}». Её можно изменить перед применением.`);
}
async function shareRuntimeProfile(id: string) {
  const profile = visibleRuntimeProfiles().find((item) => item.id === id);
  if (!profile) return toast('Профиль не найден. Обновите список профилей.', 'error');
  const text = serializeProfileForSharing(profile.name, profile.effectiveArgs ?? profile.args, status?.service.version ?? null);
  try { await writeTextToClipboard(text); toast('Профиль скопирован для чата. Если в нём есть списки, получателю может понадобиться заменить их пути.'); }
  catch { toast('Не удалось скопировать профиль. Проверьте доступ к буферу обмена.', 'error'); }
}
function pasteRuntimeProfile() {
  const modal = $('#profileImportDialog'); const input = $<HTMLTextAreaElement>('#profileImportInput');
  if (input) input.value = '';
  if (modal) openAppModal(modal, '#profileImportInput');
}
function cancelProfileImport() {
  closeAppModal($('#profileImportDialog'));
}
function acceptProfileImport() {
  const raw = $<HTMLTextAreaElement>('#profileImportInput')?.value ?? '';
  const parsed = parseProfilePaste(raw);
  if (!parsed.ok) {
    const message = parsed.reason === 'shell-syntax' ? 'В тексте есть shell-команда, переменная или оператор. Обычные кавычки вокруг конфигурации разрешены.'
      : parsed.reason === 'unclosed-quote' ? 'В тексте осталась незакрытая кавычка.'
        : parsed.reason === 'profile-limit' ? 'Вставленный текст содержит больше 64 профилей.'
          : parsed.reason === 'argument-limit' ? 'Вставленный текст превышает лимит размера или числа аргументов.'
            : 'Не удалось найти параметры nfqws2 в тексте.';
    return toast(message, 'error');
  }
  const imported = parsed.profiles;
  if (runtimeProfileDraft.length + imported.length > 64 || runtimeProfileDraft.flat().length + imported.flat().length > 8192) return toast('После добавления превысится лимит 64 пользовательских профиля или 8192 аргумента.', 'error');
  runtimeProfileDraft.push(...imported); selectedRuntimeProfileId = `custom-${runtimeProfileDraft.length}`;
  cancelProfileImport(); markRuntimeProfilesDirty();
}
function selectRuntimeProfile(id: string) { selectedRuntimeProfileId = id; renderRuntimeProfiles(); }
function setRuntimeProfileMode(mode: string) {
  runtimeProfileEditorMode = mode === 'text' ? 'text' : 'builder';
  renderRuntimeProfiles();
}
function appendRuntimeProfileArgument(argument: string) {
  if (!argument) return;
  const profile = runtimeProfilesState?.profiles.find((item) => item.id === selectedRuntimeProfileId);
  if (profile && !profile.editable) {
    const listTarget = getRuntimeProfileListTarget(profile, argument);
    if (listTarget) {
      if (!listTarget.editable) return toast(`${listTarget.source} связан с общим режимом и здесь доступен только для просмотра.`, 'error');
      const current = profile.components?.find((component) => component.source === listTarget.source);
      const args = [...(runtimeProfileManagedDraft[listTarget.source] ?? current?.args ?? [])];
      if (!args.includes(argument)) args.push(argument);
      runtimeProfileManagedDraft[listTarget.source] = args;
      markRuntimeProfilesDirty(profile.id);
      return;
    }
  }
  const target = runtimeProfileDraftTarget(selectedRuntimeProfileId);
  if (!target) return;
  if (!target.managed && target.customIndex !== null) {
    const updated = appendArgumentToCustomProfile(runtimeProfileDraft, target.customIndex, argument);
    if (!updated) return;
    runtimeProfileDraft = updated;
    markRuntimeProfilesDirty(selectedRuntimeProfileId);
    return;
  }
  target.args.push(argument); storeRuntimeProfileArgs(selectedRuntimeProfileId, target.args);
}
function updateRuntimeProfileStep(id: string, argumentIndex: string, update: (argument: string) => string) {
  const position = Number(argumentIndex); const target = runtimeProfileDraftTarget(id); const args = target?.args;
  if (!args || !Number.isInteger(position) || !args[position]?.startsWith('--lua-desync=')) return;
  const updated = update(args[position]!);
  if (updated === args[position]) return;
  args[position] = updated; storeRuntimeProfileArgs(id, args);
}
function createProfileStepGrip() {
  const grip = document.createElement('span'); grip.className = 'profile-step-grip'; grip.textContent = '⋮⋮'; grip.setAttribute('aria-hidden', 'true');
  return grip;
}
function bindProfileStepDrop(row: HTMLElement, profileId: string, targetIndex: number) {
  row.draggable = true;
  let profileStepDragAllowed = false;
  row.addEventListener('pointerdown', (event) => {
    profileStepDragAllowed = !(event.target instanceof Element && event.target.closest('input,select,button,textarea,summary,a,[contenteditable="true"]'));
  });
  row.addEventListener('dragstart', (event) => {
    const target = event.target;
    if (!profileStepDragAllowed || (target instanceof HTMLElement && target.closest('input,select,button,textarea,summary,a,[contenteditable="true"]'))) { event.preventDefault(); return; }
    draggedRuntimeProfileStep = { profileId, index: targetIndex };
    row.classList.add('is-dragging');
    if (event.dataTransfer) { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData('text/plain', `${profileId}:${targetIndex}`); event.dataTransfer.setDragImage(row, Math.min(40, row.clientWidth / 4), Math.min(32, row.clientHeight / 4)); }
  });
  row.addEventListener('dragend', () => {
    draggedRuntimeProfileStep = null;
    document.querySelectorAll('.profile-step-row.is-dragging,.profile-step-row.is-drop-target,.profile-step-row.is-drop-before,.profile-step-row.is-drop-after').forEach((item) => item.classList.remove('is-dragging', 'is-drop-target', 'is-drop-before', 'is-drop-after'));
  });
  row.addEventListener('dragover', (event) => {
    if (draggedRuntimeProfileStep?.profileId !== profileId) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    const before = event.clientY < row.getBoundingClientRect().top + row.getBoundingClientRect().height / 2;
    row.classList.add('is-drop-target'); row.classList.toggle('is-drop-before', before); row.classList.toggle('is-drop-after', !before);
  });
  row.addEventListener('dragleave', (event) => { if (!row.contains(event.relatedTarget as Node | null)) row.classList.remove('is-drop-target', 'is-drop-before', 'is-drop-after'); });
  row.addEventListener('drop', (event) => {
    event.preventDefault();
    if (draggedRuntimeProfileStep?.profileId !== profileId) return;
    const from = draggedRuntimeProfileStep.index;
    const before = event.clientY < row.getBoundingClientRect().top + row.getBoundingClientRect().height / 2;
    const to = before ? targetIndex - (from < targetIndex ? 1 : 0) : targetIndex + (from > targetIndex ? 1 : 0);
    moveRuntimeProfileStepTo(profileId, from, Math.max(0, to)); draggedRuntimeProfileStep = null;
    document.querySelectorAll('.profile-step-row.is-dragging,.profile-step-row.is-drop-target,.profile-step-row.is-drop-before,.profile-step-row.is-drop-after').forEach((item) => item.classList.remove('is-dragging', 'is-drop-target', 'is-drop-before', 'is-drop-after'));
  });
}
function moveRuntimeProfileStepTo(id: string, from: number, to: number) {
  const target = runtimeProfileDraftTarget(id);
  if (!target || !Number.isInteger(from) || !Number.isInteger(to)) return;
  const oldRows = Array.from(document.querySelectorAll<HTMLElement>('.profile-step-row'));
  const oldPositions = oldRows.map((row) => row.getBoundingClientRect().top);
  storeRuntimeProfileArgs(id, moveProfileStepTo(target.args, from, to));
  requestAnimationFrame(() => {
    const newRows = Array.from(document.querySelectorAll<HTMLElement>('.profile-step-row'));
    newRows.forEach((row, newIndex) => {
      const oldIndex = newIndex === to ? from : from < to && newIndex >= from && newIndex < to ? newIndex + 1 : from > to && newIndex > to && newIndex <= from ? newIndex - 1 : newIndex;
      const previousTop = oldPositions[oldIndex];
      if (previousTop === undefined) return;
      const offset = previousTop - row.getBoundingClientRect().top;
      if (offset) row.animate([{ transform: `translateY(${offset}px)` }, { transform: 'translateY(0)' }], { duration: 180, easing: 'cubic-bezier(.2,.7,.2,1)' });
    });
  });
}
function renameRuntimeProfile(id: string, name: string) {
  const target = runtimeProfileDraftTarget(id); const args = target?.args;
  if (!args || target?.managed) return;
  if (name.trim() && !/^[\p{L}\p{N}_.-]{1,32}$/u.test(name.trim())) return toast('Имя: буквы, цифры, точка, дефис или подчёркивание; не больше 32 символов.', 'error');
  storeRuntimeProfileArgs(id, replaceProfileName(args, name));
}
function updateRuntimeProfileArgument(id: string, argumentIndex: string, value: string) {
  const position = Number(argumentIndex); const target = runtimeProfileDraftTarget(id); const args = target?.args;
  if (!args || !Number.isInteger(position) || !args[position]) return;
  const updated = replaceProfileArgument(args, position, value.trim());
  if (updated[position] === args[position] && value.trim() !== args[position]) return toast('Один параметр должен оставаться единым аргументом без shell-синтаксиса.', 'error');
  storeRuntimeProfileArgs(id, updated);
}
function updateRuntimeProfileFilterValue(id: string, argumentIndex: string, valueIndex: string, value: string) {
  if (!value.trim()) return removeRuntimeProfileFilterValue(id, argumentIndex, valueIndex);
  const target = runtimeProfileDraftTarget(id); const position = Number(argumentIndex); const args = target?.args; const current = args?.[position];
  if (!args || !current) return;
  const updated = updateProfileFilterValue(current, Number(valueIndex), value);
  if (!updated) return toast('Значение фильтра должно быть одним элементом без запятых и shell-синтаксиса.', 'error');
  args[position] = updated; storeRuntimeProfileArgs(id, args);
}
function removeRuntimeProfileFilterValue(id: string, argumentIndex: string, valueIndex: string) {
  const target = runtimeProfileDraftTarget(id); const position = Number(argumentIndex); const args = target?.args; const current = args?.[position];
  if (!args || !current) return;
  const updated = removeProfileFilterValue(current, Number(valueIndex));
  if (updated) args[position] = updated;
  else args.splice(position, 1);
  storeRuntimeProfileArgs(id, args);
}
function appendRuntimeProfileFilterValue(id: string, argumentIndex: string, value: string): boolean {
  const target = runtimeProfileDraftTarget(id); const position = Number(argumentIndex); const args = target?.args; const current = args?.[position];
  if (!args || !current) return false;
  const updated = appendProfileFilterValue(current, value);
  if (!updated) { toast('Введите одно значение без запятых и shell-синтаксиса.', 'error'); return false; }
  args[position] = updated; storeRuntimeProfileArgs(id, args); return true;
}
function addRuntimeProfileStepParameter(id: string, argumentIndex: string, parameter: string) {
  const position = Number(argumentIndex); const current = runtimeProfileDraftTarget(id)?.args[position];
  if (!current) return;
  const updated = appendDesyncParameter(current, parameter);
  if (updated === current) return toast('Параметр должен быть одним ключом, значением или флагом без пробелов.', 'error');
  updateRuntimeProfileStep(id, argumentIndex, () => updated);
}
function updateRuntimeProfileStepParameter(id: string, argumentIndex: string, parameterIndex: string, value: string) {
  const position = Number(argumentIndex); const current = runtimeProfileDraftTarget(id)?.args[position];
  if (!current) return;
  const parameter = splitDesyncParameters(current)[Number(parameterIndex)];
  if (!parameter) return;
  const updated = updateDesyncParameter(current, Number(parameterIndex), `${parameter.split('=')[0]}=${value}`);
  if (updated === current && value !== parameter.slice(parameter.indexOf('=') + 1)) return toast('Значение параметра должно быть одним аргументом без пробелов и shell-синтаксиса.', 'error');
  updateRuntimeProfileStep(id, argumentIndex, () => updated);
}
function removeRuntimeProfileStepParameter(id: string, argumentIndex: string, parameterIndex: string) {
  updateRuntimeProfileStep(id, argumentIndex, (argument) => removeDesyncParameter(argument, Number(parameterIndex)));
}
function moveRuntimeProfileStep(id: string, stepIndex: string, direction: string) {
  const step = Number(stepIndex); const target = runtimeProfileDraftTarget(id); const args = target?.args;
  if (!args || !Number.isInteger(step) || (direction !== 'up' && direction !== 'down')) return;
  storeRuntimeProfileArgs(id, reorderProfileSteps(args, step, direction === 'up' ? -1 : 1));
}
function removeRuntimeProfileArgument(id: string, argIndex: string) {
  const position = Number(argIndex); const target = runtimeProfileDraftTarget(id);
  if (!Number.isInteger(position) || !target?.args[position]) return;
  target.args.splice(position, 1); storeRuntimeProfileArgs(id, target.args);
}
function mutateRuntimeProfile(id: string, action: 'up' | 'down' | 'delete') {
  const index = Number(id.slice('custom-'.length)) - 1;
  if (!Number.isInteger(index) || !runtimeProfileDraft[index]) return;
  if (action === 'delete') {
    runtimeProfileDraft.splice(index, 1);
    selectedRuntimeProfileId = runtimeProfileDraft.length ? `custom-${Math.min(index + 1, runtimeProfileDraft.length)}` : '';
  } else {
    const target = index + (action === 'up' ? -1 : 1);
    if (target < 0 || target >= runtimeProfileDraft.length) return;
    [runtimeProfileDraft[index], runtimeProfileDraft[target]] = [runtimeProfileDraft[target]!, runtimeProfileDraft[index]!];
    selectedRuntimeProfileId = `custom-${target + 1}`;
  }
  markRuntimeProfilesDirty();
}
async function saveRuntimeProfiles() {
  if (!runtimeProfilesState || (!runtimeProfilesDirty && !runtimeProfileManagedDirty)) return toast('Нет изменений профилей для применения.');
  requestDangerousConfirmation('Применить изменения профилей?', 'Конфигурация будет сохранена в резервную копию, проверена через nfqws2 --dry-run, затем nfqws2 перезапустится с возможностью отката.', async () => {
    setBusy(true); setRestartInProgress(true);
    try {
      let result: Awaited<ReturnType<typeof nfqwsProfiles.saveCustom>> | undefined;
      if (runtimeProfilesDirty) {
        result = await nfqwsProfiles.saveCustom(runtimeProfileDraft, runtimeProfilesState!.revision);
        config = result.config;
        runtimeProfilesDirty = false;
      }
      if (runtimeProfileManagedDirty) {
        if (runtimeProfilesDirty === false && result) runtimeProfilesState = await nfqwsProfiles.list();
        result = await nfqwsProfiles.saveSettings(runtimeProfileManagedDraft, runtimeProfilesState!.revision);
      }
      if (!result) return;
      config = result.config;
      setNfqwsConfigText(config.content);
      toast(result.message || 'Профили nfqws2 применены.'); await Promise.all([loadProfiles(), fetchStatus()]);
    } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось применить профили.', 'error'); }
    finally { setRestartInProgress(false); setBusy(false); }
  });
}
function requestDangerousConfirmation(title: string, message: string, action: () => Promise<void>, onCancel: () => void = () => undefined) {
  setText('#operationConfirmTitle', title); setText('#operationConfirmMessage', message); pendingConfirmation = action;
  pendingConfirmationCancel = onCancel;
  const modal = $('#operationConfirmModal'); if (modal) openAppModal(modal, '#operationConfirmCancel');
}
function stopUpdatePickerAutoRefresh() {
  if (updatePickerRefreshTimer !== null) window.clearInterval(updatePickerRefreshTimer);
  updatePickerRefreshTimer = null;
}
function closeUpdateModal() { stopUpdatePickerAutoRefresh(); closeAppModal($('#updateModalBackdrop')); }
function closeDangerousConfirmation(confirmed = false) {
  const onCancel = pendingConfirmationCancel; pendingConfirmation = null; pendingConfirmationCancel = null;
  if (!confirmed) onCancel?.();
  closeAppModal($('#operationConfirmModal'));
}
async function confirmDangerousOperation() {
  const action = pendingConfirmation; closeDangerousConfirmation(true); if (action) await action();
}
async function loadChangeHistory(query = '') {
  const host = $('#changeHistoryList'); if (!host) return;
  const status = $('#changeHistoryStatus'); if (status) status.textContent = 'Загружаем историю…';
  try {
    const result = await nfqwsChangeHistory.list();
    changeHistoryCache = result.items;
    changeHistoryCanRestoreInitial = result.canRestoreInitial;
    const initial = $('#changeHistoryRestoreInitial');
    if (initial instanceof HTMLButtonElement) initial.disabled = !result.canRestoreInitial;
    const undo = $('#changeHistoryUndoLast');
    if (undo instanceof HTMLButtonElement) undo.disabled = !result.canUndoLast;
    renderChangeHistory($<HTMLInputElement>('#changeHistorySearch')?.value ?? query);
  } catch (error) { if (status) status.textContent = error instanceof Error ? error.message : 'Не удалось загрузить журнал изменений.'; }
}
function renderChangeHistory(query = '') {
  const host = $('#changeHistoryList'); if (!host) return;
  const needle = query.trim().toLocaleLowerCase();
  const visible = changeHistoryCache.filter((item) => !needle || [item.file, item.source, item.note, item.path].join(' ').toLocaleLowerCase().includes(needle));
  host.replaceChildren();
  if (!visible.length) {
    const empty = document.createElement('p'); empty.className = 'px-2 py-5 text-center text-xs text-slate-400';
    empty.textContent = changeHistoryCache.length ? 'Нет событий по этому фильтру.' : 'Изменений пока нет. Изменения вне панели появятся при следующей проверке журнала.';
    host.append(empty);
  }
  let previousDay = '';
  for (const item of visible) {
    const date = new Date(item.ts * 1000);
    const dayKey = `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
    if (dayKey !== previousDay) {
      const day = document.createElement('h3'); day.className = 'change-history-day';
      day.textContent = date.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' });
      host.append(day); previousDay = dayKey;
    }
    const row = document.createElement('article'); row.className = 'change-history-row';
    const summary = document.createElement('div'); summary.className = 'change-history-summary';
    const time = document.createElement('time'); time.className = 'change-history-time'; time.dateTime = date.toISOString(); time.textContent = date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const details = document.createElement('div');
    const title = document.createElement('strong'); title.className = 'change-history-file'; title.textContent = item.file === 'nfqws2.conf' ? 'Конфигурация' : item.file;
    const meta = document.createElement('p'); meta.className = 'change-history-meta'; meta.textContent = `${item.note || item.source} · ${item.source}`;
    details.append(title, meta);
    const delta = document.createElement('span'); delta.className = 'change-history-delta';
    const added = document.createElement('span'); added.className = 'text-emerald-300'; added.textContent = `+${item.addedLines}`;
    const removed = document.createElement('span'); removed.className = 'text-rose-300'; removed.textContent = ` −${item.removedLines}`;
    delta.append(added, removed, document.createTextNode(' строк'));
    summary.append(time, details, delta);

    const actions = document.createElement('div'); actions.className = 'change-history-actions';
    const diffButton = document.createElement('button'); diffButton.type = 'button'; diffButton.className = 'backup-action'; diffButton.textContent = 'Что изменилось'; diffButton.dataset.actionClick = 'toggleChangeHistoryDiff'; diffButton.dataset.args = JSON.stringify([item.id, { source: 'self' }]);
    const revertButton = document.createElement('button'); revertButton.type = 'button'; revertButton.className = 'backup-action'; revertButton.textContent = 'Откатить'; revertButton.dataset.actionClick = 'requestRevertChangeHistory'; revertButton.dataset.args = JSON.stringify([item.id]);
    actions.append(diffButton, revertButton);
    details.append(actions);
    const diff = document.createElement('pre'); diff.className = 'change-history-diff'; diff.hidden = true; diff.dataset.historyDiff = item.id;
    row.append(summary, diff); host.append(row);
  }
  setText('#changeHistoryStatus', `${visible.length} ${pluralize(visible.length, 'изменение', 'изменения', 'изменений')} · хранится до 500 событий`);
}
function changedHistoryLines(from: string | null, to: string | null) {
  const count = (value: string | null) => {
    const counts = new Map<string, number>();
    for (const line of (value ?? '').split(/\r?\n/).map((line) => line.trim()).filter(Boolean)) counts.set(line, (counts.get(line) ?? 0) + 1);
    return counts;
  };
  const before = count(from); const after = count(to);
  const removed: string[] = []; const added: string[] = [];
  for (const [line, n] of before) for (let index = (after.get(line) ?? 0); index < n; index += 1) removed.push(line);
  for (const [line, n] of after) for (let index = (before.get(line) ?? 0); index < n; index += 1) added.push(line);
  return { removed, added };
}
async function toggleChangeHistoryDiff(id: string, button: HTMLButtonElement) {
  const diff = button.closest('.change-history-row')?.querySelector<HTMLElement>('[data-history-diff]');
  if (!(diff instanceof HTMLElement)) return;
  diff.hidden = !diff.hidden;
  button.setAttribute('aria-expanded', String(!diff.hidden));
  if (diff.hidden || diff.dataset.loaded === 'true') return;
  diff.textContent = 'Загружаем diff…';
  try {
    const result = await nfqwsChangeHistory.diff(id);
    const changes = changedHistoryLines(result.from, result.to);
    diff.replaceChildren();
    for (const [kind, lines] of [['removed', changes.removed], ['added', changes.added]] as const) {
      for (const line of lines.slice(0, 200)) {
        const entry = document.createElement('span'); entry.className = kind; entry.textContent = `${kind === 'added' ? '+ ' : '− '}${line.slice(0, 1200)}${line.length > 1200 ? '…' : ''}\n`; diff.append(entry);
      }
      if (lines.length > 200) { const more = document.createElement('span'); more.textContent = `…ещё ${lines.length - 200} строк`; diff.append(more); }
    }
    if (!diff.childElementCount) diff.textContent = 'Текстовые строки не изменились.';
    if (result.current !== result.to) { const warning = document.createElement('span'); warning.className = 'block border-t border-amber-700/50 pt-2 text-amber-300'; warning.textContent = 'Текущий файл отличается от этой записи. Откат будет отклонён при конфликте.'; diff.append(warning); }
    diff.dataset.loaded = 'true';
  } catch (error) { diff.textContent = error instanceof Error ? error.message : 'Не удалось загрузить diff.'; }
}
function requestRevertChangeHistory(id: string) {
  const item = changeHistoryCache.find((entry) => entry.id === id); if (!item) return;
  requestDangerousConfirmation('Откатить это изменение?', `Файл «${item.file}» вернётся к состоянию до «${item.note || item.source}». Перед изменением будет создан резервный снимок.`, async () => {
    try { const result = await nfqwsChangeHistory.revert(id); toast(result.message); await refreshAfterHistoryRestore(); }
    catch (error) { toast(error instanceof Error ? error.message : 'Не удалось откатить изменение.', 'error'); await loadChangeHistory(); }
  });
}
function requestUndoLastChange() {
  requestDangerousConfirmation('Отменить последнее изменение?', 'Будет отменена последняя запись, которую ещё можно отменить. Перед восстановлением создастся резервный снимок.', async () => {
    try { const result = await nfqwsChangeHistory.undoLast(); toast(result.message); await refreshAfterHistoryRestore(); }
    catch (error) { toast(error instanceof Error ? error.message : 'Не удалось отменить последнее изменение.', 'error'); await loadChangeHistory(); }
  });
}
function requestRestoreInitialHistory() {
  if (!changeHistoryCanRestoreInitial) return;
  requestDangerousConfirmation('Вернуть состояние на начало истории?', 'Конфигурация и исходные списки вернутся к моменту, когда журнал впервые зафиксировал исходное состояние. Списки, созданные позже, сохранятся. Перед восстановлением создастся полный резервный снимок.', async () => {
    try { const result = await nfqwsChangeHistory.restoreInitial(); toast(result.message); await refreshAfterHistoryRestore(); }
    catch (error) { toast(error instanceof Error ? error.message : 'Не удалось вернуть исходное состояние.', 'error'); }
  });
}
async function refreshAfterHistoryRestore() {
  await Promise.all([loadChangeHistory(), loadConfig(), fetchStatus(), loadBackups()]);
  if (activeListId) await loadList(activeListId);
}
async function loadLogs() {
  if (logRequestPending) return;
  logRequestPending = true;
  const generation = ++logRequestGeneration;
  const source = currentLogSource;
  const status = $('#logsStatus'); if (status) status.textContent = `Загружаем журнал ${source}…`;
  try {
    const result = await nfqwsLogs.get(source);
    if (generation !== logRequestGeneration || source !== currentLogSource) return;
    const output = $('#logsOutput');
    const followTail = output instanceof HTMLPreElement && shouldFollowLogTail(Boolean(output.textContent), output.scrollTop, output.clientHeight, output.scrollHeight);
    currentLogContent = result.content; renderLogs(followTail);
    if (status) status.textContent = `Получен хвост журнала ${source} · ${result.content.split(/\r?\n/).filter(Boolean).length} строк.`;
  } catch (error) {
    if (generation !== logRequestGeneration || source !== currentLogSource) return;
    currentLogContent = error instanceof Error ? error.message : 'Не удалось загрузить журнал.'; renderLogs();
    if (status) status.textContent = 'Источник журнала недоступен.';
  } finally {
    if (generation === logRequestGeneration) logRequestPending = false;
  }
}
function renderLogs(followTail = false) {
  const output = $('#logsOutput'); if (!(output instanceof HTMLPreElement)) return;
  const query = ($<HTMLInputElement>('#logsSearch')?.value ?? '').trim().toLocaleLowerCase();
  output.textContent = currentLogContent.split(/\r?\n/).filter((line) => !query || line.toLocaleLowerCase().includes(query)).join('\n') || 'Совпадений нет.';
  if (followTail) output.scrollTop = output.scrollHeight;
}
function startLogPolling() {
  stopLogPolling();
  logPollingTimer = window.setInterval(() => {
    const sectionVisible = document.visibilityState === 'visible' && !document.getElementById('tabContent-logs')?.classList.contains('hidden');
    const autoRefresh = $<HTMLInputElement>('#logsAutoRefresh')?.checked ?? true;
    if (shouldPollLogs(sectionVisible, autoRefresh, logRequestPending)) void loadLogs();
  }, 5000);
}
function stopLogPolling() { if (logPollingTimer !== null) window.clearInterval(logPollingTimer); logPollingTimer = null; }
function downloadLogs() {
  const url = URL.createObjectURL(new Blob([currentLogContent], { type: 'text/plain;charset=utf-8' }));
  const link = document.createElement('a'); link.href = url; link.download = `${currentLogSource}.log`; link.click(); window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}
function clearLogs() {
  const sourceName = currentLogSource === 'nfqws2-ui' ? 'nfqws2-UI' : 'nfqws2';
  requestDangerousConfirmation(`Очистить журнал ${sourceName}?`, 'Файл журнала будет обнулён. Служба продолжит писать новые записи; конфигурация и работа nfqws2 не изменятся.', async () => {
    try { await nfqwsLogs.clear(currentLogSource); toast(`Журнал ${sourceName} очищен.`); await loadLogs(); }
    catch (error) { toast(error instanceof Error ? error.message : `Не удалось очистить журнал ${sourceName}.`, 'error'); }
  });
}
function syncAppearanceControls() {
  all<HTMLInputElement>('input[name="appearanceTheme"]').forEach((input) => { input.checked = input.value === appearanceSettings.theme; });
  all<HTMLInputElement>('input[name="appearanceLayout"]').forEach((input) => { input.checked = input.value === appearanceSettings.layout; });
  all<HTMLInputElement>('input[name="appearanceAccent"]').forEach((input) => { input.checked = input.value === appearanceSettings.accent; });
  all<HTMLInputElement>('input[name="appearanceStatusBar"]').forEach((input) => { input.checked = input.value === appearanceSettings.statusBar; });
  all<HTMLInputElement>('[data-appearance-remind-timestamps]').forEach((input) => { input.checked = appearanceSettings.remindWindowsTcpTimestamps; });
  all<HTMLInputElement>('[data-appearance-config-warnings]').forEach((input) => { input.checked = appearanceSettings.showConfigWarnings; });
  syncConfigWordWrap(appearanceSettings.wrapConfigLines !== false);
  syncConfigWarningsVisibility();
}
function syncConfigWordWrap(enabled: boolean) {
  const toggle = $('#configWordWrapToggle');
  if (toggle instanceof HTMLInputElement) toggle.checked = enabled;
  setConfigCodeEditorWrapping(enabled);
}
function setNfqwsConfigText(value: string) {
  const textarea = $('#nfqwsConfTextarea');
  if (!(textarea instanceof HTMLTextAreaElement)) return;
  if (getConfigCodeEditor()) setConfigCodeEditorContent(value);
  else textarea.value = value;
}
function selectConfigText(start: number, end: number, line?: number) {
  const view = getConfigCodeEditor();
  if (!view) return;
  const position = Math.max(0, Math.min(start, view.state.doc.length));
  view.dispatch({
    selection: { anchor: position, head: Math.max(position, Math.min(end, view.state.doc.length)) },
    effects: EditorView.scrollIntoView(position, { y: line ? 'center' : 'nearest' }),
  });
  view.focus();
}
function routerAppearanceSettings(settings: AppearanceSettings) {
  const { wrapConfigLines: _wrapConfigLines, ...routerSettings } = settings;
  return routerSettings;
}
function syncNavigationLayout(route?: Route) {
  const activeRoute = route ?? (all<HTMLElement>('[id^="tabContent-"]').find((section) => !section.classList.contains('hidden'))?.id.replace('tabContent-', '') as Route | undefined);
  const group = navigationGroupForRoute(activeRoute);
  const navigation = $('#simplifiedNavigation');
  navigation?.classList.toggle('hidden', appearanceSettings.layout !== 'simplified');
  all<HTMLButtonElement>('[data-simplified-group]').forEach((button) => {
    const active = button.dataset.simplifiedGroup === group;
    button.setAttribute('aria-pressed', String(active));
    button.classList.toggle('simplified-nav-active', active);
  });
  all<HTMLButtonElement>('[data-simplified-route]').forEach((button) => {
    const active = button.dataset.simplifiedRoute === activeRoute;
    button.setAttribute('aria-pressed', String(active));
    button.classList.toggle('simplified-nav-active', active);
    button.classList.toggle('hidden', navigationGroupForRoute(button.dataset.simplifiedRoute) !== group);
  });
}
function updateAppearance(change: Partial<AppearanceSettings>) {
  let storage: Storage | null = null; try { storage = window.localStorage; } catch { /* Apply for this session without persistent storage. */ }
  appearanceSettings = saveAppearance(storage, { ...appearanceSettings, ...change });
  applyAppearance(appearanceSettings);
  syncAppearanceControls();
  syncNavigationLayout();
  void nfqwsAppearance.save(routerAppearanceSettings(appearanceSettings)).catch((error) => toast(error instanceof Error ? `Оформление сохранено только в этом браузере: ${error.message}` : 'Оформление сохранено только в этом браузере.', 'error'));
  if (change.showConfigWarnings === true) scheduleConfigWarnings(0);
}
async function syncAppearanceFromRouter() {
  const saved = await nfqwsAppearance.get(); if (!saved) return;
  let storage: Storage | null = null; try { storage = window.localStorage; } catch { /* Use the restored settings for this session. */ }
  appearanceSettings = saveAppearance(storage, { ...saved, wrapConfigLines: appearanceSettings.wrapConfigLines }); applyAppearance(appearanceSettings); syncAppearanceControls(); syncNavigationLayout();
}
function initializeAppearanceSettings() {
  let storage: Storage | null = null;
  try { storage = window.localStorage; } catch { /* Use safe defaults if browser storage is blocked. */ }
  appearanceSettings = loadAppearance(storage);
  applyAppearance(appearanceSettings);
  syncAppearanceControls();
  syncNavigationLayout();
  void nfqwsAppearance.get().then((saved) => {
    if (!saved) { void nfqwsAppearance.save(routerAppearanceSettings(appearanceSettings)).catch(() => undefined); return; }
    appearanceSettings = saveAppearance(storage, { ...saved, wrapConfigLines: appearanceSettings.wrapConfigLines }); applyAppearance(appearanceSettings); syncAppearanceControls(); syncNavigationLayout();
  }).catch(() => undefined);
}
async function loadAboutStatus(statusOverride?: UiUpdateStatus) {
  try {
    const result = statusOverride ?? await nfqwsUpdates.uiStatus();
    setText('#aboutCurrentVersion', result.currentVersion);
    setText('#aboutLatestVersion', result.latestVersion ?? 'не удалось определить');
    const versionCommand = $('[data-about-version-command]');
    const version = result.latestVersion ?? result.currentVersion;
    if (versionCommand) {
      const tag = version.startsWith('v') ? version : `v${version}`;
      if (/^v(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$/.test(tag)) {
        versionCommand.textContent = `nfqws2-ui-ctl update ${tag} --force`;
        const title = $('[data-about-version-command-title]');
        if (title) title.textContent = 'Установить эту версию';
        const description = $('[data-about-version-command-description]');
        if (description) description.textContent = 'Переустановить эту версию или откатиться на неё.';
      }
    }
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
    const check = document.createElement('button'); check.type = 'button'; check.className = 'monitor-row-action'; check.dataset.actionClick = 'checkMonitoringSite'; check.dataset.args = JSON.stringify([site.host]); check.textContent = monitoringHostsRunning.has(site.host) ? 'Проверяем…' : '⟳ Проверить'; check.disabled = monitoringRunning || monitoringHostsRunning.has(site.host); check.title = `Проверить ${site.host}`;
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
async function persistMonitoring() {
  if (!monitoringState) return false;
  saveMonitoringState(getMonitoringStorage(), monitoringState); renderMonitoringSites();
  const enabled = ($('#monitoringEnabled') as HTMLInputElement | null)?.checked ?? true;
  monitoringIntervalMinutes = currentMonitoringInterval();
  try {
    await nfqwsMonitoring.save({ enabled, intervalMinutes: monitoringIntervalMinutes, sites: monitoringState.sites.map((site) => ({ host: site.host, results: site.results })) });
    return true;
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось сохранить мониторинг на роутере.', 'error');
    return false;
  }
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
  if (hosts.some((item) => monitoringHostsRunning.has(item))) return;
  monitoringRunning = true;
  const button = $('[data-action-click="runMonitoring"]'); if (button instanceof HTMLButtonElement) { button.disabled = true; button.textContent = 'Проверяем…'; }
  hosts.forEach((item) => monitoringHostsRunning.add(item)); renderMonitoringSites();
  const statusNode = $('#monitoringStatus'); if (statusNode) { statusNode.textContent = `Проверяются ${hosts.length} ${hosts.length === 1 ? 'сайт' : 'сайтов'} · DNS → TCP → TLS → HTTP и передача до 64 КиБ`; statusNode.setAttribute('aria-busy', 'true'); }
  try {
    await nfqwsMonitoring.run(hosts);
    await loadBackendMonitoring();
    toast(`Мониторинг завершён: ${hosts.length} сайтов проверено.`);
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Не удалось выполнить проверку мониторинга.';
    if (statusNode) statusNode.textContent = `Проверка не завершена: ${message}`;
    toast(message, 'error');
  } finally {
    monitoringRunning = false;
    hosts.forEach((item) => monitoringHostsRunning.delete(item)); renderMonitoringSites();
    if (button instanceof HTMLButtonElement) { button.disabled = false; button.textContent = '⟳ Проверить все'; }
    if (statusNode) statusNode.removeAttribute('aria-busy');
  }
}
async function addMonitoringSite() {
  const input = $('#monitorHostInput');
  if (!(input instanceof HTMLInputElement) || !monitoringState) return;
  const host = normalizeMonitoringHost(input.value);
  if (!host) return toast('Введите доменное имя без https://, пути или IP-адреса.', 'error');
  if (monitoringState.sites.some((site) => site.host === host)) return toast('Этот домен уже добавлен.', 'error');
  if (monitoringState.sites.length >= 20) return toast('Можно добавить не более 20 доменов.', 'error');
  monitoringState = { ...monitoringState, sites: [{ host, results: [] }, ...monitoringState.sites] };
  input.value = '';
  if (await persistMonitoring()) await runMonitoring(host);
}
async function fetchStatus() {
  if (statusPolling) return;
  statusPolling = true;
  try {
    status = await api<StatusResponse>('/api/status');
    const service = status.service.state;
    const running = service === 'running';
    const stopped = service === 'stopped';
    const missingHooks = running ? status.interception?.missingHooks ?? [] : [];
    const interceptionWarning = missingHooks.length > 0;
    const interceptionDetail = interceptionWarning
      ? `Перехват nfqws2 подключён не полностью: нет перехода из ${missingHooks.join(' и ')}. Перезапустите nfqws2; если проблема повторится — проверьте журнал.`
      : '';
    const label = interceptionWarning
      ? missingHooks.length > 1 ? 'Перехват отключён' : 'Перехват частичный'
      : running ? 'nfqws2 запущен' : stopped ? 'nfqws2 остановлен' : service === 'error' ? 'Ошибка nfqws2' : 'Статус неизвестен';
    const tone = interceptionWarning ? 'amber' : running ? 'emerald' : stopped || service === 'error' ? 'rose' : 'slate';
    if (!restartInProgress) {
      const text = $('#statusTextLabel'); if (text) { text.textContent = label; text.title = interceptionWarning ? interceptionDetail : status.service.detail; text.className = `text-xs sm:text-sm font-semibold text-${tone}-400 tracking-tight`; }
      const dot = $('#statusMainDot'); if (dot) dot.className = `${running && !interceptionWarning ? 'status-dot-active' : ''} relative inline-flex rounded-full h-2.5 w-2.5 bg-${tone}-400`;
      const ring = $('#statusRadarRing'); if (ring) ring.className = `status-radar absolute inline-flex h-full w-full rounded-full bg-${tone}-400`;
    }
    const monitorBadge = $('#sideMonitorBadge'); if (monitorBadge) monitorBadge.className = `w-2 h-2 rounded-full bg-${interceptionWarning ? 'amber' : running ? 'emerald' : 'slate'}-400`;
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
    setText('#statusDetail', `Опрос: ${status.service.checkedAt ? new Date(status.service.checkedAt * 1000).toLocaleTimeString() : 'выполняется'} · метрики: ${status.system.source === 'keenetic-rci' ? 'Keenetic RCI' : 'Linux'}${interceptionWarning ? ` · ${interceptionDetail}` : ''}`);
  } catch (error) {
    status = null;
    if (!restartInProgress) {
      const label = $('#statusTextLabel'); if (label) { label.textContent = 'Статус неизвестен'; label.title = ''; label.className = 'text-xs sm:text-sm font-semibold text-slate-400 tracking-tight'; }
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
      defaultNetworkInterfaces = installation.defaultNetworkInterfaces ?? [];
    } catch (error) {
      toast(error instanceof Error ? error.message : 'Не удалось проверить права записи в конфиг', 'error');
    }
    if (applyButton instanceof HTMLButtonElement) {
      applyButton.disabled = !canWriteConfig;
      applyButton.title = canWriteConfig ? '' : 'Backend не подтвердил право записи в обнаруженный конфиг';
    }
    const editor = $('#nfqwsConfTextarea');
    if (editor instanceof HTMLTextAreaElement) setNfqwsConfigText(config.content);
    setText('#confLinesBadge', `${config.content.split(/\r?\n/).length} строк`);
    setText('#currentConfigPath', 'Конфигурация');
    const configTitle = $('#currentConfigPath'); if (configTitle) configTitle.title = config.file;
    renderNfqwsArgumentEditors(config.content);
    updateQuickParameters(config.content, networkInterfaces, defaultNetworkInterfaces);
    const queue = configAssignmentValue(config.content, 'NFQUEUE_NUM');
    setText('#queueBadge', queue ? `Q:${queue.replace(/^['"]|['"]$/g, '')}` : 'Q:—');
    syncConfigLines();
    renderConfigOutline();
    scheduleConfigWarnings(0);
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
  syncConfigWarningsVisibility();
  const copyButton = $('#copyConfigTextButton');
  copyButton?.classList.toggle('hidden', section !== 'editor');
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
function setConfigClearModal(open: boolean) {
  const modal = $('#configClearModal');
  if (!modal) return;
  if (open) openAppModal(modal, '[data-action-click="cancelClearConfig"]'); else closeAppModal(modal);
}
function confirmClearConfig() {
  const editor = $('#nfqwsConfTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return;
  setNfqwsConfigText('');
  setConfigClearModal(false);
  getConfigCodeEditor()?.focus();
  toast('Черновик конфигурации очищен.');
}
function setListClearModal(open: boolean) {
  const modal = $('#listClearModal');
  if (!modal) return;
  if (open) openAppModal(modal, '[data-action-click="cancelClearListDraft"]'); else closeAppModal(modal);
}
function confirmClearListDraft() {
  const editor = $('#listEditorTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return;
  editor.value = '';
  selectedListRows.clear();
  syncListLineNumbers(editor);
  setListClearModal(false);
  editor.focus();
  toast('Черновик списка очищен.');
}
function toggleListEditorFullscreen() {
  const panel = $('#listEditorPanel');
  const button = $('#listEditorFullscreenToggle');
  if (!panel || !(button instanceof HTMLButtonElement)) return;
  const fullscreen = !panel.classList.contains('list-editor-fullscreen');
  if (fullscreen) {
    const parent = panel.parentNode;
    if (!parent) return;
    listFullscreenPlaceholder = document.createComment('list-editor-placeholder');
    parent.insertBefore(listFullscreenPlaceholder, panel);
    document.body.append(panel);
    panel.classList.add('list-editor-fullscreen');
    document.documentElement.classList.add('list-editor-fullscreen-open');
  } else {
    panel.classList.remove('list-editor-fullscreen');
    if (listFullscreenPlaceholder?.parentNode) listFullscreenPlaceholder.parentNode.insertBefore(panel, listFullscreenPlaceholder);
    listFullscreenPlaceholder?.remove();
    listFullscreenPlaceholder = null;
    document.documentElement.classList.remove('list-editor-fullscreen-open');
  }
  button.setAttribute('aria-pressed', String(fullscreen));
  const label = fullscreen ? 'Свернуть редактор списка' : 'Развернуть редактор списка';
  button.setAttribute('aria-label', label);
  button.setAttribute('data-action-tooltip', label);
  button.removeAttribute('title');
  button.innerHTML = fullscreen
    ? '<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M8 8H5a2 2 0 0 1-2-2V3m13 5h3a2 2 0 0 0 2-2V3M3 16v3a2 2 0 0 0 2 2h5m11-5v3a2 2 0 0 1-2 2h-3"></path></svg>'
    : '<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H5a2 2 0 0 0-2 2v3m13-5h3a2 2 0 0 1 2 2v3M3 16v3a2 2 0 0 0 2 2h3m13-5v3a2 2 0 0 1-2 2h-3"></path></svg>';
  if (fullscreen && listEditorView === 'text') $('#listEditorTextarea')?.focus({ preventScroll: true });
}
function toggleConfigEditorFullscreen() {
  const panel = $('#configEditorPanel');
  const button = $('#configEditorFullscreenToggle');
  if (!panel || !(button instanceof HTMLButtonElement)) return;
  const fullscreen = !panel.classList.contains('config-editor-fullscreen');
  if (fullscreen) {
    const parent = panel.parentNode;
    if (!parent) return;
    fullscreenPlaceholder = document.createComment('config-editor-placeholder');
    parent.insertBefore(fullscreenPlaceholder, panel);
    document.body.append(panel);
    panel.classList.add('config-editor-fullscreen');
  } else {
    panel.classList.remove('config-editor-fullscreen');
    if (fullscreenPlaceholder?.parentNode) fullscreenPlaceholder.parentNode.insertBefore(panel, fullscreenPlaceholder);
    fullscreenPlaceholder?.remove();
    fullscreenPlaceholder = null;
  }
  button.setAttribute('aria-pressed', String(fullscreen));
  button.setAttribute('aria-label', fullscreen ? 'Свернуть редактор' : 'Развернуть редактор');
  button.title = fullscreen ? 'Свернуть редактор' : 'Развернуть редактор';
  if (fullscreen) getConfigCodeEditor()?.focus();
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
  configArgumentViews.forEach((view) => view.destroy());
  configArgumentViews.clear();
  host.replaceChildren();
  for (const [key, label] of NFQWS_ARGUMENT_GROUPS) {
    const card = document.createElement('section');
    card.className = 'block rounded-xl border border-slate-800 bg-[#070b14] p-3';
    const heading = document.createElement('div');
    heading.className = 'mb-2 block font-mono text-xs font-bold text-cyan-200';
    heading.textContent = `${key} · ${label}`;
    const field = document.createElement('textarea');
    const minimumRows = key === 'NFQWS_ARGS_CUSTOM' ? 5 : 3;
    field.id = `configArgument-${key}`;
    field.rows = minimumRows;
    field.spellcheck = false;
    field.dataset.configArgument = 'true';
    field.className = 'config-code-editor-state';
    field.tabIndex = -1;
    field.setAttribute('aria-hidden', 'true');
    field.value = configAssignmentValue(content, key);
    field.placeholder = key === 'NFQWS_ARGS_CUSTOM' ? '--new --filter-tcp=443 --hostlist-domains=example.com …' : 'Введите аргументы для этой группы';
    field.title = 'Изменение обновит черновик; сохранить его можно кнопкой «Применить конфигурацию».';
    const editor = document.createElement('div');
    editor.className = 'config-argument-editor flex min-w-0 overflow-hidden rounded-lg border border-slate-700 bg-slate-950 transition';
    const codeHost = document.createElement('div');
    codeHost.className = 'config-argument-code-editor min-w-0 flex-1 overflow-hidden';
    editor.append(codeHost, field);
    const syncEditor = () => {
      const layout = configEditorLayout(field.value, minimumRows);
      field.rows = layout.visibleRows;
      codeHost.style.height = configEditorViewportHeight(layout.visibleRows);
    };
    field.addEventListener('input', syncEditor);
    syncEditor();
    field.addEventListener('change', () => void updateConfigVariable(key, field.value));
    const view = mountConfigArgumentEditor(codeHost, field.value, (value) => {
      field.value = value;
      field.dispatchEvent(new Event('input', { bubbles: true }));
    }, `${key} · ${label}`, field.placeholder);
    codeHost.addEventListener('focusout', () => field.dispatchEvent(new Event('change', { bubbles: true })));
    configArgumentViews.set(key, view);
    card.append(heading, editor);
    host.append(card);
  }
}
function updateQuickParameters(content: string, networkInterfaces: string[] = [], defaultInterfaces: string[] = []) {
  const availableModes = new Set(availableNfqwsModes(content));
  const interfaceSelect = $('[data-key="ISP_INTERFACE"]');
  if (interfaceSelect instanceof HTMLSelectElement) {
    const configured = configAssignmentValue(content, 'ISP_INTERFACE');
    interfaceSelect.replaceChildren();
    for (const name of networkInterfaceChoices(networkInterfaces, configured, defaultInterfaces)) interfaceSelect.add(new Option(name, name));
    interfaceSelect.add(new Option('Ввести имя вручную…', '__manual__'));
    interfaceSelect.value = configured.trim();
    interfaceSelect.disabled = false;
    interfaceSelect.title = '';
    interfaceSelect.classList.remove('hidden');
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
  if (key === 'ISP_INTERFACE') {
    value = value.trim();
    if (!isValidNetworkInterfaceSelection(value)) {
      toast('Укажите до 16 имён интерфейсов через пробел; каждое имя — 1–15 латинских букв, цифр или символов . _ @ + -.', 'error');
      updateQuickParameters(editor.value, discoveredNetworkInterfaces, defaultNetworkInterfaces);
      return;
    }
  }
  const before = editor.value;
  setBusy(true);
  try {
    const preview = await nfqwsConfig.preview(before, key, value);
    if (editor.value !== before) throw new Error('Черновик изменился во время запроса; повторите выбор настройки.');
    setNfqwsConfigText(preview.content);
    syncConfigLines();
    renderConfigOutline();
    renderNfqwsArgumentEditors(editor.value);
    scheduleConfigWarnings(0);
    toast('Черновик изменён. Нажмите «Применить конфигурацию», чтобы сохранить.');
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось изменить черновик', 'error');
  } finally {
    setBusy(false);
    updateQuickParameters(editor.value, discoveredNetworkInterfaces, defaultNetworkInterfaces);
  }
}
function handleIspInterfaceSelection(value: string) {
  const editor = $('#nfqwsConfTextarea');
  const select = $('#ispInterfaceSelect');
  const manualEntry = $('#ispInterfaceManualEntry');
  const manualInput = $('#ispInterfaceManualInput');
  if (!(select instanceof HTMLSelectElement) || !(editor instanceof HTMLTextAreaElement) || !(manualEntry instanceof HTMLElement) || !(manualInput instanceof HTMLInputElement)) return;
  if (value === '__manual__') {
    manualEntry.classList.remove('hidden');
    manualInput.value = configAssignmentValue(editor.value, 'ISP_INTERFACE');
    manualInput.focus();
    manualInput.select();
    return;
  }
  manualEntry.classList.add('hidden');
  void updateConfigVariable('ISP_INTERFACE', value);
}
function submitIspInterfaceManually() {
  const input = $('#ispInterfaceManualInput');
  if (!(input instanceof HTMLInputElement)) return;
  void updateIspInterfaceManually(input.value);
}
async function updateIspInterfaceManually(value: string) {
  await updateConfigVariable('ISP_INTERFACE', value);
  const editor = $('#nfqwsConfTextarea');
  const manualEntry = $('#ispInterfaceManualEntry');
  if (editor instanceof HTMLTextAreaElement && manualEntry instanceof HTMLElement && configAssignmentValue(editor.value, 'ISP_INTERFACE') === value.trim()) {
    manualEntry.classList.add('hidden');
  }
}
function syncConfigLines() {
  const editor = $('#nfqwsConfTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return;
  const count = Math.max(1, editor.value.split('\n').length);
  setText('#confLinesBadge', `${count} строк`);
}
function syncConfigWarningsVisibility() {
  const panel = $('#configWarningsPanel');
  const badge = $('#configWarningsBadge');
  const editorSection = $('#configEditorPanel');
  const visible = appearanceSettings.showConfigWarnings && !editorSection?.classList.contains('hidden');
  panel?.classList.toggle('hidden', !visible);
  badge?.classList.toggle('hidden', !appearanceSettings.showConfigWarnings);
  if (!appearanceSettings.showConfigWarnings) {
    if (configWarningsTimer !== null) window.clearTimeout(configWarningsTimer);
    configWarningsTimer = null;
    configWarningsRequest += 1;
  }
}
function scheduleConfigWarnings(delay = 500) {
  if (configWarningsTimer !== null) window.clearTimeout(configWarningsTimer);
  configWarningsTimer = null;
  if (!appearanceSettings.showConfigWarnings) { syncConfigWarningsVisibility(); return; }
  configWarningsTimer = window.setTimeout(() => { configWarningsTimer = null; void validateConfigWarnings(); }, delay);
}
async function validateConfigWarnings() {
  const editor = $('#nfqwsConfTextarea');
  if (!(editor instanceof HTMLTextAreaElement) || !appearanceSettings.showConfigWarnings) return;
  const content = editor.value;
  const request = ++configWarningsRequest;
  const list = $('#configWarningsList');
  const summary = $('#configWarningsSummary');
  if (!list || !summary) return;
  summary.textContent = 'Проверка…';
  try {
    const luaEntries = configLuaArgumentEntries(content);
    const [result] = await Promise.all([
      nfqwsConfig.validate(content),
      luaEntries.length ? ensureConfigWarningsLuaCatalog() : Promise.resolve(),
    ]);
    if (request !== configWarningsRequest || editor.value !== content || !appearanceSettings.showConfigWarnings) return;
    const luaWarnings = diagnoseConfigLuaArguments(content, runtimeProfileLuaCatalog, runtimeProfileLuaStandardGroups)
      .map((diagnostic) => ({ severity: 'warning' as const, code: 'lua_argument', message: diagnostic.message, line: diagnostic.line }));
    const warnings = [...result.diagnostics.filter((diagnostic) => diagnostic.severity === 'warning'), ...luaWarnings];
    const badge = $('#configWarningsBadge');
    if (badge) badge.textContent = warnings.length ? `${warnings.length} ${pluralize(warnings.length, 'предупреждение', 'предупреждения', 'предупреждений')}` : 'Предупреждений нет';
    summary.textContent = warnings.length ? `Найдено: ${warnings.length}` : 'Замечаний нет';
    list.replaceChildren();
    if (!warnings.length) {
      const empty = document.createElement('p');
      empty.className = 'px-1 py-1 text-xs text-slate-400';
      empty.textContent = result.valid ? 'В текущем черновике предупреждений нет.' : 'Предупреждений нет; перед применением проверьте ошибки валидации.';
      list.append(empty);
    }
    for (const warning of warnings) {
      const row = document.createElement('div');
      row.className = 'config-warning-row flex flex-wrap items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-sm';
      const message = document.createElement('p');
      message.className = 'flex min-w-0 flex-1 items-start gap-2 text-slate-200';
      const icon = document.createElement('span');
      icon.className = 'shrink-0 text-amber-400';
      icon.setAttribute('aria-hidden', 'true');
      icon.textContent = '⚠';
      const text = document.createElement('span');
      text.className = 'min-w-0';
      text.textContent = `${warning.line ? `Строка ${warning.line}: ` : ''}${warning.message}`;
      message.append(icon, text);
      row.append(message);
      if (warning.line) {
        const goToLine = document.createElement('button');
        goToLine.type = 'button';
        goToLine.className = 'config-warning-action shrink-0 rounded-lg border px-2.5 py-1 text-xs font-medium transition-colors';
        goToLine.textContent = `К строке ${warning.line}`;
        goToLine.addEventListener('click', () => {
          const start = content.split(/\r?\n/).slice(0, warning.line! - 1).join('\n').length + (warning.line! > 1 ? 1 : 0);
          const end = content.indexOf('\n', start);
          selectConfigText(start, end < 0 ? content.length : end, warning.line ?? undefined);
        });
        row.append(goToLine);
      }
      list.append(row);
    }
    syncConfigWarningsVisibility();
  } catch {
    if (request !== configWarningsRequest) return;
    summary.textContent = 'Проверка недоступна';
    list.replaceChildren();
    const unavailable = document.createElement('p');
    unavailable.className = 'px-1 py-1 text-xs text-slate-400';
    unavailable.textContent = 'Не удалось получить предупреждения от backend.';
    list.append(unavailable);
  }
}
async function ensureConfigWarningsLuaCatalog() {
  if (runtimeProfileLuaCatalog.length) return;
  if (!configWarningsCatalogPromise) {
    configWarningsCatalogPromise = fetchRuntimeProfileCatalog()
      .catch(() => undefined)
      .finally(() => { configWarningsCatalogPromise = null; });
  }
  await configWarningsCatalogPromise;
}
let configOutlineRenderedContent: string | null = null;
let configOutlineButtons: HTMLButtonElement[] = [];
let activeConfigOutlineButton: HTMLButtonElement | null = null;
function updateActiveConfigOutline() {
  const cursor = getConfigCodeEditor()?.state.selection.main.head ?? 0;
  let low = 0;
  let high = configOutlineButtons.length;
  while (low < high) {
    const middle = (low + high) >>> 1;
    if (Number(configOutlineButtons[middle].dataset.configOutlineOffset) <= cursor) low = middle + 1;
    else high = middle;
  }
  const nextActive = configOutlineButtons[low - 1] ?? null;
  if (nextActive === activeConfigOutlineButton) return;
  if (activeConfigOutlineButton) {
    activeConfigOutlineButton.classList.remove('bg-blue-500/10', 'text-blue-200');
    activeConfigOutlineButton.classList.add('text-slate-200');
    activeConfigOutlineButton.removeAttribute('aria-current');
  }
  activeConfigOutlineButton = nextActive;
  activeConfigOutlineButton?.classList.add('bg-blue-500/10', 'text-blue-200');
  activeConfigOutlineButton?.classList.remove('text-slate-200');
  activeConfigOutlineButton?.setAttribute('aria-current', 'location');
}
function renderConfigOutline() {
  const editor = $('#nfqwsConfTextarea');
  const entriesHost = $('#configOutlineEntries');
  if (!(editor instanceof HTMLTextAreaElement) || !entriesHost) return;
  const content = editor.value;
  if (content === configOutlineRenderedContent) { updateActiveConfigOutline(); return; }
  configOutlineRenderedContent = content;
  const entries = configOutlineEntries(content);
  entriesHost.replaceChildren();
  configOutlineButtons = [];
  activeConfigOutlineButton = null;
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
    button.className = 'config-outline-entry flex w-full items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-left font-mono text-xs transition-colors hover:bg-slate-800 text-slate-200';
    button.dataset.configOutlineOffset = String(entry.offset);
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
      selectConfigText(entry.offset, entry.offset + entry.name.length, entry.line);
      updateActiveConfigOutline();
    });
    entriesHost.append(button);
    configOutlineButtons.push(button);
  }
  updateActiveConfigOutline();
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
  const baselineContent = config.content;
  const baselineRevision = config.revision;
  if (configDraftDisposition(draft, baselineContent, config.content, baselineRevision, config.revision) === 'unchanged') return toast('Конфигурация не изменена.');
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
  const validatedDraftState = configDraftDisposition(draft, baselineContent, config.content, baselineRevision, config.revision);
  if (validatedDraftState === 'unchanged') {
    setBusy(false);
    return toast('Конфигурация не изменена.');
  }
  if (validatedDraftState === 'stale') {
    setBusy(false);
    return toast('Конфигурация на роутере изменилась во время проверки. Загрузите её заново и повторите действие.', 'error');
  }
  if (!validation.valid) {
    setBusy(false);
    const messages = validation.diagnostics.filter((item) => item.severity === 'error')
      .map((item) => `${item.line ? `Строка ${item.line}: ` : ''}${item.message}`);
    return toast(`Конфигурация не прошла проверку: ${messages.join(' · ')}`, 'error');
  }
  if (configLuaArgumentEntries(draft).length) await ensureConfigWarningsLuaCatalog();
  if (editor.value !== draft) {
    setBusy(false);
    return toast('Черновик изменился во время проверки. Проверьте конфигурацию повторно.', 'error');
  }
  const luaWarnings = diagnoseConfigLuaArguments(draft, runtimeProfileLuaCatalog, runtimeProfileLuaStandardGroups)
    .map((diagnostic) => ({ severity: 'warning' as const, code: 'lua_argument', message: diagnostic.message, line: diagnostic.line }));
  const warnings = [...validation.diagnostics.filter((item) => item.severity === 'warning'), ...luaWarnings];
  const binaryCheckNote = validation.checks.installedBinary === 'not_run'
    ? 'Проверка установленным бинарником не выполнялась.'
    : 'Проверка установленным бинарником выполнена.';
  const applyValidatedConfig = async () => {
    if (editor.value !== draft || !config) { setBusy(false); return toast('Черновик изменился во время подтверждения. Проверьте конфигурацию повторно.', 'error'); }
    const confirmedDraftState = configDraftDisposition(draft, baselineContent, config.content, baselineRevision, config.revision);
    if (confirmedDraftState === 'unchanged') { setBusy(false); return toast('Конфигурация уже соответствует этому черновику.'); }
    if (confirmedDraftState === 'stale') { setBusy(false); return toast('Конфигурация на роутере изменилась во время подтверждения. Загрузите её заново.', 'error'); }
    setBusy(true);
    setRestartInProgress(true);
    try {
      const result = await nfqwsApply.apply(draft, config.revision);
      config = result.config;
      setRestartInProgress(false);
      await fetchStatus();
      toast(lifecycleMessage('restart'));
    } catch (error) {
      setRestartInProgress(false);
      if (error instanceof Error && error.message.includes('Исходный конфиг восстановлен')) await loadConfig();
      await fetchStatus();
      toast(error instanceof Error ? error.message : 'Не удалось сохранить конфигурацию', 'error');
    }
    finally { setRestartInProgress(false); setBusy(false); }
  };
  if (warnings.length || validation.checks.installedBinary === 'not_run') {
    setBusy(false);
    requestDangerousConfirmation('Проверка конфигурации', [
      'Backend проверил только безопасно разбираемый синтаксис и известные ограничения init-скрипта.',
      ...warnings.map((item) => `${item.line ? `Строка ${item.line}: ` : ''}${item.message}`),
      binaryCheckNote,
      'Продолжить запись и перезапуск?'
    ].join('\n\n'), applyValidatedConfig, () => setBusy(false));
    return;
  }
  await applyValidatedConfig();
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
      const usages = discovered?.usedBy ?? [];
      const usageTitle = !discovered ? '' : discovered.usageKnown === false
        ? '\nНе удалось проверить, какие профили используют список.'
        : usages.length === 0 ? discovered.validationRequired === false
          ? '\nНе подключён ни к одному профилю.'
          : '\nНе используется ни одним разобранным профилем.'
          : `\nИспользуют: ${usages.map((usage) => `#${usage.profileIndex} ${usage.profileName} (${listUsageRoleLabel(usage.role)})`).join(', ')}`;
      const validationNotice = listValidationNotice(discovered?.validationRequired);
      candidate.title = discovered ? `${tabId ? listDisplayName(tabId) : 'Список'} — ${discovered.path}${usageTitle}${validationNotice ? `\n${validationNotice}` : ''}` : 'Файл не найден ни по ссылке конфигурации, ни в штатном каталоге lists';
    }
    if (!summary) {
      activeList = null;
      $('#listAutoUpdateStatus')?.classList.add('hidden');
      const editor = $('#listEditorTextarea');
      if (editor instanceof HTMLTextAreaElement) { editor.value = ''; editor.placeholder = 'Файл отсутствует; панель не создаёт списки автоматически.'; }
      setText('#currentListFilePath', 'Файл не найден');
      setText('#listProfileUsageStatus', '');
      setText('#listFooterCount', 'Список недоступен');
      return toast(`Файл ${id} не найден по настройкам nfqws2 или в штатном каталоге lists.`, 'error');
    }
    const result = await nfqwsLists.get(id);
    activeList = result;
    activeListId = id;
    const autoStatus = $('#listAutoUpdateStatus');
    if (autoStatus instanceof HTMLElement) {
      const managed = summary.managedAsn;
      autoStatus.classList.toggle('hidden', !managed);
      if (managed) {
        autoStatus.className = `text-xs font-medium ${managed.paused || managed.lastError ? 'text-amber-300' : 'text-slate-400'}`;
        const lastSuccess = managed.lastSuccessDate ? `обновлён ${managed.lastSuccessDate}` : 'ещё не обновлялся';
        const lastAttempt = managed.lastAttemptDate ? ` · попытка ${managed.lastAttemptDate}` : '';
        autoStatus.textContent = managed.paused
          ? `AS${managed.asn} · автообновление остановлено: ${managed.lastError || 'список изменён вручную'} · ${lastSuccess}`
          : managed.pending
            ? `AS${managed.asn} · завершается обновление списка · ${lastSuccess}`
            : managed.lastError
              ? `AS${managed.asn} · ошибка ежедневного обновления${lastAttempt}: ${managed.lastError}; список сохранён, ${lastSuccess}`
              : `AS${managed.asn} · ${managed.prefixCount} ${pluralize(managed.prefixCount, 'префикс', 'префикса', 'префиксов')} · ${lastSuccess} · обновляется ежедневно`;
      }
    }
    selectedListRows.clear();
    const editor = $('#listEditorTextarea');
    if (editor instanceof HTMLTextAreaElement) { editor.value = result.content; editor.placeholder = ''; }
    const locationLabel = $('#currentListFilePath');
    if (locationLabel) {
      locationLabel.textContent = `${listDisplayName(id)} — ${id}`;
      locationLabel.title = result.path;
      locationLabel.setAttribute('aria-label', `${listDisplayName(id)}. Путь: ${result.path}`);
    }
    const usageStatus = $('#listProfileUsageStatus');
    if (usageStatus) {
      const usages = summary.usedBy ?? [];
      usageStatus.textContent = formatListProfileUsageStatus({
        usageKnown: summary.usageKnown,
        validationRequired: summary.validationRequired,
        usages: usages.map((usage) => `#${usage.profileIndex} ${usage.profileName} · ${listUsageRoleLabel(usage.role)}`),
      });
      usageStatus.classList.toggle('text-amber-300', summary.usageKnown === false);
      usageStatus.classList.toggle('text-slate-400', summary.usageKnown !== false);
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

function listUsageRoleLabel(role: 'include' | 'exclude' | 'auto'): string {
  return role === 'exclude' ? 'исключение' : role === 'auto' ? 'автосписок' : 'подключён';
}

function setListEditorView(view: 'text' | 'rows') {
  listEditorView = view === 'rows' ? 'rows' : 'text';
  try { localStorage.setItem(LIST_EDITOR_VIEW_KEY, view); } catch { /* The view preference is optional. */ }
  const textView = $('#listTextView');
  const rowsView = $('#listRowsView');
  textView?.classList.toggle('hidden', listEditorView !== 'text');
  rowsView?.classList.toggle('hidden', listEditorView !== 'rows');
  const viewSelect = $('#listEditorViewSelect') as HTMLSelectElement | null;
  if (viewSelect && viewSelect.value !== listEditorView) viewSelect.value = listEditorView;
  const remove = $('[data-action-click="removeSelectedListLines"]');
  if (remove) remove.textContent = listEditorView === 'rows' ? `Удалить выбранные (${selectedListRows.size})` : 'Удалить строку/выделение';
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
  const rows = listRowsFromContent(editor.value);
  root.replaceChildren();
  if (!rows.length) {
    const empty = document.createElement('p');
    empty.className = 'px-3 py-6 text-center text-sm text-slate-400';
    empty.textContent = 'Список пуст.';
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
    remove.setAttribute('data-action-tooltip', `Удалить строку ${row.index + 1}`);
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
    button.dataset.actionTooltip = `Пользовательский список: ${summary.path}`;
    button.dataset.actionTooltipDelayMs = '3000';
    button.className = 'list-file-tab flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-medium text-slate-400 hover:text-slate-200 border-b-2 border-transparent transition-all whitespace-nowrap';
    button.setAttribute('aria-label', `${listDisplayName(id)} — ${id}`);
    const label = document.createElement('span'); label.className = 'list-file-tab-caption';
    const filename = document.createElement('code'); filename.textContent = id; label.append(filename);
    button.append(label);
    const remove = document.createElement('button');
    remove.type = 'button';
    remove.dataset.actionClick = 'requestDeleteCustomList';
    remove.dataset.args = JSON.stringify([id]);
    remove.dataset.customListDelete = 'true';
    const removeDisabled = summary.usageKnown === false || Boolean(summary.usedBy?.length);
    remove.disabled = removeDisabled;
    remove.dataset.actionTooltip = summary.usageKnown === false
      ? 'Нельзя проверить связи списка: удаление заблокировано'
      : summary.usedBy?.length
        ? 'Список используется профилями. Сначала отключите его в конструкторе стратегий.'
        : `Удалить пользовательский список ${id}`;
    remove.className = 'list-file-delete';
    remove.setAttribute('aria-label', `Удалить пользовательский список ${id}`);
    remove.innerHTML = '<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M8 6V4h8v2m-9 0 1 14h8l1-14M10 10v6m4-6v6"></path></svg>';
    if (removeDisabled) remove.classList.add('opacity-40', 'cursor-not-allowed');
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
  openAppModal(modal, '#confirmCustomListDelete');
}
function closeCustomListDeleteModal() {
  closeAppModal($('#customListDeleteModal'));
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
function renderDiagnosisProgress(container: HTMLElement, host: string, activeStage: string, completed: Map<string, SiteDiagnosticStep>, addresses: string[], latestStep: SiteDiagnosticStep | null) {
  const active = SITE_DIAGNOSIS_STAGES.find((stage) => stage.id === activeStage);
  const overview = document.createElement('section'); overview.className = 'diagnosis-overview diagnosis-progress-overview'; overview.setAttribute('role', 'status');
  const spinner = document.createElement('span'); spinner.className = 'diagnosis-progress-spinner'; spinner.setAttribute('aria-hidden', 'true');
  const copy = document.createElement('div'); copy.className = 'diagnosis-overview-copy';
  const title = document.createElement('h3'); title.className = 'diagnosis-overview-title'; title.textContent = `Проверка выполняется · ${active?.label ?? 'завершение'}`;
  const detail = document.createElement('p'); detail.className = 'diagnosis-overview-detail';
  detail.textContent = latestStep ? `${latestStep.detail}${addresses.length ? ` · IP: ${addresses.join(', ')}` : ''}` : `Начинаю проверку ${host}`;
  copy.append(title, detail); overview.append(spinner, copy); container.replaceChildren(overview);
  const checks = document.createElement('section'); checks.className = 'diagnosis-checks diagnosis-live-checks ui-card rounded-2xl';
  const header = document.createElement('div'); header.className = 'diagnosis-checks-header';
  const heading = document.createElement('h3'); heading.className = 'diagnosis-checks-title'; heading.textContent = 'Что проверено';
  const hostLabel = document.createElement('span'); hostLabel.className = 'diagnosis-checks-host'; hostLabel.textContent = host; header.append(heading, hostLabel); checks.append(header);
  const list = document.createElement('div'); list.setAttribute('role', 'list');
  const finished = new Set(completed.keys()); if (finished.has('doh')) finished.add('dns');
  const stages: { id: string; label: string }[] = [...SITE_DIAGNOSIS_STAGES];
  for (const stage of stages) {
    const dns = stage.id === 'dns' ? completed.get('dns') : undefined;
    const doh = stage.id === 'dns' ? completed.get('doh') : undefined;
    const step = dns && doh ? { ...dns, status: doh.status !== 'ok' && dns.status === 'ok' ? 'warning' as const : dns.status, detail: `${dns.detail} · ${doh.detail}` } : dns ?? completed.get(stage.id) ?? (stage.id === 'dns' ? doh : undefined);
    const state = diagnosisProgressState(stage.id, activeStage, finished);
    const skipped = isSkippedDiagnosisStep(step);
    const row = document.createElement('div'); row.className = `diagnosis-row diagnosis-row-live is-${state}${skipped ? ' diagnosis-row-inactive' : ''}${step ? ` diagnosis-${step.status === 'ok' ? 'success' : step.status === 'error' ? 'failure' : 'unknown'}` : ''}`; row.setAttribute('role', 'listitem');
    if (state === 'active') {
      const marker = document.createElement('span'); marker.className = 'diagnosis-progress-spinner diagnosis-progress-spinner-small'; marker.setAttribute('aria-hidden', 'true'); row.append(marker);
    } else {
      const icon = diagnosisIcon(step ? step.status === 'ok' ? 'success' : step.status === 'error' ? 'failure' : 'unknown' : 'unknown');
      if (skipped) { icon.textContent = '—'; icon.setAttribute('aria-label', 'Проверка не нужна'); }
      else if (!step) { icon.textContent = '·'; icon.setAttribute('aria-label', 'Ожидает'); }
      row.append(icon);
    }
    const label = document.createElement('h4'); label.className = 'diagnosis-row-label'; label.textContent = stage.label; row.append(label);
    const body = document.createElement('div'); body.className = 'diagnosis-row-body';
    const result = document.createElement('p'); result.className = 'diagnosis-row-detail'; result.textContent = step?.detail ?? (state === 'active' ? 'Выполняется сейчас…' : 'Ожидает'); body.append(result);
    if (step?.durationMs !== null && step?.durationMs !== undefined) { const timing = document.createElement('p'); timing.className = 'diagnosis-row-metrics'; timing.textContent = `${step.durationMs} мс`; body.append(timing); }
    row.append(body); list.append(row);
  }
  checks.append(list); container.append(checks);
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
    const detail = document.createElement('p'); detail.className = 'diagnosis-row-detail'; detail.textContent = item.detail; body.append(detail);
    if (item.metrics) { const metrics = document.createElement('p'); metrics.className = 'diagnosis-row-metrics'; metrics.textContent = item.metrics; body.append(metrics); }
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
  const toastHost = $('#toastWrapper');
  const closeProgress = toastHost ? showDiagnosisProgress(toastHost, input.value.trim()) : () => undefined;
  const completed = new Map<string, SiteDiagnosticStep>();
  let latestStep: SiteDiagnosticStep | null = null;
  renderDiagnosisProgress(container, input.value.trim(), 'dns', completed, [], latestStep);
  try {
    const report = await nfqwsDiagnostics.siteStream(input.value, (progress) => {
      if (progress.step) { completed.set(progress.step.id, progress.step); latestStep = progress.step; }
      renderDiagnosisProgress(container, input.value.trim(), progress.activeStage, completed, progress.addresses, latestStep);
    });
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
    const empty = document.createElement('p'); empty.className = 'text-sm text-slate-500'; empty.textContent = 'Добавьте конфиги текстом, ссылками или файлами .conf/.txt.'; list.append(empty); return;
  }
  foreignConfigInputs.forEach((item, index) => {
    const row = document.createElement('div'); row.className = 'min-w-0 rounded-lg border border-slate-800 bg-slate-950/35 px-3 py-2';
    const header = document.createElement('div'); header.className = 'flex min-w-0 items-center justify-between gap-3';
    const title = document.createElement('div'); title.className = 'min-w-0';
    const name = document.createElement('strong'); name.className = 'block truncate text-sm text-slate-200'; name.textContent = item.name;
    const source = document.createElement('span'); source.className = 'block truncate text-[11px] text-slate-500'; source.textContent = item.source === 'Ссылка' ? item.url ?? '' : `${item.source} · ${new TextEncoder().encode(item.content ?? '').length} байт`;
    title.append(name, source);
    const actions = document.createElement('div'); actions.className = 'flex shrink-0 items-center gap-1.5';
    if (item.content !== undefined) {
      const preview = document.createElement('pre'); preview.className = 'foreign-config-preview'; preview.hidden = !expandedForeignConfigPreviews.has(item); preview.textContent = item.content;
      const toggle = document.createElement('button'); toggle.type = 'button'; toggle.className = 'rounded-md border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:border-slate-500 hover:text-white'; toggle.textContent = preview.hidden ? 'Показать' : 'Скрыть';
      toggle.setAttribute('aria-expanded', String(!preview.hidden));
      toggle.setAttribute('aria-label', `${preview.hidden ? 'Раскрыть' : 'Свернуть'} конфиг ${item.name}`);
      toggle.addEventListener('click', () => {
        preview.hidden = !preview.hidden;
        if (preview.hidden) expandedForeignConfigPreviews.delete(item);
        else expandedForeignConfigPreviews.add(item);
        toggle.textContent = preview.hidden ? 'Показать' : 'Скрыть';
        toggle.setAttribute('aria-expanded', String(!preview.hidden));
        toggle.setAttribute('aria-label', `${preview.hidden ? 'Раскрыть' : 'Свернуть'} конфиг ${item.name}`);
      });
      actions.append(toggle);
      row.append(header, preview);
    }
    const remove = document.createElement('button'); remove.type = 'button'; remove.className = 'shrink-0 rounded-md border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:border-rose-400 hover:text-rose-200'; remove.textContent = 'Убрать';
    remove.addEventListener('click', () => { foreignConfigInputs.splice(index, 1); renderForeignConfigInputs(); });
    actions.append(remove);
    header.append(title, actions);
    if (item.content === undefined) row.append(header);
    list.append(row);
  });
}

function addForeignConfig(input: ForeignConfigInput): boolean {
  if (foreignConfigInputs.length >= 10) { toast('Можно сравнить не больше десяти конфигов за один запуск.', 'error'); return false; }
  foreignConfigInputs.push(input);
  renderForeignConfigInputs();
  closeForeignConfigForms();
  toast(`Добавлен конфиг «${input.name}».`, 'success');
  return true;
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

function openForeignConfigFile() {
  const input = $('#foreignConfigFileInput');
  if (input instanceof HTMLInputElement) input.click();
}

async function addForeignConfigFile(file: File): Promise<boolean> {
  if (!isSupportedForeignConfigFileName(file.name)) { toast(`Файл «${file.name}»: выберите .conf или .txt.`, 'error'); return false; }
  if (file.size > 65536) { toast(`Файл «${file.name}» больше 64 КиБ.`, 'error'); return false; }
  try {
    const content = decodeForeignConfigText(await file.arrayBuffer());
    if (!content.trim()) { toast(`Файл «${file.name}» пустой.`, 'error'); return false; }
    return addForeignConfig({ name: file.name, source: 'Файл', content });
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось прочитать файл конфигурации.', 'error');
    return false;
  }
}

function foreignStatusLabel(value: boolean | null) {
  return value === true ? 'Доступен' : value === false ? 'Не отвечает' : 'Ожидает';
}

function appendPodkopWarning(container: HTMLElement, hosts: readonly string[] | undefined) {
  const presentation = podkopWarningPresentation(hosts);
  if (!presentation) return;
  const warning = document.createElement('aside'); warning.className = 'podkop-route-warning'; warning.setAttribute('role', 'note');
  const title = document.createElement('strong'); title.textContent = presentation.title;
  const detail = document.createElement('p'); detail.textContent = presentation.detail;
  warning.append(title, detail); container.append(warning);
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
  const openDisclosures = captureForeignConfigDisclosureState(results.querySelectorAll<HTMLDetailsElement>('details[data-disclosure-key]'));
  results.replaceChildren();
  appendPodkopWarning(results, operation.podkopHosts);
  const renderSites = (sites: ForeignConfigCheckOperation['comparisons'][number]['sites'], key: 'currentOk' | 'candidateOk', detailKey: 'currentDetail' | 'candidateDetail') => {
    const list = document.createElement('div'); list.className = 'mt-2 divide-y divide-slate-800/70';
    for (const site of sites) {
      const row = document.createElement('div'); row.className = 'grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-3 gap-y-1 py-2 text-xs sm:grid-cols-[minmax(0,33%)_auto_minmax(0,1fr)]';
      const host = document.createElement('strong'); host.className = 'min-w-0 truncate font-mono font-semibold text-white'; host.textContent = site.host;
      const result = site[key];
      const attemptLabel = document.createElement('span'); attemptLabel.className = 'inline-flex w-fit items-center gap-1.5 whitespace-nowrap rounded bg-slate-800/70 px-1.5 py-0.5 font-mono text-[10px] text-slate-200';
      const legacyCounts = site[detailKey].match(/^(\d+)\/(\d+) успешных/);
      const reportedAttempts = key === 'currentOk' ? (site.currentAttempts ?? 0) : (site.candidateAttempts ?? 0);
      const reportedSuccesses = key === 'currentOk' ? (site.currentSuccesses ?? 0) : (site.candidateSuccesses ?? 0);
      const attempts = reportedAttempts || (key === 'currentOk'
        ? (result === null ? 0 : 1)
        : Number(legacyCounts?.[2] ?? (result === null ? 0 : 1)));
      const successes = reportedAttempts > 0
        ? reportedSuccesses
        : key === 'currentOk'
          ? Number(result === true)
          : Number(legacyCounts?.[1] ?? Number(result === true));
      const averageMs = key === 'currentOk' ? site.currentAverageMs : site.candidateAverageMs;
      const attemptCount = result === null
        ? '—'
        : formatForeignConfigProbeSummary(successes, attempts, averageMs);
      const summaryText = document.createElement('span');
      summaryText.textContent = result === null ? '—' : `${successes}/${attempts} успешных`;
      attemptLabel.append(summaryText);
      if (result !== null && averageMs != null) {
        const latency = document.createElement('span'); latency.className = 'text-teal-200'; latency.textContent = `· ${averageMs} мс`;
        attemptLabel.append(latency);
      }
      attemptLabel.setAttribute('aria-label', `Успешные проверки: ${attemptCount}`);
      const statusText = document.createElement('span'); statusText.className = `col-span-2 justify-self-end text-right font-semibold sm:col-span-1 ${result === true ? 'text-emerald-300' : result === false ? 'text-rose-300' : 'text-slate-500'}`; statusText.textContent = `Домен: ${foreignStatusLabel(result)}`;
      row.append(host, attemptLabel, statusText); list.append(row);
      if (result !== true && site[detailKey]) {
        const detail = document.createElement('p'); detail.className = 'pb-2 text-[11px] leading-4 text-slate-400'; detail.textContent = site[detailKey]; list.append(detail);
      }
    }
    return list;
  };
  const makeDisclosure = (disclosureKey: string, title: string, sites: ForeignConfigCheckOperation['comparisons'][number]['sites'], key: 'currentOk' | 'candidateOk', detailKey: 'currentDetail' | 'candidateDetail', successfulLabel: string, action?: HTMLButtonElement) => {
    const passed = sites.filter((site) => site[key] === true).length;
    const panel = document.createElement('div'); panel.className = 'ui-card min-w-0 self-start h-fit overflow-hidden rounded-xl border border-slate-800';
    const heading = document.createElement('div'); heading.className = 'flex min-h-11 flex-wrap items-center justify-between gap-2 px-3 py-2';
    const headingTitle = document.createElement('strong'); headingTitle.className = 'min-w-0 text-sm font-semibold text-white'; headingTitle.textContent = title;
    const headingActions = document.createElement('div'); headingActions.className = 'flex shrink-0 items-center gap-2';
    const count = document.createElement('span'); count.className = `text-xs ${sites.length > 0 && passed === sites.length ? 'text-emerald-300' : 'text-slate-400'}`; count.textContent = `${passed}/${sites.length} доменов · ${sites.length > 0 && passed === sites.length ? successfulLabel : 'есть недоступные или ожидают'}`;
    headingActions.append(count);
    if (action) headingActions.append(action);
    heading.append(headingTitle, headingActions);
    const details = document.createElement('details'); details.className = 'border-t border-slate-800/80';
    details.dataset.disclosureKey = disclosureKey;
    details.open = openDisclosures.has(disclosureKey);
    const summary = document.createElement('summary'); summary.className = 'flex cursor-pointer list-none items-center justify-between gap-2 px-3 py-1.5 text-xs font-semibold text-slate-300'; summary.textContent = 'Результаты по сайтам';
    const chevron = document.createElement('span'); chevron.className = 'details-chevron'; chevron.setAttribute('aria-hidden', 'true'); chevron.textContent = '⌄'; summary.append(chevron);
    const siteList = renderSites(sites, key, detailKey); siteList.classList.add('px-3');
    details.append(summary, siteList); panel.append(heading, details);
    return panel;
  };

  const baselineSites = operation.comparisons[0]?.sites ?? [];
  results.append(makeDisclosure('current', 'Текущая конфигурация', baselineSites, 'currentOk', 'currentDetail', 'все доступны'));
  for (const [index, comparison] of operation.comparisons.entries()) {
    const candidateWorks = operation.state === 'complete' && comparison.sites.length > 0 && comparison.sites.every((site) => site.candidateOk === true);
    let apply: HTMLButtonElement | undefined;
    if (candidateWorks) {
      const applyButton = document.createElement('button'); applyButton.type = 'button'; applyButton.className = 'rounded-lg bg-blue-600 px-3 py-1.5 text-sm font-semibold text-white hover:bg-blue-500 disabled:opacity-50'; applyButton.textContent = 'Применить конфигурацию';
      applyButton.addEventListener('click', () => void applyForeignConfig(operation.id, index, comparison.name, applyButton));
      apply = applyButton;
    }
    const candidate = makeDisclosure(`candidate-${index}`, `Проверяемая конфигурация · ${comparison.name}`, comparison.sites, 'candidateOk', 'candidateDetail', candidateWorks ? 'Конфигурация работает' : 'все доступны', apply);
    const source = document.createElement('span'); source.className = 'block px-3 pb-2 text-[11px] text-slate-500'; source.textContent = comparison.source; candidate.append(source);
    results.append(candidate);
  }
}

async function applyForeignConfig(operationId: string, index: number, name: string, button: HTMLButtonElement) {
  requestDangerousConfirmation('Заменить текущую конфигурацию?', `Конфигурация «${name}» будет сохранена и nfqws2 перезапустится.`, async () => {
    button.disabled = true; button.textContent = 'Применение…';
    try {
      const result = await nfqwsDiagnostics.foreignConfigApply(operationId, index);
      config = result.config;
      await loadConfig();
      toast('Конфигурация применена и nfqws2 перезапущен.', 'success');
    } catch (error) {
      toast(error instanceof Error ? error.message : 'Не удалось применить конфигурацию.', 'error');
      button.disabled = false; button.textContent = 'Применить конфигурацию';
    }
  });
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
  if (!hosts.length || hosts.length > 15) return toast('Укажите от одного до пятнадцати сайтов или публичных IPv4.', 'error');
  const repeatsInput = $('#foreignConfigRepeats');
  const repeats = repeatsInput instanceof HTMLSelectElement ? Number(repeatsInput.value) : 3;
  const button = $('#runForeignConfigCheck');
  if (button instanceof HTMLButtonElement) { button.disabled = true; button.textContent = 'Запуск…'; }
  try {
    const operation = await nfqwsDiagnostics.foreignConfigStart(foreignConfigInputs, hosts, repeats);
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
  if (operation.hosts && operation.hosts.length > 1 && strategyTargetMode !== 'multi') setStrategyTargetMode('multi');
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
  appendPodkopWarning(container, operation.podkopHosts);
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
  if (operation.multiSiteBaseline?.length) {
    const baseline = document.createElement('div'); baseline.className = 'strategy-multi-domain-results';
    for (const domain of operation.multiSiteBaseline) {
      const row = document.createElement('div'); row.className = 'strategy-multi-domain-result';
      const host = document.createElement('span'); host.textContent = domain.host;
      const outcome = document.createElement('strong'); outcome.textContent = domain.eligible ? 'участвует в подборе' : domain.reason;
      row.append(host, outcome); baseline.append(row);
    }
    resultSummary.append(baseline);
  }
  if (!operation.results.length) {
    if (operation.state === 'site_unblocked') resultTitle.textContent = operation.protocol === 'quic' ? 'QUIC уже отвечает напрямую' : 'Без обхода сайт уже отвечает';
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
    const multiResult = operation.multiSiteResults?.[index];
    if (multiResult) {
      const domainRows = document.createElement('div'); domainRows.className = 'strategy-multi-domain-results';
      for (const domain of multiResult.domains) {
        const row = document.createElement('div'); row.className = 'strategy-multi-domain-result';
        const host = document.createElement('span'); host.textContent = domain.host;
        const outcome = document.createElement('strong'); outcome.textContent = `${domain.successes}/${domain.attempts} · ${domain.elapsedMs} мс${domain.confirmed ? ' · подтверждён' : ''}`;
        row.append(host, outcome); domainRows.append(row);
      }
      body.append(domainRows);
    }
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
    const communityResult = result.candidate.source.includes('Стратегии сообщества');
    const terminalResult = ['complete', 'partial', 'cancelled', 'failed', 'no_match'].includes(operation.state);
    if ((!operation.hosts || operation.hosts.length <= 1) && (result.confirmed || (communityResult && terminalResult))) {
      const share = document.createElement('button');
      share.type = 'button'; share.className = 'backup-action ml-2';
      share.dataset.actionClick = 'openCommunityShare';
      share.dataset.args = JSON.stringify([operation.id, index]);
      share.textContent = communityResult ? (result.confirmed ? 'Подтвердить для сообщества' : 'Сообщить: не сработала') : 'Поделиться';
      card.append(share);
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

async function previewFreezeJoin(operationId: string, resultIndex: string) {
  const index = Number(resultIndex);
  if (!Number.isInteger(index) || index < 0) return toast('Индекс проверенной сети некорректен.', 'error');
  const panel = $('#freezeJoinPreviewPanel');
  const summary = $('#freezeJoinPreviewSummary');
  const diff = $('#freezeJoinPreviewDiff');
  const apply = $('#applyFreezeJoinButton');
  if (!panel || !summary || !diff || !(apply instanceof HTMLButtonElement)) return;
  activeFreezeJoinPreview = null;
  panel.classList.remove('hidden');
  summary.textContent = 'Проверяю ASN, выбранный профиль и текущую конфигурацию…';
  diff.textContent = '';
  apply.disabled = true;
  try {
    const preview = await nfqwsDiagnostics.freezeJoinPreview(operationId, index);
    activeFreezeJoinPreview = preview;
    summary.textContent = `AS${preview.asn} · ${preview.prefixCount} префиксов · ${preview.listId} · профиль #${preview.profileIndex} «${preview.profileName}». ${preview.summary}`;
    const before = preview.originalContent.split(/\r?\n/);
    const after = preview.proposedContent.split(/\r?\n/);
    const maxLines = Math.max(before.length, after.length);
    for (let lineIndex = 0; lineIndex < maxLines; lineIndex += 1) {
      if (before[lineIndex] === after[lineIndex]) continue;
      const row = document.createElement('div');
      row.className = 'grid gap-1 border-b border-slate-800 py-2 last:border-0 sm:grid-cols-2';
      const oldLine = document.createElement('code'); oldLine.className = 'whitespace-pre-wrap break-all text-rose-200'; oldLine.textContent = before[lineIndex] === undefined ? '− (нет строки)' : `− ${before[lineIndex]}`;
      const newLine = document.createElement('code'); newLine.className = 'whitespace-pre-wrap break-all text-emerald-200'; newLine.textContent = after[lineIndex] === undefined ? '+ (удалено)' : `+ ${after[lineIndex]}`;
      row.append(oldLine, newLine); diff.append(row);
    }
    if (!diff.childElementCount) diff.textContent = 'Изменений в конфигурации нет.';
    apply.disabled = false;
    apply.dataset.args = JSON.stringify([preview.operationId, String(preview.resultIndex), preview.originalRevision]);
  } catch (error) {
    activeFreezeJoinPreview = null;
    panel.classList.add('hidden');
    toast(error instanceof Error ? error.message : 'Добавление ASN-сети для этого результата недоступно.', 'error');
  }
}

function closeFreezeJoinPreview() {
  activeFreezeJoinPreview = null;
  $('#freezeJoinPreviewPanel')?.classList.add('hidden');
}

function applyFreezeJoin(operationId: string, resultIndex: string, expectedRevision: string) {
  const preview = activeFreezeJoinPreview;
  if (!preview || preview.operationId !== operationId || preview.resultIndex !== Number(resultIndex) || preview.originalRevision !== expectedRevision) {
    return toast('Предпросмотр устарел. Откройте его заново.', 'error');
  }
  requestDangerousConfirmation('Добавить ASN-сеть в профиль?', `В профиль #${preview.profileIndex} «${preview.profileName}» будет добавлен список ${preview.listId} с ${preview.prefixCount} префиксами для AS${preview.asn}. Конфигурация будет проверена, сохранена в резервную копию и применена с перезапуском nfqws2.`, async () => {
    const button = $('#applyFreezeJoinButton');
    if (button instanceof HTMLButtonElement) { button.disabled = true; button.textContent = 'Применяю…'; }
    try {
      const applied = await nfqwsDiagnostics.applyFreezeJoin(operationId, Number(resultIndex), expectedRevision);
      closeFreezeJoinPreview();
      toast(applied.message || 'Список ASN добавлен в профиль.', 'success');
      await Promise.all([loadConfig(), loadProfiles()]);
    } catch (error) {
      if (button instanceof HTMLButtonElement) { button.disabled = false; button.textContent = 'Добавить и перезапустить'; }
      toast(error instanceof Error ? error.message : 'Не удалось добавить ASN-сеть в профиль.', 'error');
    }
  });
}

async function applyFreezeStrategyOneClick(operationId: string, resultIndex: string) {
  const index = Number(resultIndex);
  const actionKey = `${operationId}:${resultIndex}`;
  if (freezeStrategyApplyPending.has(actionKey) || freezeStrategyApplyDone.has(actionKey)) return;
  const rowButton = [...document.querySelectorAll<HTMLButtonElement>('[data-action-click="applyFreezeStrategyOneClick"]')]
    .find((button) => button.dataset.args === JSON.stringify([operationId, resultIndex]));
  if (!Number.isInteger(index) || index < 0) return toast('Индекс проверенного домена некорректен.', 'error');
  const editor = $('#nfqwsConfTextarea');
  if (config && editor instanceof HTMLTextAreaElement && !sameConfigText(editor.value, config.content)) {
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
  if (config && editor instanceof HTMLTextAreaElement && !sameConfigText(editor.value, config.content)) return toast('Сначала примените или отмените черновик конфигурации. Стратегия не применена.', 'error');
  const applyScope = source === 'strategyPick'
    ? 'Добавить подтверждённую стратегию как Strategy #1 в NFQWS_ARGS и сдвинуть прежние номера вниз? Это повлияет на все сайты; NFQWS_BASE_ARGS не меняется.'
    : `Добавить профиль только для ${$(summaryId)?.textContent?.split(' · ')[0] ?? 'проверенного домена'}?`;
  requestDangerousConfirmation('Применить стратегию?', `${applyScope}\n\nnfqws2 перезапустится. Будет создана резервная копия; при ошибке проверки конфигурация откатится.`, async () => {
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
  });
}

async function loadStrategyPickHistory() {
  const container = $('#autotuneConsoleBox');
  if (!container) return;
  const clearButton = $('#clearStrategyPickHistoryButton');
  try {
    const history = await nfqwsDiagnostics.strategyHistory();
    if (clearButton instanceof HTMLButtonElement) clearButton.disabled = history.length === 0;
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
    if (clearButton instanceof HTMLButtonElement) clearButton.disabled = true;
    container.replaceChildren();
    const message = document.createElement('p'); message.className = 'text-xs text-amber-300';
    message.textContent = error instanceof Error ? `История недоступна: ${error.message}` : 'История подборов недоступна';
    container.append(message);
  }
}

function setStrategyPickHistoryClearModal(open: boolean) {
  const modal = $('#strategyHistoryClearModal');
  if (!modal) return;
  if (open) openAppModal(modal, '[data-action-click="cancelClearStrategyPickHistory"]'); else closeAppModal(modal);
}

async function clearStrategyPickHistory() {
  const confirmButton = $('[data-action-click="confirmClearStrategyPickHistory"]');
  if (confirmButton instanceof HTMLButtonElement) confirmButton.disabled = true;
  try {
    const result = await nfqwsDiagnostics.clearStrategyHistory();
    setStrategyPickHistoryClearModal(false);
    await loadStrategyPickHistory();
    toast(result.removed ? `История очищена: удалено записей ${result.removed}.` : 'Завершённых подборов для удаления нет.');
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось очистить историю подборов.', 'error');
  } finally {
    if (confirmButton instanceof HTMLButtonElement) confirmButton.disabled = false;
  }
}

async function pollStrategyPick(id: string, notifyOnCompletion = true) {
  activeStrategyPickId = id; strategyPickRunning = true;
  const input = $('#scanCustomHost'); const start = $('#startScanBtn'); const stop = $('#strategyPickCancel');
  if (input instanceof HTMLInputElement) input.disabled = true;
  const multiInput = $('#strategyMultiHostList'); if (multiInput instanceof HTMLTextAreaElement) multiInput.disabled = true;
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
    if (multiInput instanceof HTMLTextAreaElement) multiInput.disabled = false;
    if (start instanceof HTMLButtonElement) { start.disabled = false; start.textContent = 'Подобрать стратегию'; }
    stop?.classList.add('hidden');
  }
}

async function startStrategyPick() {
  if (strategyPickRunning) return;
  const input = $('#scanCustomHost'); const proto = $('#strategyProtocolSelect'); const mode = $('#strategyModeSelect'); const repeatsInput = $('#strategyRepeatsSelect'); const customInput = $('#strategyCustomSteps');
  const includeCommunity = ($('#strategyIncludeCommunity') as HTMLInputElement | null)?.checked ?? false;
  if (strategyTargetMode === 'multi') {
    const list = $('#strategyMultiHostList');
    if (!(list instanceof HTMLTextAreaElement)) return toast('Поле списка сайтов недоступно.', 'error');
    const hosts = list.value.split(/[\s,;]+/).map((host) => host.trim()).filter(Boolean);
    if (hosts.length < 2 || hosts.length > 15) return toast('Для общего профиля укажите от 2 до 15 доменов.', 'error');
    const protocol = proto instanceof HTMLSelectElement && ['http', 'https'].includes(proto.value) ? proto.value as 'http' | 'https' : 'https';
    const repeats = repeatsInput instanceof HTMLSelectElement ? Number(repeatsInput.value) : 3;
    if (!Number.isInteger(repeats) || repeats < 2 || repeats > 5) return toast('Число повторов должно быть от 2 до 5.', 'error');
    const container = $('#autotuneStrategiesList'); container?.replaceChildren();
    $('#autotunePreviewPanel')?.classList.add('hidden'); activeStrategyPreview = null;
    const applyButton = $('#applyStrategyPickButton'); if (applyButton instanceof HTMLButtonElement) applyButton.disabled = true;
    try {
      const includeHistory = ($('#strategyIncludeOtherHistory') as HTMLInputElement | null)?.checked ?? true;
      const operation = await nfqwsDiagnostics.strategyMultiStart(hosts, protocol, repeats, includeHistory, includeCommunity);
      activeStrategyPickId = operation.id; localStorage.setItem(STRATEGY_PICK_STORAGE, operation.id);
      await pollStrategyPick(operation.id);
    } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось запустить многосайтовый подбор.', 'error'); }
    return;
  }
  if (!(input instanceof HTMLInputElement) || !input.value.trim()) return toast('Введите домен или публичный IPv4 для подбора.', 'error');
  const protocol = proto instanceof HTMLSelectElement && ['http', 'https', 'quic'].includes(proto.value) ? proto.value as 'http' | 'https' | 'quic' : 'https';
  if (protocol === 'quic' && isStrategyPickIpLiteral(input.value.trim())) return toast('QUIC-подбор требует доменное имя для TLS SNI. Укажите домен, например example.com.', 'error');
  const selectedMode = mode instanceof HTMLSelectElement && ['extended', 'full', 'custom'].includes(mode.value) ? mode.value as 'extended' | 'full' | 'custom' : 'quick';
  const repeats = repeatsInput instanceof HTMLSelectElement ? Number(repeatsInput.value) : 3;
  const stopAfterFirst = ($('#strategyStopAfterFirst') as HTMLInputElement | null)?.checked ?? false;
  const includeOtherHistory = ($('#strategyIncludeOtherHistory') as HTMLInputElement | null)?.checked ?? false;
  const refineLighter = ($('#strategyRefineLighter') as HTMLInputElement | null)?.checked ?? false;
  if (!Number.isInteger(repeats) || repeats < 2 || repeats > 5) return toast('Число повторов должно быть от 2 до 5.', 'error');
  let customSteps: string[][] = [];
  try { customSteps = customInput instanceof HTMLTextAreaElement ? parseCustomStrategies(strategyCustomText()) : []; }
  catch (error) { return toast(error instanceof Error ? error.message : 'Не удалось прочитать собственные стратегии.', 'error'); }
  if (selectedMode === 'custom' && customSteps.length === 0) return toast('Добавьте хотя бы один параметр --lua-desync=…', 'error');
  const container = $('#autotuneStrategiesList'); container?.replaceChildren();
  $('#autotunePreviewPanel')?.classList.add('hidden');
  activeStrategyPreview = null;
  const applyButton = $('#applyStrategyPickButton'); if (applyButton instanceof HTMLButtonElement) applyButton.disabled = true;
  try {
    const operation = await nfqwsDiagnostics.strategyStart(input.value.trim(), protocol, selectedMode, repeats, selectedMode === 'custom' ? customSteps : undefined, stopAfterFirst, includeOtherHistory, refineLighter, includeCommunity);
    activeStrategyPickId = operation.id; localStorage.setItem(STRATEGY_PICK_STORAGE, operation.id);
    await pollStrategyPick(operation.id);
  } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось запустить автоподбор', 'error'); }
}

function setStrategyTargetMode(mode: string) {
  strategyTargetMode = mode === 'multi' ? 'multi' : 'single';
  const multi = strategyTargetMode === 'multi';
  $('#strategyMultiTargetPanel')?.classList.toggle('hidden', !multi);
  $('.strategy-target-single-input')?.classList.toggle('hidden', multi);
  $('.strategy-target-transport-row')?.classList.toggle('hidden', multi);
  if (multi) {
    const protocol = $('#strategyProtocolSelect');
    if (protocol instanceof HTMLSelectElement && protocol.value === 'quic') protocol.value = 'https';
  }
  const protocol = $('#strategyProtocolSelect');
  if (protocol instanceof HTMLSelectElement) updateStrategyProtocol(protocol.value);
  all<HTMLElement>('.strategy-mode-specific').forEach((control) => control.classList.toggle('hidden', multi));
  for (const id of ['strategyStopAfterFirst', 'strategyRefineLighter']) $('#'+id)?.closest('label')?.classList.toggle('hidden', multi);
  all<HTMLButtonElement>('[data-action-click="setStrategyTargetMode"]').forEach((button) => button.classList.toggle('active', button.dataset.arg === strategyTargetMode));
}

function isStrategyPickIpLiteral(host: string) {
  const value = host.trim().replace(/^\[|\]$/g, '');
  return value.includes(':') || /^\d{1,3}(?:\.\d{1,3}){3}$/.test(value);
}

function updateStrategyProtocol(protocol: string) {
  const quic = protocol === 'quic';
  const mode = $('#strategyModeSelect');
  if (mode instanceof HTMLSelectElement) {
    for (const value of ['extended', 'full']) {
      const option = mode.querySelector<HTMLOptionElement>(`option[value="${value}"]`);
      if (option) option.disabled = quic;
    }
    if (quic && ['extended', 'full'].includes(mode.value)) mode.value = 'quick';
    updateStrategyMode(mode.value);
  }
  $('.strategy-community-option')?.classList.toggle('hidden', quic);
  $('.strategy-refine-option')?.classList.toggle('hidden', quic);
  const help = $('#strategyTransportHelp');
  if (help) help.textContent = quic
    ? 'Проверяется ответ на QUIC Initial по UDP 443 для одного домена с TLS SNI. Берутся текущий профиль и ограниченный стандартный набор; TCP/HTTP не меняется.'
    : 'HTTP и HTTPS проверяются отдельно по TCP. Для нескольких сайтов доступно только создание общего TCP-профиля.';
}

function extractPublicDomains(content: string) {
  const result = new Set<string>();
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.split(/[#!]/, 1)[0] ?? '';
    for (const raw of line.split(/[\s,;]+/)) {
      const host = raw.trim().replace(/^\*\./, '').replace(/\.$/, '').toLowerCase();
      if (host.length <= 253 && host.includes('.') && !/^\d+(?:\.\d+){3}$/.test(host)
          && host.split('.').every((label) => label.length > 0 && label.length <= 63 && /^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/.test(label))) result.add(host);
    }
  }
  return [...result];
}

async function pickRandomMultiSites() {
  const select = $('#strategyMultiListSelect');
  const button = $('#pickRandomMultiSitesButton');
  if (!(select instanceof HTMLSelectElement)) return;
  if (button instanceof HTMLButtonElement) button.disabled = true;
  try {
    const { content } = await nfqwsLists.get(select.value);
    const candidates = extractPublicDomains(content);
    for (let index = candidates.length - 1; index > 0; index -= 1) {
      const swap = Math.floor(Math.random() * (index + 1));
      [candidates[index], candidates[swap]] = [candidates[swap]!, candidates[index]!];
    }
    const chosen = candidates.slice(0, 15);
    if (chosen.length < 2) return toast('В выбранном списке найдено меньше двух доменов для проверки.', 'error');
    const textarea = $('#strategyMultiHostList');
    if (textarea instanceof HTMLTextAreaElement) textarea.value = chosen.join('\n');
    toast(`Выбрано доменов: ${chosen.length}.`);
  } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось загрузить домены из списка.', 'error'); }
  finally { if (button instanceof HTMLButtonElement) button.disabled = false; }
}

async function loadStrategyDomainLists() {
  const select = $('#strategyMultiListSelect');
  if (!(select instanceof HTMLSelectElement)) return;
  try {
    const inventory = await nfqwsLists.index();
    const lists = inventory.files.filter((file) => file.readable && /\.list$|\.txt$/i.test(file.id));
    select.replaceChildren();
    for (const list of lists) {
      const option = document.createElement('option'); option.value = list.id; option.textContent = listDisplayName(list.id); select.append(option);
    }
    if (!lists.some((list) => list.id === 'user.list') && lists[0]) select.value = lists[0].id;
  } catch { /* manual multi-site input stays available */ }
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
      setText('#freezeScanProgressCount', view.count);
      setText('#freezeScanProgressDetail', view.detail);
      const bar = $('#freezeScanProgressBar');
      if (bar instanceof HTMLElement) {
        bar.style.width = `${view.percent}%`;
        bar.parentElement?.setAttribute('aria-valuenow', String(view.percent));
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
  const container = $('#freezeScanRows');
  const resultsPanel = $('#freezeScanResults');
  if (!container || !resultsPanel) return;
  const resultsSignature = JSON.stringify({ state: operation.state, results: operation.results });
  if (container.dataset.resultsSignature === resultsSignature) return;
  container.dataset.resultsSignature = resultsSignature;
  container.replaceChildren();
  if (!operation.results.length) { resultsPanel.classList.add('hidden'); return; }
  resultsPanel.classList.remove('hidden');
  setText('#freezeScanNetworkCount', `${operation.results.length} ${pluralize(operation.results.length, 'сеть проверена', 'сети проверены', 'сетей проверено')}`);
  for (const [resultIndex, result] of operation.results.entries()) {
    const presentation = freezeRowPresentation({ freezeDetected: result.freezeDetected, foundStrategy: result.foundStrategy, directOk: result.direct.ok, currentOk: result.current.ok, directClassification: result.direct.classification, currentClassification: result.current.classification, inConf: result.inConf });
    const row = document.createElement('article'); row.className = `freeze-scan-network-row freeze-result-${presentation.tone}`;
    row.setAttribute('role', 'listitem');
    row.dataset.resultTone = presentation.tone;
    const network = document.createElement('div');
    network.className = 'freeze-result-network';
    const [networkName, networkEndpoint] = freezeRowIdentity(result).split('\n', 2);
    const label = document.createElement('strong');
    label.textContent = networkName || result.label;
    network.append(label);
    if (networkEndpoint) {
      const endpoint = document.createElement('span');
      endpoint.className = 'freeze-result-host';
      endpoint.textContent = networkEndpoint;
      network.append(endpoint);
    }
    row.append(network);

    const verifiedNetwork = document.createElement('div');
    verifiedNetwork.className = 'freeze-result-address';
    verifiedNetwork.textContent = `${result.address ?? 'адрес не найден'}\n${result.asn.state === 'verified' ? `AS${result.asn.asn} · подтверждён` : result.asn.state === 'changed' ? `ASN изменился: ожидался AS${result.asn.expected}, сейчас AS${result.asn.actual}` : 'ASN не подтверждён'}`;
    row.append(verifiedNetwork);

    const outcome = document.createElement('div');
    outcome.className = 'freeze-result-outcome';
    const badge = document.createElement('span');
    badge.className = 'freeze-result-badge';
    badge.textContent = presentation.label;
    badge.setAttribute('role', 'status');
    outcome.append(badge);
    const inConfigNote = freezeInConfigNote(result.inConf);
    if (inConfigNote) {
      const note = document.createElement('span');
      note.className = `freeze-result-config-note${result.inConf?.state === 'working' ? ' is-working' : ''}`;
      note.textContent = inConfigNote;
      outcome.append(note);
    }
    const facts = freezeRowDetails(result);
    if (facts.details || result.direct || result.current) {
      const disclosure = document.createElement('details');
      disclosure.className = 'freeze-result-details';
      const summary = document.createElement('summary');
      summary.textContent = 'Замеры и детали';
      const content = document.createElement('span');
      content.className = 'freeze-result-detail-content';
      const measurements = freezeMeasurementDetails(freezeMeasurementLabel(result.direct), freezeMeasurementLabel(result.current));
      content.textContent = [measurements, facts.details].filter(Boolean).join('\n');
      disclosure.append(summary, content);
      outcome.append(disclosure);
    }
    const applyKey = `${operation.id}:${resultIndex}`;
    if (!freezeStrategyApplyDone.has(applyKey) && freezeCanApplyToHost({ state: operation.state, targetKind: result.targetKind, freezeDetected: result.freezeDetected, foundStrategy: result.foundStrategy, inConfState: result.inConf?.state })) {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'freeze-action-button';
      button.dataset.actionClick = 'applyFreezeStrategyOneClick';
      button.dataset.args = JSON.stringify([operation.id, String(resultIndex)]);
      button.disabled = freezeStrategyApplyPending.has(applyKey);
      button.textContent = button.disabled ? 'Проверяю и применяю…' : 'Применить к сайту';
      button.title = 'Проверенный вариант добавляется только этому домену; при ошибке конфигурация откатывается';
      outcome.append(button);
    }
    if (freezeCanOfferProfileJoin({ targetKind: result.targetKind, asnVerified: result.asn.state === 'verified', freezeDetected: result.freezeDetected, foundStrategy: result.foundStrategy })) {
      const joinButton = document.createElement('button');
      joinButton.type = 'button';
      joinButton.className = 'freeze-action-button freeze-action-join';
      joinButton.dataset.actionClick = 'previewFreezeJoin';
      joinButton.dataset.args = JSON.stringify([operation.id, String(resultIndex)]);
      joinButton.textContent = 'В профиль';
      joinButton.title = 'Проверить, можно ли безопасно добавить ASN-сеть в существующий профиль';
      outcome.append(joinButton);
    }
    row.append(outcome);
    container.append(row);
  }
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
      await new Promise((resolve) => window.setTimeout(resolve, 2000));
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
  const uploadProbe = $('#freezeScanUploadProbe');
  const probeShortUploads = uploadProbe instanceof HTMLInputElement && uploadProbe.checked;
  if (extraHosts.length > 20) return toast('Можно добавить не более 20 своих доменов.', 'error');
  const results = $('#freezeScanRows');
  results?.replaceChildren();
  $('#freezeScanResults')?.classList.add('hidden');
  $('#freezeApplyPreviewPanel')?.classList.add('hidden');
  activeStrategyPreview = null;
  const applyButton = $('#applyFreezeStrategyButton'); if (applyButton instanceof HTMLButtonElement) applyButton.disabled = true;
  if (results) delete results.dataset.resultsSignature;
  $('#freezeScanSummary')?.classList.add('hidden');
  try { const operation = await nfqwsDiagnostics.freezeScanStart(extraHosts, probeShortUploads); localStorage.setItem('nfqws2-ui-freeze-scan', operation.id); await pollFreezeScan(operation.id); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось запустить проверку сетей', 'error'); }
}

async function cancelFreezeScan() {
  if (!activeFreezeScanId) return;
  const stop = $('#freezeScanCancel'); if (stop instanceof HTMLButtonElement) { stop.disabled = true; stop.textContent = 'Останавливаю…'; }
  try { const operation = await nfqwsDiagnostics.freezeScanCancel(activeFreezeScanId); setText('#freezeScanStatus', operation.message); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось остановить проверку сетей', 'error'); }
  finally { if (stop instanceof HTMLButtonElement) { stop.disabled = false; stop.textContent = 'Остановить'; } }
}

export const DISABLED_PANEL_ACTIONS = ['startStrategyAutoScan','applyStrategyToConf','pushBackupToNas','exportFullBackup','excludeBrokenDomain'] as const;

function appendReleaseRetry(list: HTMLElement) {
  const row = document.createElement('div');
  row.className = 'release-retry-row';
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'release-retry-button';
  button.dataset.actionClick = 'retryUpdateList';
  button.setAttribute('aria-label', 'Повторить загрузку списка версий');
  button.textContent = '↻ Повторить';
  row.append(button);
  list.append(row);
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
    appendReleaseRetry(list);
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
    title.className = 'block text-sm font-medium text-slate-100';
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
  stopUpdatePickerAutoRefresh();
  updateTarget = target;
  expandedReleaseTag = null;
  const modal = $('#updateModalBackdrop');
  if (!modal) return;
  openAppModal(modal, '[data-action-click="closeUpdateModal"]');
  await loadUpdatePickerReleases(false);
  if (modal.dataset.modalOpen !== 'true') return;
  updatePickerRefreshTimer = window.setInterval(() => {
    const currentModal = $('#updateModalBackdrop');
    if (document.visibilityState !== 'visible' || currentModal?.dataset.modalOpen !== 'true' || updatePolling) return;
    void loadUpdatePickerReleases(false, true);
  }, UPDATE_PICKER_REFRESH_INTERVAL_MS);
}

async function loadUpdatePickerReleases(forceRefresh: boolean, preserveView = false) {
  if (updatePickerRequestPending) return;
  updatePickerRequestPending = true;
  const previousSelection = selectedReleaseTag;
  const previousExpansion = expandedReleaseTag;
  const list = $('#updateVersionsList');
  const previousScrollTop = list?.scrollTop ?? 0;
  if (!preserveView && list) list.textContent = 'Загружаю список версий…';
  try {
    if (updateTarget === 'core') {
      const result = await nfqwsUpdates.coreReleases(forceRefresh);
      availableReleases = result.releases;
      installedCoreTag = result.installedTag;
      if (result.warning && !preserveView) toast(result.warning, 'error');
    } else {
      const result = await nfqwsUpdates.releases(forceRefresh);
      availableReleases = result.releases;
      installedCoreTag = null;
      renderUiUpdateNotice(result.updateStatus, !preserveView);
      if (result.warning && !preserveView) toast(`Показан сохранённый список релизов: ${result.warning}`, 'error');
    }
    selectedReleaseTag = preserveView && availableReleases.some((release) => release.tag === previousSelection)
      ? previousSelection
      : availableReleases[0]?.tag ?? null;
    expandedReleaseTag = preserveView && availableReleases.some((release) => release.tag === previousExpansion)
      ? previousExpansion
      : selectedReleaseTag;
    renderReleases();
    if (list) list.scrollTop = previousScrollTop;
  } catch (error) {
    if (preserveView && availableReleases.length) {
      const hint = $('#updatePickerHint');
      if (hint) hint.textContent = 'Не удалось обновить список; показаны ранее загруженные версии.';
      return;
    }
    if (list) {
      list.replaceChildren();
      const message = document.createElement('p');
      message.className = 'px-3 py-6 text-center text-sm text-rose-300';
      message.textContent = error instanceof Error ? error.message : 'Не удалось загрузить версии.';
      list.append(message);
      appendReleaseRetry(list);
    }
    toast('Не удалось получить список версий из GitHub Releases.', 'error');
  } finally {
    updatePickerRequestPending = false;
  }
}

async function installSelectedRelease() {
  if (!selectedReleaseTag || updatePolling) return;
  const tag = selectedReleaseTag;
  let reloading = false;
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
    if (current.state === 'success') { reloading = true; schedulePanelRefreshAfterUpdate(lifecycleMessage(core ? 'core-update' : 'panel-update', tag)); }
    else if (current.state === 'rolled_back') toast(current.message || 'Обновление не прошло проверку; восстановлена предыдущая версия.', 'error');
    else if (current.state === 'failed') toast(current.message || 'Не удалось установить обновление.', 'error');
    else toast('Статус обновления пока не получен. Проверьте панель через несколько секунд.', 'error');
  } catch (error) {
    toast(error instanceof Error ? error.message : 'Не удалось запустить обновление.', 'error');
  } finally {
    if (!reloading) {
      updatePolling = false;
      renderReleases();
    }
  }
}

function updateStrategyMode(mode: string) {
  $('#strategyCustomInputPanel')?.classList.toggle('hidden', mode !== 'custom');
  $('#strategyModeSelect')?.closest('.strategy-mode-tile')?.classList.toggle('strategy-mode-tile-custom', mode === 'custom');
  const help = $('#strategyModeHelp');
  if (!help) return;
  const quic = ($('#strategyProtocolSelect') as HTMLSelectElement | null)?.value === 'quic';
  help.textContent = quic ? ({
    quick: 'Для одного домена выполняются UDP-проверки QUIC Initial: текущий профиль и небольшой стандартный набор. Полный каталог отключён, чтобы не запускать масштабный перебор.',
    custom: 'Проверяется только указанная вами стратегия для QUIC; остальные TCP-настройки профиля не меняются.',
  } as Record<string, string>)[mode] ?? 'Выберите режим тестирования.' : ({
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

async function copyConfigText() {
  const editor = $('#nfqwsConfTextarea');
  if (!(editor instanceof HTMLTextAreaElement)) return toast('Редактор конфигурации недоступен.', 'error');
  try {
    await writeTextToClipboard(editor.value);
    toast('Конфигурация скопирована.');
  } catch {
    toast('Не удалось скопировать конфигурацию. Проверьте разрешение на буфер обмена.', 'error');
  }
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

function showSettingsSection(section: 'appearance' | 'security') {
  const selected = section === 'security' ? 'settingsSecurityPanel' : 'settingsAppearancePanel';
  all<HTMLElement>('.settings-section-panel').forEach((panel) => panel.classList.toggle('hidden', panel.id !== selected));
  all<HTMLButtonElement>('.settings-subtab').forEach((button) => {
    const active = button.dataset.args?.includes(`"${section}"`) ?? false;
    button.setAttribute('aria-pressed', String(active));
    button.classList.toggle('settings-subtab-active', active);
    button.classList.toggle('text-slate-100', active);
    button.classList.toggle('text-slate-400', !active);
  });
  if (section === 'security') void loadPanelAuthSettings();
}

async function loadPanelAuthSettings() {
  try {
    const status = await nfqwsAuth.status();
    const enabled = $('#panelAuthEnabled');
    if (enabled instanceof HTMLInputElement) enabled.checked = status.enabled;
    const rememberDays = $('#panelAuthRememberDays');
    if (rememberDays instanceof HTMLSelectElement) rememberDays.value = String(status.rememberDays);
    $('#panelAuthCurrentWrap')?.classList.toggle('hidden', !status.enabled);
    setText('#panelAuthState', status.enabled ? 'Защита включена' : status.hasPassword ? 'Пароль сохранён, вход пока открыт' : 'Защита выключена');
  } catch (error) { setText('#panelAuthState', error instanceof Error ? error.message : 'Не удалось загрузить настройки'); }
}

async function savePanelAuthSettings() {
  const enabled = $('#panelAuthEnabled'); const password = $('#panelAuthPassword'); const current = $('#panelAuthCurrentPassword');
  const rememberDays = $('#panelAuthRememberDays');
  if (!(enabled instanceof HTMLInputElement) || !(password instanceof HTMLInputElement) || !(current instanceof HTMLInputElement) || !(rememberDays instanceof HTMLSelectElement)) return;
  const button = $('[data-action-click="savePanelAuthSettings"]');
  if (button instanceof HTMLButtonElement) { button.disabled = true; button.textContent = 'Сохраняю…'; }
  try {
    await nfqwsAuth.save(enabled.checked, password.value, current.value, Number(rememberDays.value));
    password.value = ''; current.value = '';
    setText('#panelAuthFeedback', 'Настройки входа сохранены.');
    toast('Настройки входа сохранены.');
    await loadPanelAuthSettings();
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Не удалось сохранить настройки входа.';
    setText('#panelAuthFeedback', message); toast(message, 'error');
  } finally { if (button instanceof HTMLButtonElement) { button.disabled = false; button.textContent = 'Сохранить настройки'; } }
}

async function logoutPanel() {
  try { await nfqwsAuth.logout(); window.location.reload(); }
  catch (error) { toast(error instanceof Error ? error.message : 'Не удалось выйти.', 'error'); }
}

function bindPrototypeActions() {
  $('#operationConfirmCancel')?.addEventListener('click', () => closeDangerousConfirmation());
  $('#operationConfirmContinue')?.addEventListener('click', () => void confirmDangerousOperation());
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
    openAppModal(modal, '[data-action-click="closeDiffModal"]');
  };
  const handlers: Record<string, (...args: any[]) => unknown> = {
    switchMainTab: (route: Route) => showRoute(route),
    switchDpiLabTab: (tab: 'visualizer' | 'live' | 'pcap') => switchDpiLabTab(tab),
    visualizeDpiStrategy: () => void visualizeDpiStrategy(),
    runDpiLiveStrategyTest: () => void runDpiLiveStrategyTest(),
    analyzeDpiCapture: () => void analyzeDpiCapture(),
    saveDpiExperiment: () => void saveDpiExperiment(),
    refreshDpiExperiments: () => void loadDpiExperiments(),
    showSettingsSection: (section: 'appearance' | 'security') => showSettingsSection(section),
    savePanelAuthSettings: () => void savePanelAuthSettings(),
    logoutPanel: () => void logoutPanel(),
    createBackup: () => void createSnapshot(),
    createSnapshot: () => void createSnapshot(),
    deleteSnapshot: (id: string) => void deleteSnapshot(id),
    refreshBackups: () => void loadBackups(),
    restoreSnapshot: (id: string) => void restoreSnapshot(id),
    pinSnapshot: (id: string, pinned: string | boolean) => void (async () => { try { await nfqwsSnapshots.pin(id, pinned === true || pinned === 'true'); await loadBackups(); } catch (error) { toast(error instanceof Error ? error.message : 'Не удалось закрепить снимок.', 'error'); } })(),
    addRuntimeProfile: () => addRuntimeProfile(),
    shareRuntimeProfile: (id: string) => void shareRuntimeProfile(id),
    copyRuntimeProfile: (id: string) => copyRuntimeProfile(id),
    pasteRuntimeProfile: () => pasteRuntimeProfile(),
    acceptProfileImport: () => acceptProfileImport(),
    cancelProfileImport: () => cancelProfileImport(),
    selectRuntimeProfile: (id: string) => selectRuntimeProfile(id),
    setRuntimeProfileMode: (mode: string) => setRuntimeProfileMode(mode),
    appendRuntimeProfileArgument: (argument: string) => appendRuntimeProfileArgument(argument),
    setRuntimeProfileStepParameters: (id: string, index: string, parameters: string) => updateRuntimeProfileStep(id, index, (argument) => replaceDesyncParameters(argument, parameters)),
    setRuntimeProfileStepParameter: (id: string, index: string, parameterIndex: string, value: string) => updateRuntimeProfileStepParameter(id, index, parameterIndex, value),
    removeRuntimeProfileStepParameter: (id: string, index: string, parameterIndex: string) => removeRuntimeProfileStepParameter(id, index, parameterIndex),
    renameRuntimeProfile: (id: string, name: string) => renameRuntimeProfile(id, name),
    updateRuntimeProfileArgument: (id: string, index: string, value: string) => updateRuntimeProfileArgument(id, index, value),
    moveRuntimeProfileStep: (id: string, index: string, direction: string) => moveRuntimeProfileStep(id, index, direction),
    removeRuntimeProfileArgument: (id: string, index: string) => removeRuntimeProfileArgument(id, index),
    moveRuntimeProfileUp: (id: string) => mutateRuntimeProfile(id, 'up'),
    moveRuntimeProfileDown: (id: string) => mutateRuntimeProfile(id, 'down'),
    deleteRuntimeProfile: (id: string) => mutateRuntimeProfile(id, 'delete'),
    saveRuntimeProfiles: () => void saveRuntimeProfiles(),
    refreshProfiles: () => void loadProfiles(),
    refreshChangeHistory: () => void loadChangeHistory($<HTMLInputElement>('#changeHistorySearch')?.value ?? ''),
    openCommunityShare: (operationId: string, candidateIndex: string) => void openCommunityShare(operationId, Number(candidateIndex)),
    submitCommunityShare: () => submitCommunityShare(),
    closeCommunityShare: () => { const modal = $('#communityShareModal'); if (modal) closeAppModal(modal); clearCommunityShare(); },
    editCommunityProvider: () => { communityProviderEditing = true; if (communityStrategies) renderCommunityStrategies(communityStrategies); $('#communityProviderAsn')?.focus(); },
    cancelCommunityProviderEdit: () => { communityProviderEditing = false; if (communityStrategies) renderCommunityStrategies(communityStrategies); },
    detectCommunityProvider: () => void loadCommunityStrategies(true, true),
    refreshCommunityStrategies: () => void loadCommunityStrategies(false, true),
    saveCommunityProvider: () => void saveCommunityProvider(),
    testCommunityStrategy: (id: string) => void testCommunityStrategy(id),
    copyCommunityStrategy: (id: string) => void copyCommunityStrategy(id),
    filterChangeHistory: () => renderChangeHistory($<HTMLInputElement>('#changeHistorySearch')?.value ?? ''),
    toggleChangeHistoryDiff: (id: string, button: HTMLButtonElement) => void toggleChangeHistoryDiff(id, button),
    requestRevertChangeHistory: (id: string) => requestRevertChangeHistory(id),
    requestUndoLastChange: () => requestUndoLastChange(),
    requestRestoreInitialHistory: () => requestRestoreInitialHistory(),
    refreshLogs: () => void loadLogs(),
    filterLogs: () => renderLogs(),
    changeLogSource: (source: LogSource) => { currentLogSource = source === 'nfqws2' ? 'nfqws2' : 'nfqws2-ui'; logRequestPending = false; void loadLogs(); },
    toggleLogAutoRefresh: (enabled: boolean) => { setText('#logsAutoRefreshStatus', enabled ? 'Автообновление каждые 5 секунд' : 'Автообновление приостановлено'); },
    downloadLogs: () => downloadLogs(),
    clearLogs: () => clearLogs(),
    saveSnapshotRetention: () => void saveSnapshotSettings().catch((error) => toast(error instanceof Error ? error.message : 'Не удалось сохранить срок хранения.', 'error')),
    toggleAutomaticSnapshots: (enabled: boolean) => void toggleAutomaticSnapshots(enabled),
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
    setAppearanceConfigWarnings: (showConfigWarnings: boolean) => updateAppearance({ showConfigWarnings }),
    switchSimplifiedNavGroup: (group: string) => { const routes: Record<string, Route> = { configuration: 'conf', 'site-check': 'check', tuning: 'autotune', monitoring: 'monitor', lists: 'lists', system: 'appearance' }; const route = routes[group]; if (route) showRoute(route); },
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
      void persistMonitoring();
    },
    switchConfigSection: (section: 'editor' | 'settings' | 'scripts') => switchConfigSection(section),
    refreshLuaScripts: () => void loadLuaScripts(),
    runSiteDiagnosis: () => void runSiteDiagnosis(),
    runSiteTrace: () => void runSiteTrace(),
    stopSiteTrace: () => void stopSiteTrace(),
    openForeignConfigText: () => showForeignConfigForm('text'),
    openForeignConfigUrl: () => showForeignConfigForm('url'),
    openForeignConfigFile: () => openForeignConfigFile(),
    closeForeignConfigForms: () => closeForeignConfigForms(),
    addForeignConfigText: () => addForeignConfigText(),
    addForeignConfigUrl: () => addForeignConfigUrl(),
    runForeignConfigCheck: () => void runForeignConfigCheck(),
    cancelForeignConfigCheck: () => void cancelForeignConfigCheck(),
    startStrategyPick: () => void startStrategyPick(),
    setStrategyTargetMode: (mode: string) => setStrategyTargetMode(mode),
    pickRandomMultiSites: () => void pickRandomMultiSites(),
    cancelStrategyPick: () => void cancelStrategyPick(),
    toggleStrategyCustomInput: (mode: string) => updateStrategyMode(mode),
    updateStrategyProtocol: (protocol: string) => updateStrategyProtocol(protocol),
    copyStrategyParams: (button: HTMLButtonElement) => {
      const input = $('#strategyCustomSteps');
      if (!(input instanceof HTMLTextAreaElement)) return;
      copyStrategyText(strategyCustomText(), button);
    },
    copyStrategyResult: (text: string, button: HTMLButtonElement) => copyStrategyText(text, button),
    previewStrategyPick: (index: string) => void previewStrategyPick(index),
    previewFreezeStrategy: (operationId: string, index: string) => void previewFreezeStrategy(operationId, index),
    previewFreezeJoin: (operationId: string, index: string) => void previewFreezeJoin(operationId, index),
    applyFreezeJoin: (operationId: string, index: string, revision: string) => void applyFreezeJoin(operationId, index, revision),
    closeFreezeJoinPreview: () => closeFreezeJoinPreview(),
    applyFreezeStrategyOneClick: (operationId: string, index: string) => void applyFreezeStrategyOneClick(operationId, index),
    applyStrategyPick: (source: string, operationId: string, index: string, revision: string) => void applyStrategyPick(source, operationId, index, revision),
    runFreezeScan: () => void startFreezeScan(),
    cancelFreezeScan: () => void cancelFreezeScan(),
    setDiagPreset: (host: string) => { const input = $('#diagDomainInput'); if (input instanceof HTMLInputElement && !diagnosisRunning) { input.value = host; input.focus(); } },
    restartService: () => void serviceAction('restart'),
    toggleDaemonPower: () => void serviceAction(serviceControlPresentation(status?.service.state).action),
    applyConfigAndRestart: () => void saveConfigAndApply(),
    requestClearConfig: () => setConfigClearModal(true),
    cancelClearConfig: () => setConfigClearModal(false),
    confirmClearConfig: () => confirmClearConfig(),
    requestClearStrategyPickHistory: () => setStrategyPickHistoryClearModal(true),
    cancelClearStrategyPickHistory: () => setStrategyPickHistoryClearModal(false),
    confirmClearStrategyPickHistory: () => void clearStrategyPickHistory(),
    toggleConfigEditorFullscreen: () => toggleConfigEditorFullscreen(),
    requestClearListDraft: () => setListClearModal(true),
    cancelClearListDraft: () => setListClearModal(false),
    confirmClearListDraft: () => confirmClearListDraft(),
    toggleListEditorFullscreen: () => toggleListEditorFullscreen(),
    copyConfigText: () => void copyConfigText(),
    updateConfVariable: (key: string, value: string) => void updateConfigVariable(key, value),
    selectIspInterface: (value: string) => handleIspInterfaceSelection(value),
    submitIspInterfaceManually: () => submitIspInterfaceManually(),
    updateIspInterfaceManually: (value: string) => void updateIspInterfaceManually(value),
    copySnippetToClipboard: (value: string | HTMLButtonElement) => void (async () => {
      const button = value instanceof HTMLButtonElement ? value : null;
      const command = button?.closest('.about-command-row')?.querySelector('code')?.textContent?.trim();
      const text = typeof value === 'string' ? value : command;
      if (!text) return toast('Команда не найдена.', 'error');
      const label = button?.querySelector<HTMLElement>('[data-about-copy-label="true"]');
      const copyStatus = $('#aboutCopyStatus');
      const originalLabel = label?.textContent ?? 'Копировать';
      if (button) {
        button.disabled = true;
        if (label) label.textContent = 'Копирую…';
        button.setAttribute('aria-label', 'Копирование команды');
        button.title = 'Копирование команды';
        button.classList.remove('about-copy-success');
      }
      try {
        await writeTextToClipboard(text);
        if (button) {
          button.disabled = false;
          if (label) label.textContent = 'Скопировано';
          button.setAttribute('aria-label', 'Команда скопирована');
          button.title = 'Команда скопирована';
          button.classList.add('about-copy-success');
          if (copyStatus) copyStatus.textContent = 'Команда скопирована в буфер обмена.';
          window.setTimeout(() => {
            if (!button.isConnected) return;
            if (label) label.textContent = originalLabel;
            button.setAttribute('aria-label', 'Копировать команду');
            button.title = 'Копировать команду';
            button.classList.remove('about-copy-success');
            if (copyStatus) copyStatus.textContent = '';
          }, 1800);
        } else toast('Фрагмент скопирован');
      } catch {
        if (button) {
          button.disabled = false;
          if (label) label.textContent = originalLabel;
          button.setAttribute('aria-label', 'Копировать команду');
          button.title = 'Копировать команду';
        }
        toast('Буфер обмена недоступен', 'error');
      }
    })(),
    handleConfInput: () => {
      syncConfigLines();
      renderConfigOutline();
      scheduleConfigWarnings();
      const editor = $('#nfqwsConfTextarea');
      if (editor instanceof HTMLTextAreaElement) updateQuickParameters(editor.value, discoveredNetworkInterfaces, defaultNetworkInterfaces);
    },
    toggleWordWrapConf: (enabled: boolean) => {
      let storage: Storage | null = null; try { storage = window.localStorage; } catch { /* Keep this session's choice if browser storage is blocked. */ }
      appearanceSettings = saveAppearance(storage, { ...appearanceSettings, wrapConfigLines: enabled });
      syncConfigWordWrap(enabled);
    },
    toggleConfigOutline: () => toggleConfigOutline(),
    scrollToConfMarker: (marker: string) => { const editor = $('#nfqwsConfTextarea'); if (!(editor instanceof HTMLTextAreaElement)) return; const offset = editor.value.toLowerCase().indexOf(String(marker).toLowerCase()); if (offset < 0) return toast(`Секция ${marker} не найдена`, 'error'); const line = editor.value.slice(0, offset).split('\n').length; selectConfigText(offset, offset, line); },
    toggleDiffModal: () => { const modal = $('#diffModalBackdrop'); if (modal && !modal.classList.contains('hidden')) closeAppModal(modal); else showDiff(); },
    resetDefaultConf: () => void loadConfig(),
    closeUpdateModal: () => closeUpdateModal(),
    closeDiffModal: () => closeAppModal($('#diffModalBackdrop')),
    openUpdateModal: (target: 'ui' | 'core') => void openUpdatePicker(target),
    retryUpdateList: () => void loadUpdatePickerReleases(true),
    openLatestUiUpdate: (tag: string) => { uiUpdateNoticeOpenedTag = tag; hideUiUpdateNotice(); void openUpdatePicker('ui'); },
    dismissUiUpdateNotice: (tag: string) => dismissUiUpdateNotice(tag),
    dismissCoreUpdateNotice: (tag: string) => { try { window.localStorage.setItem(CORE_UPDATE_DISMISSED_TAG_KEY, tag); } catch { /* The notice can still be closed for this page session. */ } hideCoreUpdateNotice(); },
    selectVersionItem: (tag: string) => { if (availableReleases.some((release) => release.tag === tag) && !updatePolling) { const toggling = expandedReleaseTag === tag; selectedReleaseTag = tag; expandedReleaseTag = toggling ? null : tag; renderReleases(); } },
    installSelectedRelease: () => void installSelectedRelease(),
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
  const strategyCustomState = $('#strategyCustomSteps');
  const strategyCustomHost = $('#strategyCustomStepsEditor');
  if (strategyCustomState instanceof HTMLTextAreaElement && strategyCustomHost instanceof HTMLElement) {
    strategyCustomCodeEditor = mountConfigArgumentEditor(strategyCustomHost, strategyCustomState.value, (value) => {
      strategyCustomState.value = value;
    }, 'Параметры своей стратегии', 'одна стратегия на строку, например: --lua-desync=multisplit:pos=1,midsld');
  }
  const configState = $('#nfqwsConfTextarea');
  const configHost = $('#configCodeEditor');
  if (configState instanceof HTMLTextAreaElement && configHost instanceof HTMLElement) {
    mountConfigCodeEditor(configHost, configState.value, (value) => {
      configState.value = value;
      configState.dispatchEvent(new Event('input', { bubbles: true }));
    });
    setConfigCodeEditorWrapping(appearanceSettings.wrapConfigLines !== false);
    configHost.addEventListener('configselection', updateActiveConfigOutline);
  }
  setListEditorView(listEditorView);
  installStyledSelects(document);
  installActionTooltips(document);
  monitoringState = loadMonitoringState(getMonitoringStorage());
  renderMonitoringSites();
  void loadBackendMonitoring().catch(() => setText('#monitoringStatus', 'Не удалось загрузить настройки мониторинга с роутера.'));
  void loadPanelAuthSettings();
  void loadTelegramSettings().catch(() => setText('#telegramStatus', 'Не удалось загрузить настройки Telegram с роутера.'));
  window.setInterval(() => {
    if (document.visibilityState === 'visible' && $('#tabContent-monitor:not(.hidden) #monitorSection-telegram:not(.hidden)')) {
      void loadTelegramNotifications().catch(() => {});
    }
  }, 30_000);
  bindPrototypeActions();
  const dpiCaptureInput = $('#dpiLabCaptureFile');
  dpiCaptureInput?.addEventListener('change', () => {
    if (!(dpiCaptureInput instanceof HTMLInputElement)) return;
    const file = dpiCaptureInput.files?.[0];
    syncDpiCaptureControls();
    const availability = dpiCaptureAvailability(file?.size ?? null, dpiCaptureAnalyzing);
    setText('#dpiLabStatus', availability.disabled ? availability.reason : `${file!.name} · ${Math.ceil(file!.size / 1024)} КиБ · нажмите «Анализировать PCAP».`);
  });
  syncDpiCaptureControls();
  const dpiStrategyProfile = $('#dpiLabProfileSelect');
  $('#dpiLabStrategySource')?.addEventListener('input', () => {
    invalidateDpiStrategyAnalysis();
  });
  dpiStrategyProfile?.addEventListener('change', selectDpiStrategyProfile);
  const dpiStrategyFile = $('#dpiLabStrategyFile');
  dpiStrategyFile?.addEventListener('change', () => {
    if (!(dpiStrategyFile instanceof HTMLInputElement)) return;
    const file = dpiStrategyFile.files?.[0];
    const source = $('#dpiLabStrategySource');
    if (!file || !(source instanceof HTMLTextAreaElement)) return;
    if (!/\.(conf|txt|lua)$/i.test(file.name)) { setText('#dpiLabStrategyStatus', 'Допустимы только текстовые файлы .conf, .txt и .lua.'); dpiStrategyFile.value = ''; return; }
    if (file.size > 256 * 1024) { setText('#dpiLabStrategyStatus', 'Файл превышает лимит 256 КиБ.'); dpiStrategyFile.value = ''; return; }
    void file.text().then((text) => {
      source.value = text;
      invalidateDpiStrategyAnalysis();
      setText('#dpiLabStrategyStatus', `Загружен ${file.name} · ${file.size} байт. Содержимое рассматривается только как текст и не исполняется.`);
    }).catch(() => setText('#dpiLabStrategyStatus', 'Не удалось прочитать текстовый файл.'));
  });
  const foreignFileInput = $('#foreignConfigFileInput');
  foreignFileInput?.addEventListener('change', () => {
    if (!(foreignFileInput instanceof HTMLInputElement)) return;
    const files = Array.from(foreignFileInput.files ?? []);
    const remaining = Math.max(0, 10 - foreignConfigInputs.length);
    void (async () => {
      for (const file of files.slice(0, remaining)) await addForeignConfigFile(file);
      if (files.length > remaining) toast(`За один запуск можно сравнить до 10 конфигов. Добавлено ${remaining} из выбранных файлов.`, 'error');
    })();
    foreignFileInput.value = '';
  });
  renderForeignConfigInputs();
  void loadStrategyDomainLists();
  document.documentElement.classList.add('dark');
  const initialStrategyMode = ($('#strategyModeSelect') as HTMLSelectElement | null)?.value ?? 'quick';
  updateStrategyMode(initialStrategyMode);
  updateStrategyProtocol(($<HTMLSelectElement>('#strategyProtocolSelect'))?.value ?? 'https');
  setStrategyTargetMode('single');
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
  $('#ispInterfaceManualInput')?.addEventListener('keydown', (event) => {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    submitIspInterfaceManually();
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
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Tab') { trapAppModalFocus(event); return; }
    if (event.key === 'Escape' && document.querySelector('[data-app-modal-root][data-modal-open="true"]')) { trapAppModalFocus(event); return; }
    if (event.key === 'Escape' && $('#listEditorPanel.list-editor-fullscreen')) toggleListEditorFullscreen();
  });
  document.addEventListener('click', closeAppModalOnBackdrop);
  $('#refreshStatusBtn')?.addEventListener('click', () => void fetchStatus());
  void fetchStatus();
  void refreshUiUpdateStatus();
  window.setInterval(() => { if (document.visibilityState === 'visible') void refreshUiUpdateStatus(); }, UPDATE_STATUS_POLL_INTERVAL_MS);
  document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') void refreshUiUpdateStatus(); });
  void refreshCoreUpdateStatus();
  window.setInterval(() => { if (document.visibilityState === 'visible') void refreshCoreUpdateStatus(); }, CORE_UPDATE_CHECK_INTERVAL_MS);
  document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') void refreshCoreUpdateStatus(); });
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
