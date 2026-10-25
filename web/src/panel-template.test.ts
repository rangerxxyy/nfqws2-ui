import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import App from './App';

const template = renderToStaticMarkup(React.createElement(App));
const panelSource = readFileSync(new URL('./panel.ts', import.meta.url), 'utf8');
const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');
const panelStyles = readFileSync(new URL('./panel.css', import.meta.url), 'utf8');
const styledSelectSource = readFileSync(new URL('./styled-select.ts', import.meta.url), 'utf8');
const monitoringSource = readFileSync(new URL('./monitoring.ts', import.meta.url), 'utf8');
const actionTooltipSource = readFileSync(new URL('./action-tooltips.ts', import.meta.url), 'utf8');
const viteSource = readFileSync(new URL('../vite.config.ts', import.meta.url), 'utf8');
const entrySource = readFileSync(new URL('./main.tsx', import.meta.url), 'utf8');
const backendSource = readFileSync(new URL('../../backend/src/main.rs', import.meta.url), 'utf8');

describe('TSX panel with optional server-side authentication', () => {
  it('opens the full configuration editor first and separates configuration settings', () => {
    expect(template).toContain('id="configTabEditor"');
    expect(template).toContain('aria-pressed="true"');
    expect(template).toContain('id="configTabSettings"');
    expect(template).toContain('id="configEditorPanel"');
    expect(template).toContain('id="configSettingsPanel"');
    expect(template).toMatch(/id="configEditorPanel" class="[^"]*config-panel-enter/);
    expect(template).toMatch(/id="configSettingsPanel" class="[^"]*hidden/);
    expect(template).toContain('id="configTabScripts"');
    expect(template).toMatch(/id="configScriptsPanel" class="[^"]*hidden/);
    expect(template.indexOf('data-action-click="applyConfigAndRestart"')).toBeLessThan(template.indexOf('id="configSettingsPanel"'));
    expect(panelSource).toContain('switchConfigSection:');
    expect(panelSource).toContain("panel.classList.toggle('hidden', id !== selectedId)");
    expect(panelSource).toContain("if (section === 'scripts') void loadLuaScripts()");
    expect(panelSource).toContain("button.setAttribute('aria-pressed', String(selected))");
    expect(panelStyles).toContain('.config-subtab-active');
    const configSection = template.slice(template.indexOf('id="tabContent-conf"'), template.indexOf('id="tabContent-lists"'));
    expect(configSection).not.toContain('formatConfigButton');
    expect(configSection).not.toContain('Оформить');
    expect(template).toContain('overflow-y-auto p-4 lg:p-6');
    expect(configSection).toContain('mobile-code-editor flex h-[740px] min-w-0 bg-[#060a12] font-mono text-[14px] leading-[1.68]');
    expect(configSection).toContain('line-numbers-col bg-[#05080f] border-r border-slate-800/80 py-4 px-3');
    expect(configSection).toContain('conf-syntax-highlight absolute inset-0 m-0 overflow-hidden py-4 px-5 font-mono leading-[1.68]');
    expect(configSection).toContain('conf-editor-input absolute inset-0 min-w-0 w-full h-full bg-transparent py-4 px-5 outline-none resize-none leading-[1.68] font-mono');
  });

  it('provides safe clear and animated near-fullscreen controls for the config editor', () => {
    expect(template).toContain('data-action-click="requestClearConfig"');
    expect(template).toContain('aria-label="Очистить конфигурацию"');
    expect(template).toContain('id="configClearModal"');
    expect(template).toContain('role="dialog" aria-modal="true"');
    expect(template).toContain('data-action-click="toggleConfigEditorFullscreen"');
    expect(template).toContain('aria-label="Развернуть редактор"');
    expect(panelSource).toContain('requestClearConfig:');
    expect(panelSource).toContain('toggleConfigEditorFullscreen:');
    expect(panelSource).toContain('document.body.append(panel)');
    expect(panelSource).toContain('fullscreenPlaceholder');
    expect(panelStyles).toContain('.config-editor-fullscreen');
    expect(panelStyles).toContain('prefers-reduced-motion: reduce');
  });

  it('shows the six backend-discovered Lua scripts in a dedicated read-only section', () => {
    const tabs = ['configTabEditor', 'configTabSettings', 'configTabScripts'].map((id) => template.indexOf(`id="${id}"`));
    expect(tabs[0]).toBeLessThan(tabs[1]);
    expect(tabs[1]).toBeLessThan(tabs[2]);
    expect(template).toContain('id="luaScriptTabs"');
    expect(template).toContain('id="luaScriptPath"');
    expect(template).toContain('id="luaScriptContent"');
    expect(panelSource).toContain('nfqwsScripts.list()');
    expect(panelSource).toContain('nfqwsScripts.get(name)');
    expect(backendSource).toContain('.route("/api/nfqws2/scripts", get(get_lua_scripts))');
    expect(backendSource).toContain('.route("/api/nfqws2/scripts/{name}", get(get_lua_script))');
    for (const name of ['zapret-antidpi.lua', 'zapret-auto.lua', 'zapret-lib.lua', 'zapret-obfs.lua', 'zapret-pcap.lua', 'zapret-tests.lua']) {
      expect(backendSource).toContain(`"${name}"`);
    }
    expect(backendSource).toContain('MAX_SCRIPT_BYTES: u64 = 512 * 1024');
  });

  it('numbers each editable config argument block without padding short values', () => {
    expect(panelSource).toContain('configEditorLayout(field.value, minimumRows)');
    expect(panelSource).toContain('gutter.textContent = layout.lineNumbers');
    expect(panelSource).toContain('field.rows = layout.visibleRows');
    expect(panelSource).toContain('gutter.style.maxHeight = viewportHeight');
    expect(panelSource).toContain('field.style.maxHeight = viewportHeight');
    expect(panelStyles).toContain('.config-argument-line-numbers');
  });

  it('validates and applies the same immutable configuration draft', () => {
    const apply = panelSource.slice(panelSource.indexOf('async function saveConfigAndApply()'), panelSource.indexOf('async function serviceAction('));
    expect(apply).toContain('const draft = editor.value;');
    expect(apply).toContain('nfqwsConfig.validate(draft)');
    expect(apply).toContain('if (editor.value !== draft)');
    expect(apply).toContain('nfqwsApply.apply(draft, config.revision)');
  });

  it('uses normal case for left navigation group headings', () => {
    const headings = [...template.matchAll(/<div class="px-3 mb-2 text-\[11px\] font-mono font-bold ([^"]+)">/g)];
    expect(headings).toHaveLength(2);
    expect(headings.every((heading) => !heading[1]?.includes('uppercase'))).toBe(true);
  });

  it('installs a shared styled menu for all native selects and preserves change behavior', () => {
    expect([...template.matchAll(/<select\b/g)]).toHaveLength(15);
    expect(panelSource).toContain('installStyledSelects(document)');
    expect(styledSelectSource).toContain("setAttribute('role', 'combobox')");
    expect(styledSelectSource).toContain("setAttribute('role', 'listbox')");
    expect(styledSelectSource).toContain("new Event('change', { bubbles: true })");
    expect(styledSelectSource).toContain('MutationObserver');
  });

  it('keeps explanatory and data text readable across panel sections and themes', () => {
    expect(panelStyles).toContain('font-size: max(0.75rem, 1em);');
    expect(panelStyles).toContain('html[data-theme="light"] { color-scheme: light; }');
    expect(panelStyles).toContain('backdrop-filter: blur(14px);');
    expect(panelStyles).toContain('animation: styled-select-in 150ms var(--ease-out) both;');
    expect(panelStyles).toContain('prefers-reduced-motion: reduce');
    expect(panelStyles).toContain('[data-theme="dark"] [class*="text-purple-"]');
    expect(panelStyles).toContain('[data-theme="light"] [class*="bg-purple-600"]');
  });

  it('exposes the full blockcheck2 catalog mode and submits it to the Rust API', () => {
    expect(template).toContain('value="full"');
    expect(template).toContain('каталог blockcheck2 (~3 000)');
    expect(template).toContain('id="strategyModeHelp"');
    expect(panelSource).toContain("['extended', 'full', 'custom']");
    expect(panelSource).toContain('Проверка идёт по одному, занимает дольше');
    expect(panelStyles).toContain('.strategy-mode-help.mode-help-enter');
    expect(panelSource).toContain('Проверено ${operation.done} из ${operation.total} стратегий');
    expect(panelSource).toContain("'Рабочая стратегия не подтверждена'");
    expect(backendSource).toContain('"full" => Some(StrategyMode::Full)');
  });

  it('keeps the requested configuration and list toolbar free of removed labels', () => {
    expect(template).toContain('Конфигурация');
    expect(template).not.toContain('Секции:');
    expect(template).not.toContain('Проверка не выполнялась');
    expect(template).not.toContain('Сравнить изменения');
    expect(template).not.toContain('Добавить в черновик');
    expect(template).not.toContain('id="currentListDescription"');
    expect(template).toContain('flex-nowrap whitespace-nowrap');
    expect(template).toContain('data-action-click="runSiteDiagnosis"');
    expect(template).not.toContain('Ломается профилем #1');
    const disabledActions = panelSource.match(/export const DISABLED_PANEL_ACTIONS = \[([\s\S]*?)\] as const/);
    expect(disabledActions?.[1]).not.toContain('runSiteDiagnosis');
    expect(disabledActions?.[1]).not.toContain('setDiagPreset');
  });
  it('places service controls beside status and orders updates core left, UI right', () => {
    const statusGroup = template.slice(template.indexOf('id="headerStatusBar"'), template.indexOf('id="headerUpdates"'));
    expect(statusGroup).toContain('id="headerServiceControls"');
    expect(statusGroup).toContain('data-action-click="restartService"');
    expect(statusGroup).toContain('aria-label="Перезапустить nfqws2"');
    expect(statusGroup).toContain('id="togglePowerBtn"');
    expect(statusGroup).toContain('aria-label="Запустить nfqws2"');
    expect(statusGroup).toContain('data-power-icon="play"');
    expect(panelSource).toContain('button.dataset.serviceAction = presentation.action');
    expect(statusGroup).toContain('id="togglePowerText" class="sr-only"');
    expect(statusGroup).toContain('h-9 w-9');
    expect(statusGroup).toContain('px-3 py-2 rounded-lg shadow-inner');
    expect(statusGroup).not.toContain('Перезапустить</span>');
    const updates = template.slice(template.indexOf('id="headerUpdates"'), template.indexOf('</header>'));
    expect(updates).not.toContain('restartService');
    expect(updates.indexOf('id="updateNfqws2Button"')).toBeLessThan(updates.indexOf('id="updateUiButton"'));
    expect(updates).toMatch(/id="updateNfqws2Button"[^>]*data-args="\[&quot;core&quot;\]"/);
    expect(updates).toMatch(/id="updateUiButton"[^>]*data-args="\[&quot;ui&quot;\]"/);
    expect(updates).toContain('>UI</span>');
  });

  it('starts with a play icon and updates the service control presentation from status', () => {
    expect(template).toContain('data-power-icon="play"');
    expect(template).toContain('data-power-icon="stop"');
    expect(panelSource).toContain('button.dataset.serviceAction = presentation.action');
  });

  it('centers the panel brand and aligns update controls to the right with balanced spacing', () => {
    expect(template).toContain('lg:grid-cols-[repeat(3,minmax(0,1fr))]');
    expect(template).toContain('2xl:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]');
    expect(template).toContain('lg:col-span-3 2xl:col-span-1');
    expect(template).toContain('text-xl lg:text-2xl');
    const updates = template.slice(template.indexOf('id="headerUpdates"'), template.indexOf('</header>'));
    expect(updates).toContain('lg:justify-end');
    expect(updates).toContain('gap-3');
    expect(updates).not.toContain('lg:pr-8');
  });

  it('widens memory and CPU meters and spaces version controls on the right', () => {
    expect(template).toContain('min-w-[150px]');
    expect(template).toContain('min-w-[108px]');
    expect(template).toContain('w-28 sm:w-32');
    expect(template).toContain('w-16 sm:w-20');
    const updates = template.slice(template.indexOf('id="headerUpdates"'), template.indexOf('</header>'));
    expect(updates).toContain('lg:justify-end');
    expect(updates).toContain('gap-3');
  });

  it('renders returned HTTP codes and animates diagnostic cards respectfully', () => {
    expect(template).toContain('class="diagnosis-results"');
    expect(template).toContain('Диагноз блокировки');
    expect(panelSource).toContain('item.metrics');
    expect(panelSource).toContain('diagnosis-row');
    expect(panelStyles).toContain('@media (prefers-reduced-motion: reduce)');
    expect(panelStyles).toContain('.diag-result-card');
  });

  it('adds a compact text, link, and file workflow for comparing external nfqws2 configs', () => {
    const section = template.slice(template.indexOf('id="tabContent-configCheck"'), template.indexOf('id="tabContent-monitor"'));
    expect(template).toContain('id="navBtn-configCheck"');
    expect(section).toContain('id="foreignConfigTextContent"');
    expect(section).toContain('id="foreignConfigUrl"');
    expect(section).toContain('id="foreignConfigFileInput"');
    expect(section).toContain('data-action-click="openForeignConfigFile"');
    expect(section).not.toContain('＋ Текстом');
    expect(section).not.toContain('＋ По ссылке');
    expect(section).toContain('Файл .conf/.txt');
    expect(section).toContain('accept=".conf,.txt,text/plain"');
    expect(section).toContain('файлом .conf/.txt');
    expect(section).toContain('id="foreignConfigHosts"');
    expect(section).toContain('id="foreignConfigRepeats"');
    expect(section).toContain('id="foreignConfigResults"');
    expect(section).toContain('до 10 вариантов');
    expect(section).toContain('(1–15, домен или публичный IPv4)');
    expect(panelSource).toContain('foreignConfigInputs.length >= 10');
    expect(panelSource).toContain('isSupportedForeignConfigFileName(file.name)');
    expect(panelSource).toContain('hosts.length > 15');
    expect(section).toContain('data-action-click="runForeignConfigCheck"');
    expect(panelSource).toContain('nfqwsDiagnostics.foreignConfigStart');
    expect(panelSource).toContain('nfqwsDiagnostics.foreignConfigStatus');
    expect(panelSource).toContain('nfqwsDiagnostics.foreignConfigCancel');
    expect(panelSource).toContain('Конфигурация работает');
    expect(panelSource).toContain('Текущая конфигурация');
    expect(panelSource).toContain('Проверяемая конфигурация');
    expect(panelSource).toContain("document.createElement('details')");
    expect(panelSource).toContain("chevron.className = 'details-chevron'");
    expect(panelStyles).toContain('details[open] > summary .details-chevron');
    expect(panelSource).toContain("sm:grid-cols-[minmax(0,33%)_auto_minmax(0,1fr)]");
    expect(panelSource).toContain('col-span-2 justify-self-end text-right font-semibold sm:col-span-1');
    expect(panelSource).toContain("attemptLabel.className = 'inline-flex w-fit items-center gap-1.5");
    expect(panelSource).toContain("latency.className = 'text-teal-200'");
    expect(panelSource).toContain("details.className = 'ui-card min-w-0 self-start h-fit rounded-xl border border-slate-800 p-3'");
    expect(panelSource).toContain('`Домен: ${foreignStatusLabel(result)}`');
    expect(panelSource).toContain('nfqwsDiagnostics.foreignConfigApply');
    expect(backendSource).toContain('MAX_FOREIGN_CONFIGS');
    expect(backendSource).toContain('MAX_FOREIGN_CONFIG_HOSTS');
    expect(backendSource).toContain('parse_public_https_config_url');
    expect(backendSource).toContain('external_config_test_args');
    expect(backendSource).toContain('NFQWS_BASE_ARGS');
  });

  it('preserves each comparison disclosure while polling and exposes repeat timing per site', () => {
    expect(panelSource).toContain('captureForeignConfigDisclosureState(results.querySelectorAll');
    expect(panelSource).toContain('data-disclosure-key');
    expect(panelSource).toContain('candidateAttempts');
    expect(panelSource).toContain('candidateSuccesses');
    expect(panelSource).toContain('candidateAverageMs');
    expect(panelSource).toContain('currentAverageMs');
    expect(backendSource).toContain('candidate_attempts: usize');
    expect(backendSource).toContain('candidate_successes: usize');
    expect(backendSource).toContain('candidate_average_ms: Option<u64>');
  });

  it('connects the ASN prefix generator to RIPEstat and the ipset list editor', () => {
    expect(template).toContain('id="asnNumber"');
    expect(template).toContain('data-action-click="fetchAsnPrefixes"');
    expect(template).toContain('data-action-click="setAsnQuery"');
    expect([...template.matchAll(/data-action-click="setAsnQuery"/g)]).toHaveLength(6);
    expect(panelSource).toContain('nfqwsNetwork.prefixes(query)');
    expect(panelSource).toContain("nfqwsLists.save('ipset.list'");
    expect(backendSource).toContain('/api/nfqws2/asn-prefixes');
    expect(backendSource).toContain('resolve_asn_query(&request.query)');
    expect(panelSource).not.toContain("'fetchAsnPrefixes',");
  });

  it('keeps the support report section out of the interface for now', () => {
    expect(template).not.toContain('id="navBtn-supportReport"');
    expect(template).not.toContain('id="tabContent-supportReport"');
    expect(template).not.toContain('Отчёт для помощи');
  });

  it('styles the trace protocol picker and diagnosis helper text for clear reading', () => {
    expect(template).toContain('class="trace-protocol-picker"');
    expect(template).toContain('diagnosis-help mt-4');
    expect(panelStyles).toContain('.trace-protocol-picker:focus-within');
    expect(panelStyles).toContain('.diagnosis-row-detail');
    expect(template).toContain('lg:pr-2');
  });

  it('wraps existing status controls and metrics on narrow screens without changing their controls', () => {
    expect(template).toContain('class="metric-group flex items-center gap-2.5"');
    expect(panelStyles).toContain('@media (max-width: 640px)');
    expect(panelStyles).toContain('#headerStatusBar { flex-wrap: wrap;');
    expect(template).toContain('id="headerServiceControls"');
    expect(template).toContain('id="ramMetricText"');
    expect(template).toContain('id="cpuMetricText"');
  });

  it('reflows navigation and editors for phone-sized screens', () => {
    expect(template).toContain('class="app-main-layout flex-1 min-h-0 flex overflow-hidden');
    expect(template).toContain('class="mobile-code-editor flex h-[740px]');
    expect(template).toContain('class="mobile-list-editor flex h-[610px]');
    expect(panelStyles).toContain('@media (max-width: 767px)');
    expect(panelStyles).toContain('.app-main-layout > aside');
    expect(panelStyles).toContain('.app-main-layout > aside > div:first-child');
    expect(panelStyles).toContain('.mobile-code-editor');
    expect(panelStyles).toContain('.mobile-list-editor');
  });

  it('keeps the 16 KB diagnosis as one mobile-friendly test with one start and one cancel action', () => {
    const picker = template.slice(template.indexOf('id="tabContent-autotune"'), template.indexOf('id="tabContent-monitor"'));
    const diagnostic16k = template.slice(template.indexOf('id="tabContent-diagnostic16k"'), template.indexOf('id="tabContent-monitor"'));
    expect(picker).toContain('flex flex-col lg:flex-row');
    expect(picker).toContain('strategy-settings-grid');
    expect(picker).toContain('strategy-target-transport-row');
    expect(panelStyles).toContain('.strategy-target-transport-row { display:grid; grid-template-columns:minmax(0,1fr) minmax(13rem,.62fr);');
    expect(panelStyles).toContain('.autotune-settings-card { padding: .75rem 1rem; }');
    expect(panelStyles).toContain('.strategy-settings-grid { max-width: 980px; gap:.15rem; }');
    expect(panelStyles).toContain('.strategy-settings-grid > div.strategy-setting-row { min-width:0; display:grid;');
    expect(panelStyles).toContain('border:0; background:transparent;');
    expect(picker).not.toContain('md:grid-cols-5');
    expect(picker).toContain('id="strategyRepeatsSelect"');
    expect(picker).toContain('id="strategyStopAfterFirst"');
    expect(picker).toContain('id="strategyIncludeCommunity"');
    expect(picker).toContain('id="strategyIncludeOtherHistory"');
    expect(picker).toContain('id="strategyRefineLighter"');
    expect(picker).toContain('Искать более лёгкий вариант');
    expect(panelSource).toContain("$('#strategyStopAfterFirst')");
    expect(panelSource).toContain("$('#strategyRefineLighter')");
    expect(picker).toContain('id="strategyCustomSteps"');
    expect(picker).toContain('value="custom"');
    expect(picker).toContain('Расширенный · каталог blockcheck2');
    expect(panelSource).toContain("value.split(/\\r?\\n/)");
    expect(picker).toContain('min-h-[180px] 2xl:h-[360px]');
    expect(diagnostic16k).toContain('flex flex-col sm:flex-row');
    expect(diagnostic16k).toContain('min-w-0 flex-1');
    expect(diagnostic16k).toContain('id="freezeScanHostsInput"');
    expect(diagnostic16k).toContain('Запустить пробу');
    expect(diagnostic16k).toContain('id="freezeScanSummary"');
    expect(diagnostic16k).toContain('id="freezeScanProgress"');
    expect(diagnostic16k).toContain('id="freezeScanProgressSteps"');
    expect(diagnostic16k).toContain('id="freezeScanResults"');
    expect(diagnostic16k.match(/data-action-click="runFreezeScan"/g)).toHaveLength(1);
    expect(diagnostic16k.match(/data-action-click="cancelFreezeScan"/g)).toHaveLength(1);
    expect(diagnostic16k).not.toContain('transferDomainInput');
    expect(diagnostic16k).not.toContain('Диагностика передачи данных');
    expect(panelSource).toContain('freezeScanStart');
    expect(panelSource).toContain('freeze-scan-table');
    expect(panelSource).toContain('freezeRowIdentity(result)');
    expect(panelSource).toContain('freezeRowDetails(result)');
    expect(panelSource).toContain("operation.stage === 'strategy_search'");
    expect(panelSource).toContain('freezeProgressPresentation(operation)');
    expect(panelSource).toContain("status.classList.toggle('hidden', !active)");
    expect(panelSource).toContain('Подробности проверки');
    expect(panelSource).toContain('result.address ??');
    expect(template).toContain('id="freezeApplyPreviewPanel"');
    expect(panelSource).toContain('freezeStrategyPreview');
    expect(panelSource).toContain('JSON.stringify({ state: operation.state, results: operation.results })');
    expect(panelSource).toContain('freezeCanApplyToHost');
    expect(panelSource).toContain('freeze-result-badge');
    expect(panelSource).toContain('Применить к сайту');
    expect(panelSource).toContain('applyFreezeStrategyOneClick');
    expect(panelSource).toContain('Проверенный вариант добавляется только этому домену');
    expect(panelStyles).toContain('@keyframes freeze-row-enter');
    expect(panelStyles).toContain('.freeze-result-details summary');
    expect(panelStyles).toContain('prefers-reduced-motion: reduce');
    expect(panelSource).toContain("status.setAttribute('aria-busy'");
    expect(panelSource).toContain("cell.dataset.label = labels[index]");
    expect(panelSource).toContain('cancelFreezeScan');
  });

  it('keeps long strategy jobs cancellable until the backend reports a terminal state', () => {
    expect(panelSource).not.toContain('attempt < 600');
    expect(panelSource).toContain("while (['queued', 'running', 'stopping'].includes(current.state))");
    expect(panelSource).toContain('consecutiveFailures >= 10');
    expect(panelSource).toContain('localStorage.setItem(STRATEGY_PICK_STORAGE, operation.id)');
  });

  it('retains completed diagnostic operation IDs so the latest results restore after reload', () => {
    expect(panelSource).not.toContain('localStorage.removeItem(STRATEGY_PICK_STORAGE)');
    expect(panelSource).toContain('const savedPick = localStorage.getItem(STRATEGY_PICK_STORAGE)');
    expect(panelSource).toContain("const savedFreezeScan = localStorage.getItem('nfqws2-ui-freeze-scan')");
  });

  it('shows persisted strategy history and can reopen a saved result', () => {
    expect(template).toContain('История последних подборов');
    expect(template).toContain('id="autotuneConsoleBox"');
    expect(template).toContain('data-action-click="requestClearStrategyPickHistory"');
    expect(panelSource).toContain('nfqwsDiagnostics.strategyHistory()');
    expect(panelSource).toContain('nfqwsDiagnostics.clearStrategyHistory()');
    expect(panelSource).toContain('confirmClearStrategyPickHistory: () => void clearStrategyPickHistory()');
    expect(panelSource).toContain('Открыть результаты подбора для');
    expect(panelSource).toContain('if (strategyPickRunning) return;');
    expect(backendSource).toContain('/api/nfqws2/strategies/pick/history');
  });

  it('offers single-site and two-to-fifteen-site strategy picker modes backed by the multi-site API', () => {
    expect(template).toContain('data-action-click="setStrategyTargetMode"');
    expect(template).toContain('id="strategyMultiTargetPanel"');
    expect(template).toContain('id="strategyMultiHostList"');
    expect(template).toContain('id="strategyMultiListSelect"');
    expect(template).toContain('id="pickRandomMultiSitesButton"');
    expect(template).toContain('От 2 до 15 сайтов');
    expect(panelSource).toContain('nfqwsDiagnostics.strategyMultiStart');
    expect(panelSource).toContain('operation.multiSiteResults');
    expect(panelSource).toContain('domain.successes}/${domain.attempts}');
  });

  it('synchronizes config editor scrolling once per animation frame without rebuilding syntax highlighting', () => {
    const scrollSync = panelSource.slice(panelSource.indexOf('function syncConfigScroll'), panelSource.indexOf('function escapeConfigHtml'));
    expect(scrollSync).toContain('requestAnimationFrame');
    expect(scrollSync).toContain('scrollTop');
    expect(scrollSync).toContain('scrollLeft');
    expect(scrollSync).not.toContain('syncConfigLines(');
    expect(panelSource).toContain("addEventListener('scroll', syncConfigScroll, { passive: true })");
  });

  it('clears stale candidate cards when a selected operation has no candidate results', () => {
    const renderer = panelSource.slice(panelSource.indexOf('function renderStrategyPick'), panelSource.indexOf('async function loadStrategyPickHistory'));
    expect(renderer).toContain('container.replaceChildren()');
    expect(renderer).toContain('if (!operation.results.length)');
    expect(renderer).not.toContain('if (!container || !operation.results.length) return');
  });

  it('reports attempted candidates separately from result rows and keeps strategy details compact', () => {
    const renderer = panelSource.slice(panelSource.indexOf('function renderStrategyPick'), panelSource.indexOf('function renderStrategyPickPreview'));
    expect(renderer).toContain('operation.done} из ${operation.total}');
    expect(renderer).toContain("details.className = 'strategy-pick-steps'");
    expect(panelStyles).toContain('.strategy-pick-result-row');
    expect(renderer).not.toContain('подтверждено ${confirmed.length} из ${operation.results.length} стратегий');
  });

  it('exposes a confirmed strategy preview followed by explicit guarded apply', () => {
    const renderer = panelSource.slice(panelSource.indexOf('function renderStrategyPick'), panelSource.indexOf('async function loadStrategyPickHistory'));
    expect(renderer).toContain("['complete', 'partial', 'cancelled', 'failed'].includes(operation.state)");
    expect(renderer).toContain("details.dataset.candidateKey = candidateKey");
    expect(renderer).toContain('expandedResults.has(candidateKey)');
    expect(renderer).toContain("preview.dataset.actionClick = 'previewStrategyPick'");
    expect(renderer).toContain('Предпросмотр и применение Strategy #1');
    expect(panelSource).toContain('nfqwsDiagnostics.strategyPreview');
    expect(template).toContain('id="autotunePreviewPanel"');
    expect(template).toContain('data-action-click="applyStrategyPick"');
    expect(panelSource).toContain('nfqwsDiagnostics.applyStrategy');
    expect(backendSource).toContain('/api/nfqws2/strategies/apply');
    expect(backendSource).toContain('run_candidates_with_compaction_and_results');
    expect(backendSource).toContain('run_full_candidates_with_batches_and_results');
    expect(backendSource).toContain('operation.results.push(candidate.clone())');
    expect(panelSource).toContain('window.confirm(');
    expect(panelSource).toContain("result.targetKind");
    expect(panelStyles).toContain('freeze-result-error');
    expect(panelStyles).toContain('freeze-result-success');
  });

  it('shows editable draft fields for each nfqws2 argument group in configuration settings', () => {
    expect(template).toContain('id="nfqwsArgEditors"');
    expect(panelSource).toContain("['NFQWS_BASE_ARGS', 'Базовые параметры']");
    expect(panelSource).toContain("['NFQWS_ARGS_QUIC', 'QUIC']");
    expect(panelSource).toContain("['NFQWS_ARGS_UDP', 'UDP']");
    expect(panelSource).toContain("['NFQWS_ARGS_IPSET', 'IPSET']");
    expect(panelSource).toContain("['NFQWS_ARGS_CUSTOM', 'Пользовательские доменные профили']");
    expect(panelSource).toContain('updateConfigVariable(key, field.value)');
    expect(backendSource).toContain('set_literal_or_append(&request.key, &request.value)');
  });

  it('places the UI release notice in its own bottom-right notification area', () => {
    expect(template).toContain('id="uiUpdateNoticeWrapper"');
    expect(template).toContain('fixed bottom-5 right-5');
    expect(panelSource).toContain("const host = $('#uiUpdateNoticeWrapper')");
  });

  it('renders a concise nfqws2-UI browser title and themed favicon in all shells', () => {
    expect(backendSource).toContain('<title>nfqws2-UI</title>');
    expect(backendSource).toContain('rel="icon" type="image/svg+xml" href="/favicon.svg"');
    expect(entrySource).toContain("document.title = 'nfqws2-UI'");
    expect(viteSource).toContain('<title>nfqws2-UI</title>');
    expect(existsSync(new URL('../public/favicon.svg', import.meta.url))).toBe(true);
  });

  it('only offers delete actions for custom lists and deletes with revision protection', () => {
    expect(template).toContain('id="customListTabs"');
    expect(panelSource).toContain("const host = $('#customListTabs')");
    expect(panelSource).toContain('host.replaceChildren()');
    expect(panelSource).toContain("remove.dataset.actionClick = 'requestDeleteCustomList'");
    expect(panelSource).toContain('<svg aria-hidden="true" viewBox="0 0 24 24"');
    expect(panelSource).toContain('await nfqwsLists.delete(id, current.revision)');
    expect(readFileSync(new URL('./api.ts', import.meta.url), 'utf8')).toContain('method: \'DELETE\'');
    expect(backendSource).toContain('async fn delete_custom_list');
    expect(backendSource).toContain('safe_custom_list_id(&id)');
  });

  it('supports accessible custom tooltips for navigation and all action buttons', () => {
    expect(actionTooltipSource).toContain("'button, .side-nav-btn'");
    expect(actionTooltipSource).toContain('fallbackTooltipText');
  });

  it('persists manual monitoring checks and keeps diagnostic stages separate from history cycles', () => {
    expect(panelSource).toContain('nfqwsMonitoring.run(hosts)');
    expect(backendSource).toContain('/api/nfqws2/diagnostics/monitor');
    expect(panelSource).toContain("const labels: Record<string, string> = { dns: 'DNS', tcp: 'TCP connect', tls: 'TLS', http: 'HTTP', transfer: 'Передача' }");
    expect(panelStyles).toContain('.monitor-stage-list');
    expect(template).toContain('История последних 96 проверок хранится на роутере');
    expect(panelSource).toContain('nfqwsMonitoring.run(hosts)');
    expect(panelSource).toContain('slice(0, 48).reverse()');
    expect(panelSource).toContain('Проверок в истории: ${site.results.length}');
    expect(panelSource).toContain('Этапы последнего цикла');
    expect(backendSource).toContain('telegram_notifications');
    expect(backendSource).toContain('TELEGRAM_NOTIFICATION_TTL_SECS: u64 = 24 * 60 * 60');
    expect(backendSource).toContain('check_saved_sites(&state, true, hosts)');
    expect(backendSource).toContain('monitoring_lock.lock().await');
    expect(backendSource).toContain('record_monitoring_completion(&mut updated, only_hosts.is_none(), now_unix_seconds())');
    expect(backendSource).toContain('/api/nfqws2/telegram/notifications');
  });

  it('shows an honest compact bottom status while a domain diagnosis is running', () => {
    expect(panelSource).toContain('Проверка домена');
    expect(panelSource).toContain('На роутере проверяются DNS');
    expect(panelSource).toContain("setAttribute('role', 'status')");
    expect(panelSource).toContain('showDiagnosisProgress');
  });

  it('exposes the manually connected monitoring page and keeps only unfinished sections unavailable', () => {
    expect(template).toMatch(/id="navBtn-monitor" class="(?![^\"]*hidden)[^\"]*side-nav-btn/);
    expect(template).toContain('id="monitorHostInput"');
    expect(template).toContain('id="monitorSites"');
    expect(panelSource).toContain('nfqwsMonitoring.run(hosts)');
    expect(panelSource).toContain('renderTelegramNotifications(cfg.notifications)');
    expect(monitoringSource).toContain('MAX_MONITORING_SITES = 20');
    expect(panelSource).toContain('Проверка ${hosts.length} сайтов параллельно: DNS → TCP → TLS → HTTP и до 64 КиБ');
    expect(template).toMatch(/id="navBtn-community" class="(?![^"]*hidden)[^"]*side-nav-btn/);
    expect(template).toMatch(/id="tabContent-community" class="hidden/);
    expect(template).toContain('data-action-click="loadCommunityStrategies"');
    expect(panelSource).toContain('nfqwsDiagnostics.communityStrategies');
    expect(template).toMatch(/id="navBtn-autotune" class="(?![^"]*hidden)[^"]*side-nav-btn/);
    expect(template).toContain('data-action-click="startStrategyPick"');
    expect(template).toContain('id="tabContent-diagnostic16k"');
    expect(template).toContain('data-action-click="runFreezeScan"');
    expect(panelSource).toContain('UNAVAILABLE_PANEL_ROUTES');
  });

  it('installs delayed styled tooltips for action buttons', () => {
    const tooltipSource = readFileSync(new URL('./action-tooltips.ts', import.meta.url), 'utf8');
    expect(panelSource).toContain('installActionTooltips(document)');
    expect(tooltipSource).toContain('scheduleDelayedTooltip');
    expect(tooltipSource).toContain('element.removeAttribute(\'title\')');
    expect(tooltipSource).toContain("setAttribute('role', 'tooltip')");
    expect(tooltipSource).toContain('DEFAULT_ACTION_TOOLTIP_DELAY_MS = 2000');
    expect(tooltipSource).toContain("root.querySelectorAll('button, .side-nav-btn').forEach(prepare)");
    expect(tooltipSource).toContain("target.closest<HTMLButtonElement>('button, .side-nav-btn')");
    expect(panelStyles).toContain('.action-tooltip');
  });

  it('removes the legacy logs, backups, and support report sections from navigation', () => {
    for (const route of ['logs']) {
      expect(template).not.toContain(`id="navBtn-${route}"`);
      expect(template).not.toContain(`id="tabContent-${route}"`);
    }
    for (const route of ['snapshots', 'supportReport']) {
      expect(template).not.toContain(`id="navBtn-${route}"`);
      expect(template).not.toContain(`id="tabContent-${route}"`);
    }
    expect(panelSource).toContain("UNAVAILABLE_PANEL_ROUTES = ['community', 'snapshots', 'supportReport']");
    expect(panelSource).toContain('nfqwsSnapshots.create(');
    expect(backendSource).toContain('"/api/snapshots"');
    expect(backendSource).toContain('get(get_snapshots).post(create_snapshot_api)');
  });

  it('keeps the interface in TSX and removes the HTML entry file', () => {
    expect(existsSync(new URL('../index.html', import.meta.url))).toBe(false);
    expect(template).toContain('id="tabContent-conf"');
    expect(template).toContain('id="tabContent-lists"');
  });

  it('allows the page and trace log to scroll inside the viewport shell', () => {
    expect(entrySource).toContain("mount.className = 'flex flex-col h-full min-h-0'");
    expect(template).toContain('flex-1 min-h-0 flex overflow-hidden');
    expect(template).toContain('<main class="flex-1 min-h-0 overflow-y-auto');
  });

  it('does not show third-party project names in the panel copy', () => {
    expect(template.toLowerCase()).not.toContain('zemidala');
  });

  it('keeps the panel structure and exposes the two settings subsections', () => {
    expect(template).toContain('id="tabContent-conf"');
    expect(template).toContain('id="tabContent-lists"');
    expect(template).toContain('Настройки и оформление UI');
    expect(template).toContain('id="settingsAppearancePanel"');
    expect(template).toContain('id="settingsSecurityPanel"');
    expect(template).toContain('id="panelAuthEnabled"');
    expect(template).toContain('data-action-click="savePanelAuthSettings"');
    expect(entrySource).toContain('nfqwsAuth.status()');
    expect(entrySource).toContain('Забыть через');
    expect(entrySource).toContain('<option value="0">Никогда</option>');
    expect(backendSource).toContain('/api/auth/login');
    expect(backendSource).toContain('auth::AuthManager::middleware');
  });

  it('copies the exact About command from its row and confirms the result accessibly', () => {
    expect(template).toContain('id="aboutCopyStatus"');
    expect(template).toContain('aria-label="Копировать команду"');
    expect(appSource).toContain('JSON.stringify([{source:"self"}])');
    expect(panelSource).toContain("closest('.about-command-row')?.querySelector('code')?.textContent?.trim()");
    expect(panelSource).toContain("setText('#aboutCopyStatus', 'Команда скопирована в буфер обмена.')");
  });

  it('shows the configuration structure panel enabled on first render', () => {
    expect(template).toMatch(/id="configStructureToggle"[^>]*aria-pressed="true"[^>]*aria-expanded="true"/);
    expect(template).toMatch(/id="configOutline"[^>]*class="config-outline(?! hidden)/);
    expect(template).toContain('config-editor-workspace-open grid grid-cols-1');
    expect(panelSource).toContain("button.setAttribute('aria-pressed', String(open))");
  });

  it('nudges the paired updater controls left by one spacing unit on desktop', () => {
    const updates = template.slice(template.indexOf('id="headerUpdates"'), template.indexOf('</header>'));
    expect(updates).toContain('lg:-translate-x-1');
    expect(updates).toContain('2xl:-translate-x-1');
    expect(updates).toContain('id="updateNfqws2Button"');
    expect(updates).toContain('id="updateUiButton"');
  });

  it('shows the diagnosis checklist before its request and replays results in order', () => {
    expect(panelSource).toContain("renderDiagnosisQueue(container, SITE_DIAGNOSIS_STAGES.map");
    expect(panelSource).toContain('for (let index = 0; index < stages.length; index += 1)');
    expect(panelSource).toContain('await waitForDiagnosisAnimation()');
    expect(panelStyles).toContain('.diagnosis-queue-step.is-active');
    expect(panelStyles).toContain('.diagnosis-queue-step.is-result-failure');
  });

  it('styles checkbox inputs while keeping their native keyboard semantics', () => {
    expect(panelStyles).toContain('input[type="checkbox"] {');
    expect(panelStyles).toContain('input[type="checkbox"]:checked::before');
    expect(panelStyles).toContain('input[type="checkbox"]:focus-visible');
    expect(panelStyles).toContain('[data-theme="light"] input[type="checkbox"]');
    expect(panelStyles).toContain('input[type="checkbox"]::before {');
  });

  it('uses the full appearance card as a keyboard-accessible radio label', () => {
    expect(appSource).toContain('<label className={"appearance-choice"}><input type={"radio"} name={"appearanceTheme"}');
    expect(appSource).toContain('name={"appearanceLayout"} value={value}');
    expect(appSource).toContain('name={"appearanceAccent"} value={value}');
    expect(appSource).toContain('name={"appearanceStatusBar"} value={value}');
    for (const mode of ['standard', 'minimal', 'compact', 'stacked', 'reverse']) expect(appSource).toContain(`'${mode}'`);
    expect(appSource).not.toContain('>Расширенный</strong>');
    expect(appSource).toContain('Статус-бар');
    expect(panelStyles).toContain('.appearance-choice input[type="radio"], .appearance-layout-choice input[type="radio"]');
    expect(panelStyles).toContain('position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer;');
    expect(panelStyles).toContain('.appearance-choice:has(input:focus-visible)');
    expect(panelStyles).toContain('.appearance-choice:has(input:checked)');
    expect(panelStyles).toContain('[data-layout="right"] .app-main-layout');
    expect(panelStyles).toContain('[data-layout="top"] .app-main-layout');
    expect(panelStyles).toContain('[data-layout="tiles"] .app-main-layout');
    expect(template).toContain('Синий (стандартная)');
  });

  it('removes the selection overlay across appearance palettes', () => {
    expect(panelStyles).toContain('*::selection, *::-moz-selection { background:transparent !important; color:inherit !important;');
    expect(panelStyles).toContain('.conf-editor-input::selection, .conf-editor-input::-moz-selection { color:transparent !important;');
    expect(panelStyles).not.toContain('selection:bg-blue-500/30');
    expect(appSource).not.toContain('selection:bg-blue-500/30');
    expect(entrySource).not.toContain('selection:bg-blue-500/30');
  });

  it('does not let accent background rules match hover utility classes outside hover', () => {
    expect(panelStyles).toContain('[data-accent] [class~="bg-blue-600"], [data-accent] [class~="bg-blue-500"]');
    expect(panelStyles).toContain('[data-accent] [class~="hover:bg-blue-500"]:hover');
    expect(panelStyles).toContain('[data-accent] [class~="hover:bg-blue-600"]:hover');
    expect(panelStyles).not.toContain('[data-accent] [class*="bg-blue-600"], [data-accent] [class*="bg-blue-500"]');
  });

  it('recolors translucent blue highlights to the selected accent without making them solid', () => {
    for (const opacity of [5, 10, 15, 20, 25]) {
      expect(panelStyles).toContain(`[class~="bg-blue-500/${opacity}"]`);
      expect(panelStyles).toContain(`var(--accent-color) ${opacity}%,transparent`);
    }
    expect(panelStyles).toContain('[class~="hover:bg-blue-500/25"]:hover');
    expect(panelStyles).toContain('var(--accent-color) 25%,transparent');
  });

  it('keeps primary navigation labels visible in the tiles layout', () => {
    expect(panelStyles).toContain('[data-layout="tiles"] .app-main-layout > aside nav button > span:last-child { display:none; }');
    expect(panelStyles).not.toContain('[data-layout="tiles"] .app-main-layout > aside nav button > span { display:none; }');
  });

  it('uses the selected palette for the active navigation tile indicator and glow', () => {
    expect(panelStyles).toContain('[data-accent] .side-nav-btn.active::before');
    expect(panelStyles).toContain('background-color:var(--accent-color)');
    expect(panelStyles).toContain('box-shadow:0 0 8px color-mix(in srgb,var(--accent-color) 55%,transparent)');
  });

  it('keeps custom strategy input inside its mode tile and exposes copy feedback', () => {
    const modeTile = appSource.slice(appSource.indexOf('id={"strategyModeSelect"}'), appSource.indexOf('id={"scanProgressLabel"}'));
    expect(modeTile).toContain('id={"strategyCustomInputPanel"}');
    expect(modeTile).toContain('id={"strategyCustomDetailsDisclosure"}');
    expect(modeTile).toContain('<summary');
    expect(modeTile).toContain('text-xs leading-5 text-slate-400');
    expect(modeTile).toContain('id={"strategyCustomSteps"}');
    expect(modeTile).toContain('text-xs font-mono leading-5 text-white');
    expect(appSource).not.toMatch(/<div id=\{"strategyCustomInputPanel"\}[^>]*md:col-span-2/);
    expect(modeTile).toContain('data-action-click={"copyStrategyParams"}');
    expect(panelSource).toContain('copyStrategyParams:');
    expect(panelSource).toContain("copy.dataset.actionClick = 'copyStrategyResult'");
    expect(panelSource).toContain('copyStrategyResult:');
    expect(panelSource).toContain("closest('.strategy-mode-tile')?.classList.toggle('strategy-mode-tile-custom'");
    expect(panelSource).toContain("document.execCommand('copy')");
    expect(panelStyles).toContain('.strategy-copy-success');
  });

  it('offers list and text editor views while keeping list-row tools accessible', () => {
    const lists = template.slice(template.indexOf('id="tabContent-lists"'), template.indexOf('id="tabContent-check"'));
    expect(lists).toContain('id="listTextView"');
    expect(lists).toContain('id="listRowsView"');
    expect(lists).toContain('data-args="[&quot;rows&quot;]"');
    expect(panelSource).toContain("localStorage.setItem(LIST_EDITOR_VIEW_KEY, view)");
    expect(panelSource).toContain('listRowsFromContent(editor.value');
    expect(panelSource).toContain('removeListLinesByIndices(editor.value');
    expect(panelSource).toContain("$('#findNextListEntryButton')?.classList.toggle('hidden', listEditorView === 'rows')");
    expect(panelStyles).toContain('.list-editor-row');
    expect(panelStyles).toContain('[data-theme="light"] .list-editor-row');
  });

  it('places config actions in the same row as its section tabs', () => {
    const navStart = appSource.indexOf('<nav aria-label={"Раздел конфигурации"}');
    const navEnd = appSource.indexOf('</nav>', navStart);
    const configNav = appSource.slice(navStart, navEnd);
    expect(configNav).toContain('data-action-click={"copyConfigText"}');
    expect(configNav).toContain('data-action-click={"applyConfigAndRestart"}');
  });

  it('documents the panel-specific terminal command instead of zemidala aliases', () => {
    expect(appSource).toContain('nfqws2-ui-ctl update --check');
    expect(appSource).toContain('nfqws2-ui-ctl update v1.0.1 --force');
    expect(appSource).toContain('nfqws2-ui-ctl status');
    expect(appSource).not.toContain('nfqws-ui-setup');
    expect(appSource).not.toContain('nfqws-ui update');
  });

  it('shows Telegram route-specific settings only for the selected transport', () => {
    expect(appSource).toContain('id={"telegramInterfaceField"} className={"hidden grid');
    expect(appSource).toContain('id={"telegramProxyField"} className={"hidden grid');
    expect(appSource).toContain('data-action-change={"toggleTelegramTransportFields"}');
    expect(panelSource).toContain("via !== 'interface'");
    expect(panelSource).toContain("via !== 'proxy'");
    expect(panelSource).toContain('toggleTelegramTransportFields(cfg.sendVia)');
  });

  it('labels every auto-strategy row with a plain-language confirmation result', () => {
    expect(panelSource).toContain("result.confirmed ? 'Подтверждена' : 'Не подтверждена'");
    expect(panelSource).toContain('strategy-pick-result-badge');
    expect(panelSource).toContain('result.successes}/${result.attempts} успешных');
    expect(panelStyles).toContain('.strategy-pick-result-badge.is-confirmed');
    expect(panelStyles).toContain('.strategy-pick-result-badge.is-unconfirmed');
    expect(panelSource).toContain('strategy-pick-result-status-icon');
    expect(panelStyles).toContain('.strategy-pick-result-status-icon');
    expect(panelSource).toContain("preview.dataset.actionClick = 'previewStrategyPick'");
    expect(panelSource).toContain('Предпросмотр и применение Strategy #1');
    expect(panelSource).toContain('NFQWS_BASE_ARGS не меняется.');
    expect(panelSource).toContain("nfqwsDiagnostics.strategyPreview(operationId, index)");
  });

  it('lets long mobile navigation labels wrap within their grid cells', () => {
    expect(panelStyles).toContain('.app-main-layout > aside nav button > div > span');
    expect(panelStyles).toContain('overflow-wrap: anywhere; line-height: 1.25;');
    expect(panelStyles).toContain('grid-template-columns: minmax(0, 1fr);');
    expect(panelStyles).toContain('max-height: 50vh; overflow-y: auto;');
    expect(template).toContain('Списки доменов и IP');
    expect(template).not.toContain('Резервные копии');
  });

  it('does not ship example credentials in login controls', () => {
    expect(template).not.toContain('authPassword');
    expect(appSource).toContain('id={"panelAuthPassword"}');
    expect(template).not.toContain('7194028410:AAH8x9kLmP2qW_nfqws2_router_bot');
  });

  it('keeps remember-me duration visible and removes the obsolete password minimum', () => {
    expect(entrySource).toContain('id="rememberDays"');
    expect(entrySource).toContain('disabled={!remember}');
    expect(entrySource).toContain('<option value="0">Никогда</option>');
    expect(appSource).not.toContain('minLength={12}');
    expect(appSource).not.toContain('Не менее 12 символов');
    expect(appSource).toContain('даже из одного символа');
  });

  it('keeps list deduplication as a real local draft action', () => {
    expect(panelSource).toMatch(/deduplicateCurrentList:\s*\(\)\s*=>/);
    const disabledList = panelSource.match(/export const DISABLED_PANEL_ACTIONS = \[([\s\S]*?)\] as const/);
    expect(disabledList?.[1]).not.toContain('deduplicateCurrentList');
    expect(panelSource).toContain('deduplicateListContent');
  });

  it('provides list draft add, line removal, search, import, and export actions without fabricated presets', () => {
    expect(template).not.toContain('id="listAddEntriesInput"');
    expect(template).toContain('id="listSearchInput"');
    expect(template).toContain('id="listImportFile"');
    expect(template).not.toContain('data-action-click="appendListEntriesToDraft"');
    expect(template).toContain('data-action-click="removeSelectedListLines"');
    expect(template).toContain('data-action-click="findNextListEntry"');
    expect(template).toContain('data-action-click="exportListDraft"');
    expect(template).not.toContain('injectPresetDomains');
    expect(panelSource).toContain('nfqwsLists.save(activeList.id, editor.value, activeList.revision)');
  });

  it('matches each list tab by its real element id when enabling files from the API index', () => {
    expect(panelSource).toContain('listIdFromTabElementId(candidate.id)');
    expect(panelSource).toContain('availableListIds.has(tabId)');
    expect(panelSource).toContain('renderCustomListTabs(index.files)');
    expect(template).toContain('id="listFileTabs"');
    expect(panelSource).not.toContain('index.files.some((file) => file.id === candidate.dataset.arg)');
  });

  it('shows list filenames in tabs and localized names in the selected list details', () => {
    expect(template).toContain('user.list');
    expect(template).toContain('auto.list');
    expect(template).toContain('exclude.list');
    expect(template).toContain('ipset.list');
    expect(template).toContain('ipset_exclude.list');
    expect(panelSource).toContain("'user.list': 'Пользовательские домены'");
    expect(panelSource).toContain("'auto.list': 'Домены для автообучения'");
    expect(panelSource).toContain("'exclude.list': 'Исключённые домены'");
    expect(panelSource).toContain("'ipset.list': 'IP-адреса'");
    expect(panelSource).toContain("'ipset_exclude.list': 'Исключённые IP-адреса'");
    expect(panelSource).toContain('listDisplayName(id)');
    expect(panelSource).toContain('title = result.path');
  });

  it('refreshes real CPU and memory metrics every three seconds with usage thresholds', () => {
    expect(panelSource).toContain('window.setInterval(() => { if (document.visibilityState === \'visible\') void fetchStatus(); }, 3000)');
    expect(panelSource).toContain('resourceUsagePercent(used, total)');
    expect(panelSource).toContain("import { resourceUsagePercent, usageTone } from './metrics'");
    expect(template).toContain('id="ramMetricText"');
    expect(template).toContain('id="cpuMetricText"');
  });

  it('shows a spinning status-bar indicator during service restarts and reapplies the backend state afterward', () => {
    expect(template).toContain('id="statusIndicator"');
    expect(panelSource).toContain('setRestartInProgress(true)');
    expect(panelSource).toContain('setRestartInProgress(false)');
    expect(panelStyles).toMatch(/\.status-restarting\s+#statusRadarRing[\s\S]*?animation:\s*status-spin/);
    expect(panelStyles).toContain('@keyframes status-spin');
  });

  it('validates the config in Rust before applying and confirms unresolved shell values', () => {
    expect(panelSource).toContain('await nfqwsConfig.validate(draft)');
    expect(panelSource).toContain('if (!validation.valid)');
    expect(panelSource).toContain('validation.checks.installedBinary');
    expect(panelSource).toContain('Проверка установленным бинарником не выполнялась');
    expect(panelSource.indexOf('await nfqwsConfig.validate(draft)')).toBeLessThan(panelSource.indexOf('nfqwsApply.apply(draft, config.revision)'));
  });

  it('keeps separate entry points for UI and core releases without in-modal switching', () => {
    expect(template).toContain('data-action-click="openUpdateModal" data-args="[&quot;core&quot;]"');
    expect(template).toContain('data-action-click="openUpdateModal" data-args="[&quot;ui&quot;]"');
    expect(template).not.toContain('data-action-click="switchUpdateTarget"');
    expect(template).not.toContain('id="updateTargetCore"');
    expect(panelSource).toContain("nfqwsUpdates.coreReleases()");
    expect(panelSource).toContain("nfqwsUpdates.installCore(tag)");
    expect(panelSource).toContain("renderReleaseMarkdown(release.notes)");
    expect(panelSource).not.toMatch(/installSelectedRelease.*toast\('Эта функция пока не подключена/);
  });

  it('does not register a beforeunload warning that blocks browser reload or close', () => {
    expect(panelSource).not.toMatch(/beforeunload|returnValue/);
    expect(entrySource).not.toMatch(/beforeunload|returnValue/);
    const serviceActions = panelSource.slice(panelSource.indexOf('async function serviceAction'), panelSource.indexOf('async function loadList'));
    expect(serviceActions).not.toContain('window.confirm(');
  });

  it('checks UI updates in the background and provides a persistent dismissible notice', () => {
    expect(panelSource).toContain("nfqwsUpdates.uiStatus()");
    expect(panelSource).toContain("window.setInterval(() => { if (document.visibilityState === 'visible') void refreshUiUpdateStatus(); }, 5 * 60 * 1000)");
    expect(panelSource).toContain("dataset.actionClick = 'openLatestUiUpdate'");
    expect(panelSource).toContain("dataset.actionClick = 'dismissUiUpdateNotice'");
    expect(panelStyles).toContain('.version-pill.update-available');
    expect(panelStyles).toContain('.ui-update-notice');
    expect(panelSource).toContain("document.createElementNS('http://www.w3.org/2000/svg', 'svg')");
    expect(panelStyles).toContain('.ui-update-notice > button { margin-left: 26px;');
  });

  it('shows the confirmed update result after a cache-fresh page reload', () => {
    expect(panelSource).toContain('schedulePanelRefreshAfterUpdate');
    expect(panelSource).toContain('nfqws2-ui-update-notice');
    expect(panelSource).toContain('window.location.reload()');
    expect(backendSource).toContain('cache-control');
  });

  it('improves font rendering and preserves reduced-motion preferences', () => {
    expect(panelStyles).toContain('-webkit-font-smoothing: antialiased');
    expect(panelStyles).toContain('text-rendering: optimizeLegibility');
    expect(panelStyles).toContain('.status-dot-active');
    expect(panelStyles).toContain('prefers-reduced-motion: reduce');
  });

  it('keeps configuration settings focused and omits the removed runtime summary block', () => {
    expect(template).not.toContain('TCP Circular (Стратегии 1–6)');
    expect(template).not.toContain('5 файлов OK');
    expect(template).not.toContain('id="nfqwsConfigSections"');
    expect(template).not.toContain('id="nfqwsRuntimeProfiles"');
    expect(template).not.toContain('Профили фактически работающего процесса:');
    expect(panelSource).not.toContain('summarizeNfqwsConfig');
    expect(panelSource).not.toContain('nfqwsRuntime.get()');
  });

  it('uses backend-discovered network interfaces instead of guessed router names', () => {
    expect(template).not.toContain('eth3 (Провайдер)');
    expect(template).not.toContain('ppp0 (PPPoE)');
    expect(template).not.toContain('eth2.2 (VLAN)');
    expect(template).not.toContain('nwg0 (WireGuard)');
    expect(template).toContain('data-key="ISP_INTERFACE"');
    expect(panelSource).toContain('installation.networkInterfaces');
    expect(backendSource).toContain('network_interface_names(Path::new("/sys/class/net"))');
  });

  it('keeps Lua script source browsing in the Scripts tab instead of duplicating paths in settings', () => {
    expect(template).toContain('id="configScriptsPanel"');
    expect(template).not.toContain('id="moduleFilesContainer"');
    expect(template).not.toContain('/opt/etc/nfqws2/lua/');
    expect(template).not.toContain('/opt/etc/nfqws2/blobs/');
    expect(panelSource).toContain('nfqwsScripts.get(name)');
  });

  it('loads the selected mode from the backend config and limits choices to declared mode variables', () => {
    expect(panelSource).toContain('availableNfqwsModes(content)');
    expect(panelSource).toContain('select.value = [...select.options].some((option) => option.value === selectedMode) ? selectedMode :');
  });

  it('does not show invented router metrics, monitoring, provider, or update progress', () => {
    expect(template).toMatch(/id="ramProgressBar"[^>]*style="width:0%"/);
    expect(template).toMatch(/id="cpuProgressBar"[^>]*style="width:0%"/);
    expect(template).toContain('Уведомления Telegram');
    expect(template).toContain('каждые 30 мин');
    expect(template).toContain('ASN не определён');
    expect(template).toContain('Варианты пока не загружены.');
    expect(template).toContain('0%');
    expect(template).not.toContain('Ростелеком (AS12389)');
    expect(template).not.toContain('Рейтинг успешности: 96%');
    expect(template).not.toContain('Активен (каждые 15 мин)');
    expect(template).not.toContain('45%');
    expect(template).not.toContain('21 ms • OK');
    expect(template).not.toContain('482910492');
    expect(template).not.toContain('42 префикса');
    expect(template).not.toContain('nfqws2 запущен');
  });

  it('keeps all styling and interactions local for the router CSP', () => {
    expect(template).not.toMatch(/\son(?:click|input|change|scroll|submit)=/i);
    expect(template).not.toContain('cdn.tailwindcss.com');
    expect(template).not.toContain('fonts.googleapis.com');
    expect(template).toMatch(/data-action(?:-(?:click|input|change|scroll|submit))=/);
    expect(panelSource).toContain("import './panel.css'");
    expect(panelSource).toContain("import '@fontsource-variable/inter'");
    expect(panelSource).toContain("import '@fontsource-variable/jetbrains-mono'");
  });

  it('keeps site diagnosis and packet tracing as separate navigation sections', () => {
    expect(template).toContain('id="navBtn-check"');
    expect(template).toContain('id="tabContent-check"');
    expect(template).toContain('id="navBtn-trace"');
    expect(template).toContain('id="tabContent-trace"');
    expect(template).toContain('data-action-click="runSiteTrace"');
  });

  it('polls the router-side trace operation and exposes graceful stop', () => {
    expect(template).toContain('id="traceStopButton"');
    expect(template).toContain('id="traceOperationStatus"');
    expect(panelSource).toContain('nfqwsDiagnostics.traceStart');
    expect(panelSource).toContain('nfqwsDiagnostics.traceStatus');
    expect(panelSource).toContain('nfqwsDiagnostics.traceStop');
    expect(panelSource).toContain('stopSiteTrace:');
  });

  it('does not advertise QUIC strategy picking until a real isolated QUIC runner exists', () => {
    expect(template).not.toContain('<option value="quic">QUIC · UDP 443</option>');
    expect(template).toContain('перебор QUIC-стратегий пока не подключён');
    expect(backendSource).toContain('Перебор стратегий QUIC пока не поддерживается');
  });

  it('has no duplicate action attributes and every template action is wired', () => {
    const tags = template.match(/<[a-z][^>]*>/gi) ?? [];
    for (const tag of tags) {
      expect(tag.match(/\bdata-action=/g)?.length ?? 0).toBeLessThanOrEqual(1);
      expect(tag.match(/\bdata-event=/g)?.length ?? 0).toBeLessThanOrEqual(1);
    }
    const actions = new Set([...template.matchAll(/\bdata-action(?:-(?:click|input|change|scroll|submit))?="([^"]+)"/g)].map((match) => match[1]));
    const declaredActions = new Set([...panelSource.matchAll(/^\s{4}([A-Za-z_$][\w$]*):/gm)].map((match) => match[1]));
    const disabledList = panelSource.match(/export const DISABLED_PANEL_ACTIONS = \[([\s\S]*?)\] as const/);
    const disabledActions = new Set([...(disabledList?.[1].matchAll(/'([A-Za-z_$][\w$]*)'/g) ?? [])].map((match) => match[1]));
    for (const action of actions) {
      expect(declaredActions.has(action) || disabledActions.has(action), `action ${action} must be explicitly handled`).toBe(true);
    }
  });
});
