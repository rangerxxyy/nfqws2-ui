import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import App from './App';

const template = renderToStaticMarkup(React.createElement(App));
const panelSource = readFileSync(new URL('./panel.ts', import.meta.url), 'utf8');
const panelStyles = readFileSync(new URL('./panel.css', import.meta.url), 'utf8');
const entrySource = readFileSync(new URL('./main.tsx', import.meta.url), 'utf8');
const backendSource = readFileSync(new URL('../../backend/src/main.rs', import.meta.url), 'utf8');

describe('TSX panel without authentication', () => {
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
    expect(statusGroup).toContain('aria-label="Остановить nfqws2"');
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

  it('shows an honest compact bottom status while a domain diagnosis is running', () => {
    expect(panelSource).toContain('Проверка домена');
    expect(panelSource).toContain('На роутере проверяются DNS');
    expect(panelSource).toContain("setAttribute('role', 'status')");
    expect(panelSource).toContain('showDiagnosisProgress');
  });

  it('hides unfinished sections and prevents opening them through tab actions', () => {
    for (const route of ['autotune', 'monitor', 'community']) {
      expect(template).toMatch(new RegExp(`id="navBtn-${route}" class="[^"]*hidden`));
      expect(template).toMatch(new RegExp(`id="tabContent-${route}" class="hidden`));
    }
    expect(panelSource).toContain('UNAVAILABLE_PANEL_ROUTES');
  });

  it('removes the logs and backup sections from navigation and panel actions', () => {
    for (const route of ['logs', 'snapshots']) {
      expect(template).not.toContain(`id="navBtn-${route}"`);
      expect(template).not.toContain(`id="tabContent-${route}"`);
    }
    expect(template).not.toContain('Система и Архивы');
    expect(template).not.toContain('148,920 пакетов');
    expect(template).not.toContain('Удалённое копирование на NAS по SSH');
    expect(panelSource).not.toContain('loadLogs');
    expect(panelSource).not.toContain('loadBackups');
    expect(panelSource).not.toContain('restoreSnapshot');
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

  it('keeps the supplied panel structure and has no authorization screen', () => {
    expect(template).toContain('id="tabContent-conf"');
    expect(template).toContain('id="tabContent-lists"');
    expect(template).not.toContain('authForm');
    expect(template).not.toContain('navBtn-auth');
  });

  it('does not ship login controls or example credentials', () => {
    expect(template).not.toContain('authPassword');
    expect(template).not.toContain('7194028410:AAH8x9kLmP2qW_nfqws2_router_bot');
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
    expect(panelSource).toContain('await nfqwsConfig.validate(editor.value)');
    expect(panelSource).toContain('if (!validation.valid)');
    expect(panelSource).toContain('validation.checks.installedBinary');
    expect(panelSource).toContain('Проверка установленным бинарником не выполнялась');
    expect(panelSource.indexOf('await nfqwsConfig.validate(editor.value)')).toBeLessThan(panelSource.indexOf('nfqwsApply.apply(editor.value, config.revision)'));
  });

  it('keeps separate version pickers for the panel and the nfqws2 core', () => {
    expect(template).toContain('data-action-click="openUpdateModal" data-args="[&quot;core&quot;]"');
    expect(template).toContain('data-action-click="openUpdateModal" data-args="[&quot;ui&quot;]"');
    expect(template).toContain('data-action-click="switchUpdateTarget"');
    expect(template).toContain('id="updateTargetCore"');
    expect(panelSource).toContain("nfqwsUpdates.coreReleases()");
    expect(panelSource).toContain("nfqwsUpdates.installCore(tag)");
    expect(panelSource).toContain("textContent = line.replace");
    expect(panelSource).not.toMatch(/installSelectedRelease.*toast\('Эта функция пока не подключена/);
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

  it('does not label example profiles as active and renders config-derived sections', () => {
    expect(template).not.toContain('TCP Circular (Стратегии 1–6)');
    expect(template).not.toContain('5 файлов OK');
    expect(template).toContain('id="nfqwsConfigSections"');
    expect(panelSource).toContain('summarizeNfqwsConfig');
    expect(panelSource).toContain('renderConfigSections(config.content)');
  });

  it('shows profiles from the running nfqws2 process separately from config-derived sections', () => {
    expect(template).toContain('id="nfqwsRuntimeProfiles"');
    expect(template).toContain('Профили фактически работающего процесса:');
    expect(panelSource).toContain('nfqwsRuntime.get()');
    expect(panelSource).toContain('Снято с /proc/');
    expect(backendSource).toContain('.route("/api/nfqws2/runtime", get(get_runtime))');
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

  it('shows Lua/blob files from the backend instead of guessed example paths', () => {
    expect(template).toContain('id="moduleFilesContainer"');
    expect(template).not.toContain('/opt/etc/nfqws2/lua/');
    expect(template).not.toContain('/opt/etc/nfqws2/blobs/');
    expect(panelSource).toContain('installation.moduleReferences');
    expect(panelSource).toContain('renderConfigModules(moduleReferences)');
  });

  it('loads the selected mode from the backend config and limits choices to declared mode variables', () => {
    expect(panelSource).toContain('availableNfqwsModes(content)');
    expect(panelSource).toContain('select.value = [...select.options].some((option) => option.value === selectedMode) ? selectedMode :');
  });

  it('does not show invented router metrics, monitoring, provider, or update progress', () => {
    expect(template).toMatch(/id="ramProgressBar"[^>]*style="width:0%"/);
    expect(template).toMatch(/id="cpuProgressBar"[^>]*style="width:0%"/);
    expect(template).toContain('Данные мониторинга не подключены');
    expect(template).toContain('Провайдер не определён');
    expect(template).toContain('Нет данных сообщества из backend.');
    expect(template).toContain('Нет контрольных сайтов из backend.');
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
