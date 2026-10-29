import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import App from './App';

const template = renderToStaticMarkup(React.createElement(App));
const panelSource = readFileSync(new URL('./panel.ts', import.meta.url), 'utf8');
const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');
const panelStyles = readFileSync(new URL('./panel.css', import.meta.url), 'utf8');
const codeEditorSource = readFileSync(new URL('./config-code-editor.ts', import.meta.url), 'utf8');
const styledSelectSource = readFileSync(new URL('./styled-select.ts', import.meta.url), 'utf8');
const monitoringSource = readFileSync(new URL('./monitoring.ts', import.meta.url), 'utf8');
const actionTooltipSource = readFileSync(new URL('./action-tooltips.ts', import.meta.url), 'utf8');
const viteSource = readFileSync(new URL('../vite.config.ts', import.meta.url), 'utf8');
const entrySource = readFileSync(new URL('./main.tsx', import.meta.url), 'utf8');
const backendSource = readFileSync(new URL('../../backend/src/main.rs', import.meta.url), 'utf8');

describe('TSX panel with optional server-side authentication', () => {
  it('opens DPI Lab on a text-first strategy visualizer while retaining PCAP and isolated test tabs', () => {
    expect(template).toContain('id="dpiLabPane-visualizer"');
    expect(template).toContain('id="dpiLabPane-live"');
    expect(template).toContain('id="dpiLabPane-pcap"');
    expect(template).toContain('accept=".conf,.txt,.lua,text/plain"');
    expect(panelSource).toContain("switchDpiLabTab('visualizer')");
    expect(panelSource).toContain('nfqwsDpiLab.analyzeStrategy(text)');
    expect(panelSource).toContain("'custom', 2, [steps]");
    expect(panelSource).toContain('журнал фактических Lua-вызовов этот API не предоставляет');
  });

  it('keeps the simple result concise and gates technical sections behind detail levels', () => {
    expect(template).toContain('id="dpiLabQuickOverview"');
    expect(template).toContain('data-dpi-min-level="advanced"');
    expect(template).toContain('id="dpiLabExpertPanel"');
    expect(panelSource).toContain("dpiHumanOverview(analysis)");
    expect(panelSource).toContain("element.dataset.dpiMinLevel");
  });

  it('shows list usage by profile and disables deletion while a profile still uses that list', () => {
    expect(template).toContain('id="listProfileUsageStatus"');
    expect(panelSource).toContain('summary.usedBy');
    expect(panelSource).toContain('summary.usageKnown === false');
    expect(panelSource).toContain("listUsageRoleLabel(usage.role)");
    expect(panelSource).toContain('Список используется профилями. Сначала отключите его в конструкторе стратегий.');
    expect(backendSource).toContain('fn list_profile_usage(');
    expect(backendSource).toContain('Список используется профилями {names}');
  });

  it('does not expose the deferred package leftovers tools in the current UI', () => {
    expect(template).not.toContain('Файлы после обновлений');
    expect(template).not.toContain('id="packageLeftoverCompareModal"');
    expect(panelSource).not.toContain('loadPackageLeftovers');
  });

  it('offers inline profile renaming in the name field without a pencil or browser prompt', () => {
    expect(panelSource).toContain("nameLabel.textContent = 'Имя'");
    expect(panelSource).toContain("nameInput.className = 'runtime-profile-name-input'");
    expect(panelSource).toContain("nameInput.addEventListener('change'");
    expect(panelSource).not.toContain("openRuntimeProfileRenameDialog");
    expect(panelSource).not.toContain("textContent = '✎'");
    expect(panelSource).toContain("renameRuntimeProfile:");
    expect(panelSource).not.toMatch(/\b(?:window\.)?prompt\s*\(/);
  });

  it('exposes snapshot retention limits and saves the local storage budget', () => {
    expect(template).toContain('id="snapshotMaxLocalMib"');
    expect(template).toContain('id="snapshotMaxCount"');
    expect(template).toContain('id="snapshotDays"');
    expect(template).toContain('data-action-click="saveSnapshotRetention"');
    expect(panelSource).toContain("maxLocalMib: number('#snapshotMaxLocalMib', current.maxLocalMib)");
    expect(panelSource).toContain('data.totalBytes > snapshotLimitBytes');
  });

  it('shows available router filesystem space and warns below the maximum archive size', () => {
    expect(template).toContain('id="snapshotDiskSpaceStatus"');
    expect(panelSource).toContain('snapshotFreeSpaceState(data.availableBytes, data.maxArchiveBytes)');
    expect(panelSource).toContain('меньше максимального размера архива');
  });

  it('keeps list tabs compact and offers a delayed explanation', () => {
    for (const filename of ['user.list', 'exclude.list', 'auto.list', 'ipset.list', 'ipset_exclude.list']) {
      expect(template).toContain(`<code>${filename}</code>`);
    }
    expect(template).not.toContain('Пользовательские домены</span>');
    expect(template).toContain('data-action-tooltip-delay-ms="3000"');
    expect(panelSource).toContain('Пользовательский список: ${summary.path}');
    expect(panelSource).toContain("button.dataset.actionTooltipDelayMs = '3000'");
    expect(panelStyles).toContain('.list-file-tab-caption');
    expect(panelStyles).toContain('overflow-wrap:anywhere');
  });

  it('shows daily ASN list refresh state and errors beside the selected list', () => {
    expect(template).toContain('id="listAutoUpdateStatus"');
    expect(panelSource).toContain('summary.managedAsn');
    expect(panelSource).toContain('обновляется ежедневно');
    expect(panelSource).toContain('lastError');
    expect(panelSource).toContain('lastSuccessDate');
    expect(panelSource).toContain('lastAttemptDate');
    expect(panelSource).toContain('автообновление остановлено');
  });

  it('keeps the profile name field compact and the constructor mode switch in the heading', () => {
    expect(panelSource).toContain("nameField.className = 'runtime-profile-name-field'");
    expect(panelSource).toContain("id = 'runtimeProfileModeToggle'");
    expect(panelSource).not.toContain("modeTitle.textContent = 'Редактор профиля'");
    expect(panelStyles).toContain('.runtime-profile-mode-toggle::before');
    expect(panelStyles).toContain('.runtime-profile-name-input');
  });

  it('renders profile parameter chips with separate key, value, and remove styles', () => {
    const keyRule = [...panelStyles.matchAll(/\.profile-parameter-key \{([^}]*)\}/g)].map((match) => match[1]).find((rule) => rule.includes('display:inline-flex')) ?? '';
    const valueRule = [...panelStyles.matchAll(/\.profile-parameter-value \{([^}]*)\}/g)].map((match) => match[1]).find((rule) => rule.includes('box-sizing:border-box')) ?? '';

    expect(panelSource).toContain("chip.className = 'profile-parameter-chip'");
    expect(panelSource).toContain("keyLabel.className = 'profile-parameter-key'");
    expect(panelSource).toContain('keyLabel.textContent = hasValue ? key : parameter');
    expect(panelSource).not.toContain('`${key}=`');
    expect(panelSource).toContain("valueInput.className = 'profile-parameter-value'");
    expect(panelSource).toContain("removeParameter.className = 'profile-parameter-remove'");
    expect(panelStyles).toContain('.profile-parameter-key');
    expect(panelStyles).toContain('.profile-parameter-value');
    expect(panelStyles).toContain('.profile-parameter-chip');
    expect(panelStyles).toContain('background:linear-gradient(105deg,#211b30 0%,#1c2234 48%,#172734 100%)');
    expect(panelStyles).toContain('background:linear-gradient(100deg,#26304866,#203c4a77)');
    expect(keyRule).toMatch(/display:inline-flex/);
    expect(keyRule).toMatch(/align-items:center/);
    expect(keyRule).toMatch(/height:1\.2rem/);
    expect(keyRule).toMatch(/background:/);
    expect(keyRule).toMatch(/border-right:1px solid/);
    expect(valueRule).toMatch(/border:1px solid/);
    expect(valueRule).toMatch(/border-radius:\.24rem/);
    expect(valueRule).toMatch(/height:1\.2rem/);
    expect(valueRule).not.toMatch(/margin-left:/);
  });

  it('keeps the selected shared Lua function visibly highlighted in the themed picker', () => {
    expect(styledSelectSource).toContain("menu.dataset.profileFunctionPicker = 'true'");
    expect(panelStyles).toContain(".styled-select-menu[data-profile-function-picker='true'] .styled-select-option[aria-selected='true']");
    expect(panelStyles).toContain('var(--accent-color,#3b82f6) 16%');
  });

  it('keeps strategy rows and long parameter chips within a responsive grid', () => {
    const stepContentRule = panelStyles.match(/\.profile-step-content \{([^}]*)\}/)?.[1] ?? '';
    expect(panelSource).toContain("content.className = 'profile-step-content'");
    expect(panelSource).toContain("parameterList.className = 'profile-step-parameters'");
    expect(panelSource).toContain("controls.className = 'profile-step-controls'");
    expect(panelStyles).toContain('.profile-step-row { display:grid;');
    expect(panelStyles).toContain('.profile-step-parameters { display:flex;');
    expect(panelSource).toContain("row.classList.add('profile-builder-filter-group')");
    expect(panelStyles).toContain('.profile-builder-filter-group { display:grid; grid-template-columns:minmax(5rem,auto) minmax(0,1fr) auto;');
    expect(panelStyles).toContain('.profile-filter-remove-argument { flex:0 0 auto; align-self:center; }');
    expect(panelStyles).toContain('.profile-filter-add-input { width:9.5rem; max-width:100%; flex:0 1 9.5rem;');
    expect(panelStyles).toContain('overflow-wrap:anywhere');
    expect(panelStyles).toContain('@media (max-width:640px)');
    expect(stepContentRule).toMatch(/min-width:\s*0/);
    expect(panelStyles).toMatch(/@container \(max-width:640px\)\s*\{\s*\.profile-step-grip \{ display:none; \}\s*\.profile-step-row \{ display:flex; flex-wrap:wrap;/);
    expect(panelStyles).toMatch(/\.profile-step-content \{[^}]*flex:1 1 0;/);
    expect(panelStyles).toMatch(/\.profile-step-row > input \{[^}]*flex:1 1 12rem;/);
    expect(panelStyles).toMatch(/\.profile-step-controls \{[^}]*flex:0 0 100%;/);
  });

  it('uses shared UI button states in the list toolbar', () => {
    expect(template).toContain('list-toolbar-button list-toolbar-button-secondary');
    expect(template).toContain('list-toolbar-button list-toolbar-button-primary');
    expect(template).toContain('list-toolbar-button list-toolbar-button-danger');
    expect(panelStyles).toContain('.list-toolbar-button:focus-visible');
    expect(panelStyles).toContain('.list-view-button[aria-pressed="true"]');
    expect(panelStyles).toContain('.list-toolbar-button:disabled');
  });

  it('places change-history actions directly under the changed filename', () => {
    const renderStart = panelSource.indexOf('function renderChangeHistory(');
    const renderEnd = panelSource.indexOf('\nfunction changedHistoryLines', renderStart);
    const renderSource = panelSource.slice(renderStart, renderEnd);

    expect(renderSource).toContain('details.append(actions)');
    expect(renderSource).toContain('row.append(summary, diff)');
    expect(renderSource).not.toContain('row.append(summary, actions, diff)');
    expect(panelStyles).toMatch(/\.change-history-actions\s*\{\s*display:\s*flex;\s*flex-wrap:\s*wrap;\s*justify-content:\s*flex-start;/);
    expect(panelStyles).not.toContain('.change-history-actions { justify-content: flex-start; margin-left: 3.4rem; }');
  });

  it('shows the actual nfqws2 argument variable under every profile heading', () => {
    expect(panelSource).toContain("getRuntimeProfileConstructorTarget(profile)?.source");
    expect(panelSource).toContain("Пользовательский профиль · ${argumentVariable}");
    expect(panelSource).toContain("Штатная группа · ${argumentVariable}");
  });

  it('keeps the list footer free of the three removed helper lines', () => {
    expect(template).not.toContain('Ctrl+/ — комментировать строки или снять комментарий');
    expect(template).not.toContain('Проверка валидности доменов и CIDR-масок активна');
    expect(template).not.toContain('Служба не перезапускается при сохранении');
    expect(template).toContain('id="listFooterCount"');
  });

  it('defines shared typography and semantic color tokens for UI sections', () => {
    expect(panelStyles).toContain('--ui-font-sans:');
    expect(panelStyles).toContain('--ui-text-caption:');
    expect(panelStyles).toContain('--ui-color-text-secondary:');
    expect(panelStyles).toContain('--ui-color-warning:');
    expect(panelStyles).toContain('[class~="text-[11px]"]');
    expect(panelStyles).toContain('--ui-space-4:');
    expect(panelStyles).toContain('code,kbd,pre,samp,.font-mono');
  });

  it('uses UI confirmations instead of browser dialogs and traps modal focus', () => {
    expect(panelSource).not.toMatch(/\b(?:window\.)?(?:alert|confirm|prompt)\s*\(/);
    expect(panelSource).toContain('function trapAppModalFocus');
    expect(panelSource).toContain("event.key === 'Tab'");
    expect(panelSource).toContain('closeDangerousConfirmation()');
    expect(template).toContain('role="alertdialog"');
  });

  it('keeps config confirmation actions directly bound and outside global busy disabling', () => {
    expect(template).toContain('id="operationConfirmCancel"');
    expect(template).toContain('id="operationConfirmContinue"');
    expect(template).not.toContain('data-action-click="cancelOperationConfirmation"');
    expect(template).not.toContain('data-action-click="confirmOperationConfirmation"');
    expect(panelSource).toContain("$('#operationConfirmCancel')?.addEventListener('click', () => closeDangerousConfirmation());");
    expect(panelSource).toContain("$('#operationConfirmContinue')?.addEventListener('click', () => void confirmDangerousOperation());");
    expect(panelSource).toContain("openAppModal(modal, '#operationConfirmCancel')");
  });

  it('releases the busy lock while config confirmation is open and restores it on continue', () => {
    const apply = panelSource.match(/async function saveConfigAndApply[\s\S]*?\n}\n/)?.[0] ?? '';
    expect(apply).toContain("setBusy(false);\n    requestDangerousConfirmation('Проверка конфигурации'");
    expect(apply).toContain('const applyValidatedConfig = async () => {\n    if (editor.value !== draft || !config)');
    expect(apply).toContain('    setBusy(true);\n    setRestartInProgress(true);');
  });

  it('reloads the editor after backend confirms an automatic config rollback', () => {
    const apply = panelSource.match(/async function saveConfigAndApply[\s\S]*?\n}\n/)?.[0] ?? '';
    expect(apply).toContain("error.message.includes('Исходный конфиг восстановлен')");
    expect(apply).toContain('await loadConfig()');
  });

  it('skips validation and the warning dialog when the configuration has no edits', () => {
    const apply = panelSource.match(/async function saveConfigAndApply[\s\S]*?\n}\n/)?.[0] ?? '';
    const unchangedGuard = apply.indexOf("configDraftDisposition(draft, baselineContent, config.content, baselineRevision, config.revision) === 'unchanged'");
    const validationCall = apply.indexOf('nfqwsConfig.validate(draft)');
    expect(unchangedGuard).toBeGreaterThanOrEqual(0);
    expect(unchangedGuard).toBeLessThan(validationCall);
    expect(apply).toContain("toast('Конфигурация не изменена.');");
    expect(apply).toContain("validatedDraftState === 'unchanged'");
    expect(apply).toContain("confirmedDraftState === 'stale'");
  });

  it('treats CRLF and LF config text as the same draft in strategy apply guards', () => {
    const oneClick = panelSource.match(/async function applyFreezeStrategyOneClick[\s\S]*?\n}\n/)?.[0] ?? '';
    const applyPicked = panelSource.match(/async function applyStrategyPick[\s\S]*?\n}\n/)?.[0] ?? '';
    expect(oneClick).toContain('sameConfigText(editor.value, config.content)');
    expect(applyPicked).toContain('sameConfigText(editor.value, config.content)');
  });

  it('keeps the UDP filter action content-sized instead of forcing the icon button dimensions', () => {
    expect(panelSource).toContain("addUdp.className = 'profile-filter-add-text'");
    expect(panelSource).toContain("confirm.className = 'profile-filter-add-text'");
    expect(panelSource).toContain("cancel.setAttribute('aria-label', 'Отменить добавление UDP-порта')");
    expect(panelSource).toContain("else if (event.key === 'Escape') { event.preventDefault(); cancel.click(); }");
    expect(panelStyles).toMatch(/\.profile-filter-add-text\s*\{[^}]*width:auto[^}]*min-height:1\.8rem/);
  });

  it('offers an explicit retry when the nfqws2 help catalog is incomplete', () => {
    expect(panelSource).toContain("retry.setAttribute('aria-label', 'Повторить загрузку справки nfqws2')");
    expect(panelSource).toContain("retry.addEventListener('click', () => void refreshRuntimeProfileCatalog(true))");
    const refresh = panelSource.match(/async function refreshRuntimeProfileCatalog[\s\S]*?\n}\n/)?.[0] ?? '';
    expect(refresh).toContain('fetchRuntimeProfileCatalog(forceRefresh)');
    expect(refresh).not.toMatch(/runtimeProfileDraft\s*=/);
  });

  it('keeps toast notifications below confirmation dialogs', () => {
    const toastClass = template.match(/id="toastWrapper" class="([^"]+)"/)?.[1] ?? '';
    const dialogClass = template.match(/id="updateModalBackdrop"[^>]*class="([^"]+)"/)?.[1] ?? '';
    expect(toastClass.split(' ')).toContain('z-[40]');
    expect(dialogClass.split(' ')).toContain('z-50');
  });

  it('keeps the 16 KB diagnostic out of the navigation and panel template', () => {
    expect(template).not.toContain('tabContent-diagnostic16k');
    expect(template).not.toContain('navBtn-diagnostic16k');
  });

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
    expect(configSection).toContain('id="configCodeEditor"');
    expect(configSection).toContain('id="nfqwsConfTextarea"');
    expect(configSection).not.toContain('conf-syntax-highlight absolute');
    expect(codeEditorSource).toContain('basicSetup');
  });

  it('applies the shared syntax palette to the NFQWS_ARGS and strategy text editors', () => {
    expect(panelStyles).toContain('.config-code-editor .conf-token-flag');
    expect(panelStyles).toContain('.config-argument-code-editor .conf-token-flag');
    expect(panelStyles).toContain('.strategy-custom-editor .conf-token-flag');
    expect(panelStyles).toContain('.config-argument-code-editor .conf-token-key');
    expect(panelStyles).toContain('.strategy-custom-editor .conf-token-value');
  });

  it('copies the current configuration with the secure-context fallback and only shows copy in the editor tab', () => {
    expect(template).toContain('id="copyConfigTextButton"');
    expect(template.indexOf('data-action-click="applyConfigAndRestart"')).toBeLessThan(template.indexOf('id="configSettingsPanel"'));
    expect(panelSource).toContain("copyButton?.classList.toggle('hidden', section !== 'editor')");
    expect(panelSource).toContain('await writeTextToClipboard(editor.value)');
    expect(panelSource).toContain("toast('Конфигурация скопирована.')");
    expect(panelSource).not.toContain('navigator.clipboard.writeText(editor.value)');
  });

  it('offers themed ISP interface choices and a final manual-entry option', () => {
    expect(template).toContain('id="ispInterfaceSelect"');
    expect(template).not.toContain('data-native-select="true"');
    expect(template).toContain('data-placeholder="Выберите интерфейс…"');
    expect(template).toContain('id="ispInterfaceManualInput"');
    expect(template).not.toContain('Интерфейсы не обнаружены; введите имя вручную.');
    expect(template).not.toContain('настроен, но не обнаружен');
    expect(panelSource).not.toContain("new Option('Интерфейс не задан'");
    expect(panelSource).toContain('networkInterfaceChoices(networkInterfaces, configured, defaultInterfaces)');
    expect(panelSource).toContain("new Option('Ввести имя вручную…', '__manual__')");
    expect(template).toContain('data-action-click="submitIspInterfaceManually"');
    expect(template).toContain('placeholder="Например, eth3 или pppoe-wan"');
    expect(panelSource).toContain("$('#ispInterfaceManualInput')?.addEventListener('keydown'");
    expect(panelSource).toContain('handleIspInterfaceSelection');
    expect(panelSource).toContain('updateIspInterfaceManually');
    expect(template).not.toContain('text-slate-400">WAN</span>');
    expect(panelSource).toContain('isValidNetworkInterfaceSelection(value)');
    expect(backendSource).toContain('default_network_interfaces: default_network_interfaces(');
    expect(panelSource).toContain('nfqwsConfig.preview(before, key, value)');
    const editorInput = panelSource.slice(panelSource.indexOf('handleConfInput: () => {'), panelSource.indexOf('toggleWordWrapConf:'));
    expect(editorInput).toContain('updateQuickParameters(editor.value, discoveredNetworkInterfaces, defaultNetworkInterfaces)');
  });

  it('keeps the community page free of the fallback host field and empty-result card', () => {
    expect(template).not.toContain('id="communityTestHost"');
    expect(panelStyles).not.toContain('.community-test-host');
    expect(panelSource).not.toContain("empty.className = 'community-empty-state'");
    expect(panelSource).toContain('Укажите домен для проверки этой стратегии в поле «Сайт»');
    expect(panelSource).toContain("input.focus()");
    expect(panelStyles).toContain('.community-provider-actions .backup-action');
    expect(panelStyles).toContain('font-family:var(--ui-font-sans)');
    expect(panelStyles).toContain('min-height:2.35rem');
  });

  it('does not show a redundant custom-profile badge in the strategy constructor', () => {
    expect(panelSource).not.toContain("profile.editable ? 'Свой'");
    expect(panelSource).toContain("profile.editable ? 'Пользовательский профиль'");
  });

  it('removes the disconnected nfqws2 flags reference from configuration settings', () => {
    expect(template).not.toContain('Справочник флагов nfqws2');
    expect(template).not.toContain('docItemsContainer');
    expect(panelSource).not.toContain('filterDocs:');
  });

  it('restores profile constructor logic from v1.2.3 while allowing full profile labels to wrap', () => {
    expect(panelSource).toContain("name.className = 'truncate font-medium'");
    expect(panelSource).toContain('moveRuntimeProfileUp');
    expect(panelSource).not.toContain('runtimeProfileDisplayNames');
    expect(panelSource).toContain("sideHeading.textContent = 'Профили'");
    expect(panelSource).toContain("sideHeading.className = 'px-2 pb-1 text-[10px] font-bold text-slate-400'");
    expect(panelSource).not.toContain('summarizeProfileSelection(profile.effectiveArgs');
    expect(panelSource).not.toContain('nameWrap.append(name, summary)');
    expect(panelSource).toContain('prepareVisibleRuntimeProfiles([...custom, ...managed])');
    expect(panelStyles).toContain('#profileList button > span:nth-child(2) > span { display:block; overflow:visible; text-overflow:clip; white-space:normal; overflow-wrap:anywhere;');
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
    expect(panelSource).toContain('field.rows = layout.visibleRows');
    expect(panelSource).not.toContain('config-argument-highlight');
    expect(panelSource).not.toContain('config-argument-line-numbers');
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
    expect([...template.matchAll(/<select\b/g)]).toHaveLength(19);
    expect(panelSource).toContain('installStyledSelects(document)');
    expect(styledSelectSource).toContain("setAttribute('role', 'combobox')");
    expect(styledSelectSource).toContain("setAttribute('role', 'listbox')");
    expect(styledSelectSource).toContain("menu.setAttribute('aria-hidden', 'true')");
    expect(styledSelectSource).toContain("menu.removeAttribute('aria-hidden')");
    expect(styledSelectSource).toContain("new Event('change', { bubbles: true })");
    expect(styledSelectSource).toContain('MutationObserver');
  });

  it('keeps dropdowns above clipped panels and uses the XKeen menu palette', () => {
    expect(styledSelectSource).toContain('menu.style.left = `${left}px`');
    expect(styledSelectSource).toContain('menu.style.width = `${width}px`');
    expect(styledSelectSource).toContain('current.menu.contains(target)');
    expect(panelStyles).toContain('.styled-select-menu { position: fixed;');
    expect(panelStyles).toContain('border-radius: .55rem;');
    expect(panelStyles).toContain('background: var(--panel-surface,#111827);');
    expect(panelStyles).toContain('var(--accent-color,#3b82f6) 15%');
  });

  it('keeps explanatory and data text readable across panel sections and themes', () => {
    expect(panelStyles).toContain('font-size: max(0.75rem, 1em);');
    expect(panelStyles).toContain('html[data-theme="light"] { color-scheme: light; }');
    expect(panelStyles).toContain('backdrop-filter: blur(14px);');
    expect(panelStyles).toContain('animation: select-menu-in 150ms var(--ease-out) both;');
    expect(panelStyles).toContain(".styled-select-option[aria-selected='true'] { background:color-mix(in srgb,var(--accent-color,#3b82f6) 10%,transparent); color:var(--accent-border,#bfdbfe); font-weight:650; }");
    expect(panelStyles).toContain(".styled-select-option[data-active='true'] { outline:none; background:color-mix(in srgb,var(--accent-color,#3b82f6) 15%,transparent); color:var(--accent-border,#60a5fa); }");
    expect(panelStyles).toContain('.styled-select-menu[data-side=\'top\']');
    expect(styledSelectSource).toContain('styledSelectMenuSide');
    expect(styledSelectSource).toContain('state.activeIndex = -1;');
    expect(styledSelectSource).toContain("item.dataset.active = String(state.open && index === state.activeIndex)");
    expect(panelStyles).toContain('#toastWrapper > .toast-card.is-removing { animation: toast-out');
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
    expect(section).toContain('multiple');
    expect(section).toContain('data-action-click="openForeignConfigFile"');
    expect(section).not.toContain('＋ Текстом');
    expect(section).not.toContain('＋ По ссылке');
    expect(section).toContain('Файлы .conf/.txt');
    expect(section).toContain('accept=".conf,.txt,text/plain"');
    expect(section).toContain('файлами .conf/.txt');
    expect(section).toContain('id="foreignConfigHosts"');
    expect(section).toContain('id="foreignConfigRepeats"');
    expect(section).toContain('id="foreignConfigResults"');
    expect(section).toContain('до 10 вариантов');
    expect(section).toContain('(1–15, домен или публичный IPv4)');
    expect(panelSource).toContain('foreignConfigInputs.length >= 10');
    expect(panelSource).toContain('isSupportedForeignConfigFileName(file.name)');
    expect(panelSource).toContain('Array.from(foreignFileInput.files ?? [])');
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
    expect(panelSource).toContain("heading.className = 'flex min-h-11 flex-wrap items-center justify-between gap-2 px-3 py-2'");
    expect(panelSource).toContain('if (action) headingActions.append(action)');
    expect(panelSource).toContain("details.className = 'border-t border-slate-800/80'");
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

  it('does not expose the deferred support report tools in the current UI', () => {
    expect(template).not.toContain('Отчёт для помощи');
    expect(template).not.toContain('id="supportReportText"');
    expect(panelSource).not.toContain('buildSupportReport');
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

  it('does not render the 16 KB diagnosis panel', () => {
    const picker = template.slice(template.indexOf('id="tabContent-autotune"'), template.indexOf('id="tabContent-monitor"'));
    expect(picker).toContain('flex flex-col lg:flex-row');
    expect(picker).toContain('strategy-settings-grid');
    expect(picker).toContain('strategy-target-transport-row');
    expect(panelStyles).toContain('.strategy-target-transport-row:not(.hidden) { display:grid; grid-template-columns:minmax(0,1fr) minmax(13rem,.62fr);');
    expect(panelStyles).toContain('.autotune-settings-card { padding: .75rem 1rem; }');
    expect(panelStyles).toContain('.strategy-settings-grid { max-width: 980px; gap:.15rem; }');
    expect(panelStyles).toContain('.strategy-settings-grid > div.strategy-setting-row:not(.hidden) { min-width:0; display:grid;');
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
    expect(template).not.toContain('tabContent-diagnostic16k');
    expect(template).not.toContain('freezeScanHostsInput');
    expect(panelStyles).toContain('.freeze-result-address { min-width:0;');
    expect(panelStyles).toContain('font-size:.86rem; font-variant-numeric:tabular-nums;');
    expect(panelStyles).toContain('.freeze-result-host');
    expect(panelSource).toContain('function syncConfigWordWrap(enabled: boolean)');
    expect(panelSource).toContain('wrapConfigLines: enabled');
    expect(panelSource).toContain('const { wrapConfigLines: _wrapConfigLines, ...routerSettings } = settings;');
    expect(panelSource).toContain('nfqwsAppearance.save(routerAppearanceSettings(appearanceSettings))');
    expect(template).toContain('id="configWordWrapToggle"');
    expect(panelSource).toContain('freezeRowIdentity(result)');
    expect(panelSource).toContain('freezeRowDetails(result)');
    expect(panelSource).toContain("operation.stage === 'strategy_search'");
    expect(panelSource).toContain('freezeProgressPresentation(operation)');
    expect(panelSource).toContain("status.classList.toggle('hidden', !active)");
    expect(panelSource).toContain('Замеры и детали');
    expect(panelSource).toContain('result.address ??');
    expect(panelSource).toContain('freezeStrategyPreview');
    expect(panelSource).toContain('JSON.stringify({ state: operation.state, results: operation.results })');
    expect(panelSource).toContain('freezeCanApplyToHost');
    expect(panelSource).toContain('freezeInConfigNote(result.inConf)');
    expect(panelSource).toContain('inConfState: result.inConf?.state');
    expect(panelStyles).toContain('.freeze-result-config-note');
    expect(panelSource).toContain('freeze-result-badge');
    expect(panelSource).toContain('Применить к сайту');
    expect(panelSource).toContain('applyFreezeStrategyOneClick');
    expect(panelSource).toContain('Проверенный вариант добавляется только этому домену');
    expect(panelStyles).toContain('@keyframes freeze-row-enter');
    expect(panelStyles).toContain('.freeze-result-details summary');
    expect(panelStyles).toContain('prefers-reduced-motion: reduce');
    expect(panelSource).toContain("status.setAttribute('aria-busy'");
    expect(panelSource).toContain("row.setAttribute('role', 'listitem')");
    expect(panelSource).toContain("document.createElement('article')");
    expect(panelSource).not.toContain("document.createElement('table')");
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

  it('offers single-site QUIC strategy picking with clear transport scope', () => {
    expect(template).toContain('<option value="quic">QUIC · UDP 443</option>');
    expect(panelSource).toContain('Проверяется ответ на QUIC Initial по UDP 443 для одного домена с TLS SNI');
    expect(panelSource).toContain("['http', 'https', 'quic'].includes(proto.value)");
    expect(panelSource).toContain('QUIC-подбор требует доменное имя');
    expect(panelSource).toContain("operation.protocol === 'quic' ? 'QUIC уже отвечает напрямую'");
    expect(panelStyles).toContain('.strategy-target-transport-row:not(.hidden)');
    expect(panelStyles).toContain('.strategy-target-transport-row > label:not(.hidden)');
    expect(panelStyles).toContain('.strategy-settings-grid > div.strategy-setting-row:not(.hidden)');
  });

  it('uses one editor view for config text, gutter, syntax and selection rendering', () => {
    expect(codeEditorSource).toContain('basicSetup');
    expect(codeEditorSource).toContain('basicSetup');
    expect(codeEditorSource).toContain('lineWrapping');
    expect(codeEditorSource).toContain('syntaxPlugin');
    expect(codeEditorSource).toContain('view.visibleRanges');
    expect(codeEditorSource).toContain('fontFeatureSettings');
    expect(codeEditorSource).toContain('"calt" 0');
    expect(codeEditorSource).toContain('fontVariantLigatures: \'none\'');
    expect(codeEditorSource).toContain('"liga" 0, "clig" 0, "calt" 0');
    expect(codeEditorSource).toContain("'.cm-activeLine': { backgroundColor: 'transparent' }");
    expect(panelStyles).toContain('.config-code-editor ::selection, .config-argument-code-editor ::selection, .strategy-custom-editor ::selection { background:transparent !important; color:inherit !important; }');
    expect(codeEditorSource).toContain('update.docChanged || update.viewportChanged');
    expect(codeEditorSource).toContain('.cm-selectionBackground');
    expect(codeEditorSource).toContain('isLargeConfigDocumentLength(view.state.doc.length)');
    expect(codeEditorSource).toContain("host.classList.toggle('config-editor-large'");
    expect(panelStyles).toContain('.config-editor-large .cm-gutters { display:none !important; }');
    expect(appSource).not.toContain('confSyntaxHighlight');
    expect(panelSource).toContain('setConfigCodeEditorContent(value)');
    expect(panelSource).toContain('selectConfigText(start, end < 0 ? content.length : end, warning.line ?? undefined)');
    expect(panelSource).toContain('if (fullscreen) getConfigCodeEditor()?.focus()');
  });

  it('updates only the active outline item while selection moves', () => {
    expect(panelSource).toContain("configHost.addEventListener('configselection', updateActiveConfigOutline)");
    expect(panelSource).toContain('if (content === configOutlineRenderedContent) { updateActiveConfigOutline(); return; }');
    expect(panelSource).toContain("button.dataset.configOutlineOffset = String(entry.offset)");
    expect(panelSource).toContain('handleConfInput: () => {');
    expect(panelSource).toContain('updateQuickParameters(editor.value, discoveredNetworkInterfaces, defaultNetworkInterfaces)');
    expect(panelSource).not.toContain("$('#nfqwsConfTextarea')?.addEventListener('input'");
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
    expect(panelSource).toContain('requestDangerousConfirmation(\'Применить стратегию?\', `${applyScope}');
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

  it('places core and UI release notices in a shared bottom-right notification area', () => {
    expect(template).toContain('id="updateNoticeWrapper"');
    expect(template).toContain('fixed bottom-5 right-5');
    expect(panelSource).toContain("const host = $('#updateNoticeWrapper')");
    expect(panelSource).toContain("item.id = 'coreUpdateNotice'");
    expect(panelSource).toContain("item.id = 'uiUpdateNotice'");
    expect(panelSource).toContain('nfqwsUpdates.coreStatus()');
    expect(backendSource).toContain('/api/nfqws2/updates/status');
  });

  it('uses the shared release notice style and a white success checkmark for confirmations', () => {
    expect(panelSource).toContain("icon.classList.add('h-[18px]', 'w-[18px]', 'shrink-0', 'text-slate-100')");
    expect(panelStyles).toMatch(/\.toast-card-icon-success\s*\{[^}]*color:\s*#f8fafc/);
    expect(panelStyles).not.toMatch(/\.toast-card-icon-success\s*\{[^}]*color:\s*#34d399/);
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
    expect(template).toContain('история до 96 циклов');
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
    expect(panelSource).toContain('Проверяются ${hosts.length}');
    expect(panelSource).toContain('catch (error) {\n    const message = error instanceof Error ? error.message');
    expect(panelSource).toContain("monitoringHostsRunning.has(site.host) ? 'Проверяем…'");
    expect(panelStyles).toContain('.monitor-site-card { min-width:0;');
    expect(template).toContain('navBtn-community');
    expect(template).toContain('tabContent-community');
    expect(template).not.toContain('asnNumber');
    expect(backendSource).toContain('/api/community-strategies');
    expect(backendSource).not.toContain('/api/nfqws2/asn-prefixes');
    expect(panelSource).toContain('includeCommunity');
    expect(template).toMatch(/id="navBtn-autotune" class="(?![^"]*hidden)[^"]*side-nav-btn/);
    expect(template).toContain('data-action-click="startStrategyPick"');
    expect(template).not.toContain('id="tabContent-diagnostic16k"');
    expect(template).not.toContain('data-action-click="runFreezeScan"');
    expect(panelSource).not.toContain('UNAVAILABLE_PANEL_ROUTES');
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
    expect(tooltipSource).toContain("root.addEventListener('click', clearActiveTooltip, true)");
    expect(panelStyles).toContain('.action-tooltip');
  });

  it('exposes the profiles, backups, change history, and logs pages through the backend', () => {
    for (const route of ['profiles', 'snapshots', 'changes', 'logs']) {
      expect(template).toContain(`id="navBtn-${route}"`);
      expect(template).toContain(`id="tabContent-${route}"`);
    }
    expect(panelSource).not.toContain("UNAVAILABLE_PANEL_ROUTES = ['supportReport']");
    expect(panelSource).toContain('nfqwsSnapshots.create(');
    expect(backendSource).toContain('"/api/snapshots"');
    expect(backendSource).toContain('get(get_snapshots).post(create_snapshot_api)');
    expect(backendSource).toContain('delete_snapshot_api');
    expect(panelSource).toContain('nfqwsProfiles.saveCustom(runtimeProfileDraft');
    expect(template).toContain('id="profileEditor"');
    expect(panelStyles).toContain('#profileEditor { container-type:inline-size; }');
    expect(panelStyles).toMatch(/@container \(max-width:640px\)\s*\{\s*\.profile-step-grip \{ display:none; \}\s*\.profile-step-row \{ display:flex; flex-wrap:wrap;/);
    expect(template).toContain('id="profileImportDialog"');
    expect(panelSource).toContain('runtimeProfileDraft.push');
    expect(panelSource).toContain('moveRuntimeProfileUp');
    expect(panelSource).toContain('deleteRuntimeProfile');
    expect(panelSource).toContain("['builder', 'Конструктор']");
    expect(panelSource).toContain("['text', 'Текст']");
    expect(panelSource).toContain('appendRuntimeProfileArgument');
    expect(panelSource).not.toContain("const sections: Array<[string, string[]]> = [['Фильтры', profile.filters]");
    expect(panelSource).not.toContain("['Упорядоченные шаги', profile.steps]");
    expect(panelSource).toContain("if (profile.hasConstructor && runtimeProfileEditorMode === 'builder')");
    expect(panelSource).toContain('getRuntimeProfileConstructorTarget(profile)');
    expect(panelSource).toContain('getRuntimeProfileListTarget(profile, choice.value)');
    expect(panelSource).toContain('profileProtocolChoices(runtimeProfileCliCatalog?.l7 ?? [], profile.args)');
    expect(panelSource).toContain("'+ добавить фильтр UDP-порты'");
    expect(panelSource).toContain('appendProfileUdpFilter(target?.args ?? profile.args, port.value)');
    expect(panelSource).toContain("else if (runtimeProfileEditorMode === 'text')");
    expect(panelSource).toContain("mountConfigArgumentEditor(textEditor, profile.args.join('\\n'");
    expect(panelSource).toContain('const components = profile.components ?? []');
    expect(panelSource).toContain('saveSettings(runtimeProfileManagedDraft');
    expect(panelSource).toContain("results.setAttribute('role', 'listbox')");
    expect(panelSource).toContain("section.setAttribute('role', 'group')");
    expect(panelSource).toContain('nextProfileChoiceIndex(options.indexOf(option), event.key, options.length)');
    expect(panelSource).toContain('removeRuntimeProfileArgument');
    expect(backendSource).toContain('.put(save_runtime_profiles)');
    expect(backendSource).toContain('NFQWS_ARGS_CUSTOM');
    expect(backendSource).toContain('"/api/change-history"');
    expect(backendSource).toContain('"/api/change-history/{id}/diff"');
    expect(backendSource).toContain('"/api/change-history/{id}/revert"');
    expect(backendSource).toContain('"/api/change-history/undo-last"');
    expect(backendSource).toContain('"/api/change-history/initial"');
    expect(template).toContain('id="changeHistoryUndoLast"');
    expect(template).toContain('id="changeHistoryRestoreInitial"');
    expect(template).toContain('class="ui-card rounded-xl p-3 sm:p-4"');
    expect(template).toContain('class="change-history-controls"');
    expect(panelStyles).toContain('.change-history-controls .backup-action { min-height:2.35rem;');
    expect(panelStyles).toContain('.change-history-controls .backup-action:disabled { opacity:.78;');
    expect(panelStyles).toContain('.change-history-actions .backup-action { min-height:2.15rem;');
    expect(panelStyles).toContain('.change-history-file { color: var(--text-primary');
    expect(panelStyles).toContain('font-size: .94rem; font-weight: 680');
    expect(panelSource).toContain("diffButton.textContent = 'Что изменилось'");
    expect(panelSource).toContain('nfqwsChangeHistory.diff(id)');
    expect(panelSource).toContain('nfqwsChangeHistory.revert(id)');
    expect(panelSource).toContain('nfqwsLogs.get(source)');
    expect(panelSource).toContain('shouldPollLogs(sectionVisible, autoRefresh, logRequestPending)');
    expect(template).toContain('id="logsAutoRefresh"');
    expect(template).toContain('id="snapshotAutoEnabled"');
    expect(template).toContain('toggleAutomaticSnapshots');
    expect(template).toContain('Автоматически создавать копии');
    expect(template).toContain('id="operationConfirmModal"');
    expect(panelSource).toContain('requestDangerousConfirmation');
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

  it('links the community strategy source transparently', () => {
    expect(template).toContain('https://github.com/zemidala/nfqws2-strategies');
  });

  it('keeps the panel structure and exposes the two settings subsections', () => {
    expect(template).toContain('id="tabContent-conf"');
    expect(template).toContain('id="tabContent-lists"');
    expect(template).toContain('Настройки и оформление UI');
    expect(template).toContain('id="settingsAppearancePanel"');
    expect(template).toContain('id="settingsSecurityPanel"');
    expect(template).toContain('id="simplifiedNavigation"');
    expect(template).toContain('value="simplified"');
    expect(template).not.toContain('data-appearance-simplified-navigation');
    expect(template).toContain('data-simplified-group="site-check"');
    expect(panelSource).not.toContain("UNAVAILABLE_PANEL_ROUTES = ['supportReport']");
    expect(template).toContain('id="panelAuthEnabled"');
    expect(template).toContain('data-action-click="savePanelAuthSettings"');
    expect(entrySource).toContain('nfqwsAuth.status()');
    expect(appSource).toContain('Забывать вход через');
    expect(appSource).toContain('Никогда');
    expect(entrySource).toContain('checked={remember}');
    expect(backendSource).toContain('/api/auth/login');
    expect(backendSource).toContain('auth::AuthManager::middleware');
  });

  it('copies the exact About command from its row and confirms the result accessibly', () => {
    expect(template).toContain('id="aboutCopyStatus"');
    expect(template).toContain('aria-label="Копировать команду"');
    expect(template).toContain('data-about-copy-label="true"');
    expect(appSource).toContain('JSON.stringify([{source:"self"}])');
    expect(panelSource).toContain("closest('.about-command-row')?.querySelector('code')?.textContent?.trim()");
    expect(panelSource).toContain("copyStatus.textContent = 'Команда скопирована в буфер обмена.'");
    expect(panelSource).toContain("button.classList.add('about-copy-success')");
    expect(panelSource).toContain("button.disabled = true");
    expect(panelSource).toContain("label.textContent = 'Скопировано'");
    expect(panelStyles).toContain('.about-copy-button.about-copy-success');
    expect(panelStyles).toContain('@keyframes about-copy-check');
  });

  it('places About after the other pages and as the last system navigation item', () => {
    expect(template.indexOf('id="tabContent-logs"')).toBeLessThan(template.indexOf('id="tabContent-about"'));
    expect(template.indexOf('id="navBtn-logs"')).toBeLessThan(template.indexOf('id="navBtn-about"'));
    expect(template).toContain('about-version-grid');
    expect(template).toContain('about-command-title');
    expect(template).toContain('Установить последнюю стабильную версию панели.');
    expect(template).toContain('Показать версии панели и nfqws2, а также проверить доступность веб-панели.');
    expect(template).not.toContain('команда панели rangerxxyy; штатное управление nfqws2 не меняется');
    expect(panelStyles).toContain('font-size:.84rem; font-weight:650; line-height:1.55; white-space:pre');
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

  it('streams real site diagnosis results into the checklist as each probe completes', () => {
    const diagnosisRequest = panelSource.slice(panelSource.indexOf('async function runSiteDiagnosis()'), panelSource.indexOf('function renderTraceReport('));
    expect(diagnosisRequest).toContain('nfqwsDiagnostics.siteStream(input.value');
    expect(diagnosisRequest).toContain('renderDiagnosisProgress(container');
    expect(diagnosisRequest).not.toContain('waitForDiagnosisAnimation');
    expect(panelStyles).toContain('.diagnosis-live-checks .diagnosis-row.is-active');
    expect(panelStyles).toContain('.diagnosis-progress-spinner');
  });

  it('keeps the live diagnosis checklist aligned with the final checklist', () => {
    const liveRows = panelSource.slice(panelSource.indexOf('function renderDiagnosisProgress('), panelSource.indexOf('function renderSiteDiagnosis('));
    expect(liveRows).toContain("diagnosis-checks-header");
    expect(liveRows).toContain("diagnosis-checks-host");
    expect(panelSource).toContain("const stages: { id: string; label: string }[] = [...SITE_DIAGNOSIS_STAGES]");
    expect(panelStyles).toContain('.diagnosis-results .diagnosis-checks { height:28rem; max-height:28rem; overflow-y:auto;');
    expect(panelStyles).toMatch(/\.diagnosis-results \.diagnosis-live-checks \.diagnosis-row\s*\{[^}]*padding\s*:\s*\.42rem 0/);
    expect(panelSource).toContain("import { diagnosisRows } from './diagnosis-layout'");
  });

  it('shows installed Lua catalog warnings before Apply without converting them into blocking errors', () => {
    expect(panelSource).toContain('diagnoseConfigLuaArguments(draft');
    expect(panelSource).toContain("severity: 'warning'");
    expect(panelSource).toContain('warnings.length || validation.checks.installedBinary === \'not_run\'');
  });

  it('separates diagnosis duration from the result text in live and final rows', () => {
    const liveRow = panelSource.slice(panelSource.indexOf('function renderDiagnosisProgress('), panelSource.indexOf('function renderSiteDiagnosis('));
    const finalRow = panelSource.slice(panelSource.indexOf('function renderSiteDiagnosis('), panelSource.indexOf('async function runSiteDiagnosis('));
    expect(liveRow.indexOf('body.append(result)')).toBeLessThan(liveRow.indexOf('body.append(timing)'));
    expect(finalRow.indexOf('body.append(detail)')).toBeLessThan(finalRow.indexOf('body.append(metrics)'));
    expect(panelStyles).toMatch(/\.diagnosis-live-checks \.diagnosis-row-metrics\s*\{[^}]*margin\s*:\s*0 0 0 \.4rem/);
    expect(panelStyles).toMatch(/\.diagnosis-live-checks \.diagnosis-row-metrics\s*\{[^}]*border-left\s*:\s*1px solid/);
  });

  it('keeps the full final diagnosis checklist compact enough to reduce page scrolling', () => {
    expect(panelStyles).toMatch(/\.diagnosis-results \.diagnosis-checks-header\s*\{[^}]*padding\s*:\s*\.7rem 1\.1rem/);
    expect(panelStyles).toMatch(/\.diagnosis-results \.diagnosis-row\s*\{[^}]*padding\s*:\s*\.42rem 0/);
    expect(panelStyles).toMatch(/\.diagnosis-results \.diagnosis-row-note\s*\{[^}]*line-height\s*:\s*1\.4/);
  });

  it('styles checkbox inputs while keeping their native keyboard semantics', () => {
    expect(panelStyles).toContain('input[type="checkbox"] {');
    expect(panelStyles).toContain('input[type="checkbox"]:checked::before');
    expect(panelStyles).toContain('input[type="checkbox"]:focus-visible');
    expect(panelStyles).toContain('[data-theme="light"] input[type="checkbox"]');
    expect(panelStyles).toContain('input[type="checkbox"]::before {');
    expect(panelStyles).toContain('input[type="checkbox"]:checked { border-color:var(--accent-color,#3b82f6); background:var(--accent-strong,#2563eb);');
    expect(panelStyles).toContain('input[type="checkbox"]:focus-visible { outline:2px solid var(--accent-border,#93c5fd);');
    expect(panelStyles).toContain('[data-theme="light"] input[type="checkbox"]:not(:checked) { border-color: #94a3b8; background: #fff; }');
    expect(panelStyles).toContain('[data-theme="neon"] input[type="checkbox"]:not(:checked) { border-color: #a855f7; }');
    expect(panelStyles).not.toContain('[data-theme="neon"] input[type="checkbox"]:checked { border-color:#d946ef;');
  });

  it('colors scrollbars from the selected accent while preserving light-theme contrast', () => {
    expect(panelStyles).toContain('scrollbar-color:var(--scrollbar-thumb) var(--scrollbar-track)');
    expect(panelStyles).toContain('--scrollbar-thumb:color-mix(in srgb,var(--accent-color) 38%,#1e293b)');
    expect(panelStyles).toContain('[data-theme="light"] { --scrollbar-track:#e2e8f0;');
    expect(panelStyles).toContain('[data-theme="neon"] { --scrollbar-track:#100c19;');
    expect(panelStyles).toContain('--scrollbar-thumb:color-mix(in srgb,var(--accent-color) 42%,#49315f)');
    expect(panelStyles).toMatch(/::-webkit-scrollbar-thumb:hover\s*\{\s*background:\s*var\(--scrollbar-thumb-hover/);
    expect(panelStyles).not.toContain('[data-theme="light"] ::-webkit-scrollbar-thumb { background:#94a3b8; }');
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
    expect(panelStyles).toContain('.appearance-choice, .appearance-layout-choice, .appearance-accent-choice { transition: none; }');
    expect(panelStyles).toContain('.appearance-choice:hover, .appearance-layout-choice:hover, .appearance-accent-choice:hover { transform: none; }');
    expect(panelStyles).toContain('[data-layout="right"] .app-main-layout');
    expect(panelStyles).toContain('[data-layout="top"] .app-main-layout');
    expect(panelStyles).toContain('[data-layout="tiles"] .app-main-layout');
    expect(template).toContain('Синий (стандартная)');
  });

  it('keeps selection painting inside the active editor layer', () => {
    expect(codeEditorSource).toContain('.cm-selectionBackground');
    expect(codeEditorSource).toContain("lineHeight: compact ? '1.25rem' : '1.5'");
    expect(panelStyles).toContain('.config-code-editor .cm-selectionBackground');
    expect(panelStyles).toContain('::selection { background:#2d4f8e; color:inherit; }');
    expect(panelStyles).toContain('[data-theme="light"] ::selection { background:#dbeafe; color:inherit; }');
    expect(panelStyles).not.toContain('.cm-gutters { border-right:');
    expect(panelStyles).not.toContain('color:transparent !important; -webkit-text-fill-color:transparent !important;');
    expect(panelSource).not.toContain('highlight.innerHTML = input.value');
    expect(panelStyles).not.toContain('selection:bg-blue-500/30');
    expect(appSource).not.toContain('selection:bg-blue-500/30');
    expect(entrySource).not.toContain('selection:bg-blue-500/30');
  });

  it('keeps the accent focus halo off text editors and line-number gutters', () => {
    expect(panelStyles).toContain('.mobile-list-editor textarea:focus {');
    expect(panelStyles).toContain('.config-argument-editor:focus-within { border-color:var(--accent-border,#60a5fa);');
  });

  it('uses the selected accent for config editor caret and structure focus states', () => {
    expect(appSource).toContain('config-section-indicator h-2.5 w-2.5 rounded-full');
    expect(panelStyles).toContain('.config-section-indicator { background:var(--accent-color,#3b82f6);');
    expect(codeEditorSource).toContain('caretColor: \'var(--accent-color, #60a5fa)\'');
    expect(codeEditorSource).toContain('caretColor: \'var(--accent-color, #60a5fa)\'');
    expect(panelStyles).toContain('.config-editor-icon-button:focus-visible { outline:2px solid var(--accent-border,#60a5fa);');
    expect(panelStyles).toContain('.config-structure-toggle-active { border-color:var(--accent-border,#60a5fa);');
    expect(panelStyles).toContain('.config-outline-entry:focus-visible { outline:2px solid var(--accent-border,#60a5fa);');
    expect(panelStyles).not.toContain('.conf-editor-input { z-index:1; background:transparent !important; color:transparent !important; -webkit-text-fill-color:transparent !important; caret-color:#60a5fa;');
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
    expect(modeTile).toContain('id={"strategyCustomStepsEditor"}');
    expect(panelSource).toContain("mountConfigArgumentEditor(strategyCustomHost");
    expect(panelSource).toContain('strategyCustomCodeEditor');
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
    expect(lists).toContain('id="listEditorViewSelect"');
    expect(lists).toContain('option value="rows">Списком');
    expect(panelSource).toContain("localStorage.setItem(LIST_EDITOR_VIEW_KEY, view)");
    expect(panelSource).toContain('listRowsFromContent(editor.value');
    expect(panelSource).toContain('removeListLinesByIndices(editor.value');
    expect(panelSource).not.toContain('findNextListEntry');
    expect(appSource).toContain('aria-label={"Вид списка"}');
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
    expect(appSource).toContain('data-about-version-command={index === 1 ? "true" : undefined}');
    expect(panelSource).toContain('nfqws2-ui-ctl update ${tag} --force');
    expect(panelSource).toContain('^v(?:0|[1-9][0-9]*)\\.(?:0|[1-9][0-9]*)\\.(?:0|[1-9][0-9]*)$');
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
    expect(panelStyles).toContain('max-height: none; overflow: visible;');
    expect(template).toContain('Списки доменов и IP');
    expect(template).toContain('id="navBtn-snapshots"');
  });

  it('uses native page scrolling for vertical touch gestures on phones', () => {
    expect(panelStyles).toContain('body.h-screen.overflow-hidden { height: auto !important; min-height: 100dvh; overflow-x: hidden !important; overflow-y: auto !important;');
    expect(panelStyles).toContain('.app-main-layout.flex-1.min-h-0.overflow-hidden { flex: 0 0 auto !important; min-height: auto !important; overflow: visible !important;');
    expect(panelStyles).toContain('.app-main-layout > main { width: 100%; min-width: 0; min-height: auto; flex: 0 0 auto;');
    expect(panelStyles).toContain('overflow-x: visible; overflow-y: visible; touch-action: pan-y;');
    expect(panelStyles).toContain('-webkit-overflow-scrolling: touch;');
  });

  it('does not ship example credentials in login controls', () => {
    expect(template).not.toContain('authPassword');
    expect(appSource).toContain('id={"panelAuthPassword"}');
    expect(template).not.toContain('7194028410:AAH8x9kLmP2qW_nfqws2_router_bot');
  });

  it('keeps remember-me duration in panel settings and uses only a checkbox at login', () => {
    expect(entrySource).not.toContain('id="rememberDays"');
    expect(entrySource).toContain('Запомнить меня');
    expect(appSource).toContain('id={"panelAuthRememberDays"}');
    expect(appSource).toContain('option value={"0"}>Никогда');
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

  it('provides compact list draft tools, clear confirmation, and fullscreen controls without search', () => {
    expect(template).not.toContain('id="listAddEntriesInput"');
    expect(template).not.toContain('id="listSearchInput"');
    expect(template).not.toContain('id="findNextListEntryButton"');
    expect(template).toContain('id="listEditorViewSelect"');
    expect(template).not.toContain('id="listEditorViewGroup"');
    expect(template).toContain('id="listEditorFullscreenToggle"');
    expect(template).toContain('data-action-click="requestClearListDraft"');
    expect(template).toContain('id="listClearModal"');
    expect(template).toContain('id="listEditorPanel"');
    expect(template).not.toContain('data-action-click="appendListEntriesToDraft"');
    expect(template).toContain('id="listImportFile"');
    expect(template).toContain('data-action-click="removeSelectedListLines"');
    expect(template).toContain('data-action-click="exportListDraft"');
    expect(template).not.toContain('list-editor-secondary-toolbar');
    const listToolbar = template.slice(template.indexOf('class="list-editor-toolbar-actions'), template.indexOf('id="listEditorFullscreenToggle"'));
    expect(listToolbar).toContain('class="list-toolbar-menu"');
    expect(listToolbar).toContain('removeSelectedListLines');
    expect(listToolbar).toContain('deduplicateCurrentList');
    expect(listToolbar).toContain('openListImport');
    expect(listToolbar).toContain('exportListDraft');
    expect(template).not.toContain('injectPresetDomains');
    expect(panelSource).toContain('nfqwsLists.save(activeList.id, editor.value, activeList.revision)');
    expect(panelSource).toContain('function toggleListEditorFullscreen()');
    expect(panelSource).toContain('function confirmClearListDraft()');
    expect(panelStyles).toContain('.list-editor-fullscreen');
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
    expect(panelSource).toContain("nfqwsUpdates.coreReleases(forceRefresh)");
    expect(panelSource).toContain("nfqwsUpdates.installCore(tag)");
    expect(panelSource).toContain("renderReleaseMarkdown(release.notes)");
    expect(panelSource).not.toMatch(/installSelectedRelease.*toast\('Эта функция пока не подключена/);
  });

  it('offers an in-place retry when a release list is empty or cannot be loaded', () => {
    expect(panelSource).toContain('function appendReleaseRetry(list: HTMLElement)');
    expect(panelSource).toContain("button.dataset.actionClick = 'retryUpdateList'");
    expect(panelSource).toContain('retryUpdateList: () => void loadUpdatePickerReleases(true)');
    expect(template).not.toContain('updatePickerRefresh');
    expect(panelStyles).toContain('.release-retry-button');
  });

  it('uses cached releases on open and refreshes automatically while the picker is visible', () => {
    expect(panelSource).toContain("list.textContent = 'Загружаю список версий…'");
    expect(panelSource).not.toContain('Загружаю версии из GitHub Releases…');
    expect(panelSource).toContain('const result = await nfqwsUpdates.coreReleases(forceRefresh)');
    expect(panelSource).toContain('await loadUpdatePickerReleases(false)');
    expect(panelSource).toContain('UPDATE_PICKER_REFRESH_INTERVAL_MS = 2 * 60 * 1000');
    expect(panelSource).toContain('void loadUpdatePickerReleases(false, true)');
    expect(panelSource).toContain('stopUpdatePickerAutoRefresh(); closeAppModal');
    expect(template).not.toContain('Проверка списка каждые 2 минуты');
    expect(backendSource).toContain('load_cached_core_releases(&state, &architecture, query.force)');
    expect(backendSource).toContain('Query(query): Query<CoreReleaseQuery>');
    expect(panelSource).not.toContain("if (updateTarget !== 'ui') return");
    expect(panelSource).toContain('retryUpdateList: () => void loadUpdatePickerReleases(true)');
  });

  it('keeps About commands and copy actions from colliding at narrow widths', () => {
    expect(panelStyles).toContain('.about-command-row { display:grid; grid-template-columns:minmax(0,1fr) auto;');
    expect(panelStyles).toContain('.about-command-row > button { white-space:nowrap; }');
    expect(panelStyles).toContain('.about-command-row code { display:block; width:fit-content; max-width:100%;');
    expect(panelStyles).toContain('overflow-x:auto;');
    expect(panelStyles).toContain('@media (max-width:560px)');
  });

  it('does not register a beforeunload warning that blocks browser reload or close', () => {
    expect(panelSource).not.toMatch(/beforeunload|returnValue/);
    expect(entrySource).not.toMatch(/beforeunload|returnValue/);
    const serviceActions = panelSource.slice(panelSource.indexOf('async function serviceAction'), panelSource.indexOf('async function loadList'));
    expect(serviceActions).not.toContain('window.confirm(');
  });

  it('checks UI updates in the background and provides a persistent dismissible notice', () => {
    expect(panelSource).toContain("nfqwsUpdates.uiStatus()");
    expect(panelSource).toContain("window.setInterval(() => { if (document.visibilityState === 'visible') void refreshUiUpdateStatus(); }, UPDATE_STATUS_POLL_INTERVAL_MS)");
    expect(panelSource).toContain("dataset.actionClick = 'openLatestUiUpdate'");
    expect(panelSource).toContain("dataset.actionClick = 'dismissUiUpdateNotice'");
    expect(panelStyles).toContain('.version-pill.update-available');
    expect(panelStyles).toContain('.ui-update-notice');
    expect(panelSource).toContain("document.createElementNS('http://www.w3.org/2000/svg', 'svg')");
    expect(panelStyles).toContain('.ui-update-notice > button { margin-left: 26px;');
  });

  it('keeps update polling quiet in the modal and uses a regular-weight install button', () => {
    expect(appSource).not.toContain('Проверка списка каждые 2 минуты');
    const modal = appSource.slice(appSource.indexOf('id={"updateModalBackdrop"}'), appSource.indexOf('id={"diffModalBackdrop"}'));
    expect(modal).not.toContain('font-bold text-sm transition-all shadow-lg shadow-blue-600/25');
    expect(modal).toContain('font-normal text-sm');
    expect(modal).toContain('max-h-[90dvh] flex flex-col');
    expect(modal).toContain('max-h-[min(31.25rem,55dvh)]');
    expect(modal).toContain('max-w-[47rem]');
    expect(modal).toContain('w-full py-2 rounded-lg');
    expect(panelSource).toContain("title.className = 'block text-sm font-medium text-slate-100'");
    expect(modal).toContain('font-semibold text-white tracking-tight');
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

  it('warns about disconnected interception hooks only from a definitive status result', () => {
    expect(panelSource).toContain('status.interception?.missingHooks');
    expect(panelSource).toContain('Перехват nfqws2 подключён не полностью');
    expect(panelSource).toContain("tone = interceptionWarning ? 'amber'");
    expect(panelSource).toContain("'Перехват частичный'");
    expect(panelSource).toContain("'Перехват отключён'");
    expect(panelSource).toContain('const missingHooks = running ? status.interception?.missingHooks ?? [] : []');
    expect(panelSource).toContain('syncServiceControl(service)');
    expect(panelStyles).toContain('.status-pill:has(#statusMainDot.bg-amber-400)');
  });

  it('labels the ISP interface quick setting consistently with the other base parameters', () => {
    expect(template).toMatch(/<span>ISP_INTERFACE<\/span>[\s\S]*?<span class="text-blue-400">WAN<\/span>[\s\S]*?<select id="ispInterfaceSelect"/);
  });

  it('removes the deferred package leftovers and support report sections from both navigation layouts', () => {
    expect(template).not.toContain('id="navBtn-packageLeftovers"');
    expect(template).not.toContain('id="navBtn-supportReport"');
    expect(template).not.toContain('id="tabContent-packageLeftovers"');
    expect(template).not.toContain('id="tabContent-supportReport"');
    expect(template).not.toContain('data-simplified-route="packageLeftovers"');
    expect(template).not.toContain('data-simplified-route="supportReport"');
    expect(panelSource).not.toContain("if (route === 'packageLeftovers') void loadPackageLeftovers()");
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
    expect(template).toContain('30 мин');
    expect(template).not.toContain('ASN не определён');
    expect(template).not.toContain('Варианты пока не загружены.');
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
    expect(panelStyles).toContain('.simplified-nav.hidden, .simplified-nav-button.hidden { display: none; }');
    expect(panelStyles).toContain('[data-accent] .simplified-nav-button.simplified-nav-active');
    expect(panelStyles).toContain('[data-theme="light"][data-accent] .monitor-subtab.active');
    expect(panelStyles).toContain('[data-accent] .appearance-choice:has(input:checked)');
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

  it('uses the selected accent for navigation glow and keyboard focus instead of fixed blue', () => {
    expect(panelStyles).toContain('background-color: var(--accent-color,#3b82f6)');
    expect(panelStyles).toContain('box-shadow: 0 0 8px color-mix(in srgb,var(--accent-color,#3b82f6) 55%,transparent)');
    expect(panelStyles).toContain('outline: 2px solid var(--accent-border,#60a5fa)');
    expect(panelStyles).toContain('box-shadow: 0 0 0 3px color-mix(in srgb,var(--accent-color,#3b82f6) 18%,transparent)');
    const activeIndicator = panelStyles.match(/\.side-nav-btn\.active::before\s*\{([^}]*)\}/)?.[1] ?? '';
    expect(activeIndicator).toContain('var(--accent-color,#3b82f6)');
    expect(activeIndicator).not.toContain('background-color: #3b82f6');
  });

  it('provides a compact community strategies page backed by the provider ASN and the Zemidala repository', () => {
    expect(template).toContain('id="tabContent-community"');
    expect(template).toContain('id="navBtn-community"');
    expect(template).toContain('id="communityProviderAsn"');
    expect(template).toContain('id="communityStrategiesList"');
    expect(template).toContain('zemidala/nfqws2-strategies');
    expect(appSource).toContain('strategyIncludeCommunity');
    expect(panelSource).toContain('nfqwsCommunityStrategies.list(detect, force, autoDetect)');
    expect(panelSource).toContain('communityProviderLabel(data.asn, data.provider)');
    expect(panelSource).toContain('visibleCommunityProtocols(data.items)');
    expect(template).toContain('Что это значит и как это устроено');
    expect(template).toContain('data-action-click="editCommunityProvider"');
    expect(template).toContain('id="communityShareTargetAsn"');
    expect(backendSource).toContain('query.auto_detect');
    expect(panelSource).toContain("dataset.actionClick = 'testCommunityStrategy'");
    expect(panelSource).toContain("community-strategy-badge-skipped");
    expect(panelSource).toContain("community-strategy-badge-stale");
    expect(panelSource).toContain('allTargets.length > targets.length');
    expect(panelSource).toContain("await writeTextToClipboard(item.steps.join('\\n'))");
    expect(backendSource).toContain('/api/community-strategies');
    expect(backendSource).toContain('/api/community-strategies/target-asn');
    expect(panelSource).toContain('targetAsn: share.targetAsn');
    expect(backendSource).toContain('include_community');
    expect(panelStyles).toContain('.community-strategy-row');
    expect(panelStyles).toContain('.community-strategy-badge-skipped');
    expect(panelStyles).toContain('.community-strategy-badge-stale');
    expect(panelStyles).toContain('[data-theme="light"] #tabContent-community');
    expect(panelStyles).toContain('color:var(--community-text)');
    expect(panelStyles).toContain('[data-theme="light"] #tabContent-community .community-strategy-badge-stale');
  });

  it('warns when strategy or foreign-config probes resolve through Podkop FakeIP', () => {
    expect(panelSource).toContain('appendPodkopWarning(container, operation.podkopHosts)');
    expect(panelSource).toContain('appendPodkopWarning(results, operation.podkopHosts)');
    expect(panelStyles).toContain('.podkop-route-warning');
    expect(panelStyles).toContain('[data-theme="light"] .podkop-route-warning');
  });

  it('lets users expand added file and text configs in a scrollable preview', () => {
    expect(panelSource).toContain('if (item.content !== undefined)');
    expect(panelSource).toContain("'Раскрыть' : 'Свернуть'");
    expect(panelSource).toContain('setAttribute(\'aria-expanded\', String(!preview.hidden))');
    expect(panelSource).toContain('foreign-config-preview');
    expect(panelStyles).toContain('.foreign-config-preview { max-height:10rem;');
    expect(panelStyles).toContain('overflow:auto;');
  });

  it('polls the router-side trace operation and exposes graceful stop', () => {
    expect(template).toContain('id="traceStopButton"');
    expect(template).toContain('id="traceOperationStatus"');
    expect(panelSource).toContain('nfqwsDiagnostics.traceStart');
    expect(panelSource).toContain('nfqwsDiagnostics.traceStatus');
    expect(panelSource).toContain('nfqwsDiagnostics.traceStop');
    expect(panelSource).toContain('stopSiteTrace:');
  });

  it('limits QUIC strategy selection to the single-site bounded runner', () => {
    expect(template).toContain('<option value="quic">QUIC · UDP 443</option>');
    expect(panelSource).toContain("if (multi) {");
    expect(panelSource).toContain('if (protocol instanceof HTMLSelectElement) updateStrategyProtocol(protocol.value)');
    expect(panelSource).toContain('if (option) option.disabled = quic');
    expect(panelSource).toContain('QUIC-подбор требует доменное имя');
  });

  it('omits the profile reference panel to keep the strategy editor compact', () => {
    expect(panelSource).not.toContain("referenceSummary.textContent = 'Справочник параметров профиля'");
    expect(panelSource).not.toContain("reference.className = 'profile-reference'");
    expect(panelSource).not.toContain('runtimeProfileReferenceExpanded');
    expect(panelStyles).not.toContain('.profile-reference {');
  });

  it('allows dragging a strategy tile while keeping its controls interactive', () => {
    expect(panelSource).toContain("row.draggable = true");
    expect(panelSource).toContain("row.addEventListener('pointerdown'");
    expect(panelSource).toContain('profileStepDragAllowed = !(event.target instanceof Element && event.target.closest(');
    expect(panelSource).toContain('if (!profileStepDragAllowed || (target instanceof HTMLElement');
    expect(panelSource).toContain("target.closest('input,select,button,textarea,summary,a,[contenteditable=\"true\"]')");
    expect(panelSource).toContain("row.classList.add('is-dragging')");
    expect(panelStyles).toContain('.profile-step-row.is-dragging');
    expect(panelStyles).toContain('transition:transform .18s ease,opacity .18s ease');
  });

  it('uses the shared styled popup for Lua functions and replaces the function on selection', () => {
    expect(panelSource).not.toContain("functionSelect.dataset.nativeSelect = 'true'");
    expect(panelSource).toContain("functionSelect.addEventListener('change'");
    expect(panelSource).toContain('replaceDesyncFunction(current, functionSelect.value)');
    expect(panelSource).toContain('installStyledSelects(editor)');
    expect(panelSource).toContain("functionSelect.dataset.menuMinWidth = '360'");
    expect(panelSource).toContain("functionSelect.className = 'profile-function-select min-h-8 w-full max-w-2xl rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-200'");
    expect(styledSelectSource).toContain("trigger.dataset.profileFunctionPicker = 'true'");
    expect(panelStyles).toContain('.profile-step-desync .styled-select-trigger[data-profile-function-picker="true"]');
    expect(panelSource).not.toContain("functionSelect.dataset.actionChange = 'setRuntimeProfileStepFunction'");
  });

  it('keeps update controls locked after success until the panel reloads', () => {
    const updateFlow = panelSource.slice(panelSource.indexOf('async function installSelectedRelease()'), panelSource.indexOf('\nfunction updateStrategyMode('));
    expect(updateFlow).toContain('let reloading = false');
    expect(updateFlow).toContain('reloading = true; schedulePanelRefreshAfterUpdate');
    expect(updateFlow).toContain('if (!reloading)');
    expect(updateFlow).toContain('updatePolling = false');
    expect(panelSource).toContain('install.disabled = !selectedReleaseTag || updatePolling');
  });

  it('keeps drag feedback out of the cursor and hides the grip on narrow touch layouts', () => {
    const rowRule = panelStyles.match(/\.profile-step-row \{([^}]*)\}/)?.[1] ?? '';
    const rowActiveRule = panelStyles.match(/\.profile-step-row:active \{([^}]*)\}/)?.[1] ?? '';
    const gripRule = panelStyles.match(/\.profile-step-grip \{([^}]*)\}/)?.[1] ?? '';

    expect(rowRule).not.toMatch(/cursor:\s*grab/);
    expect(rowActiveRule).not.toMatch(/cursor:\s*grabbing/);
    expect(gripRule).not.toMatch(/cursor:\s*grab|touch-action:\s*none/);
    expect(panelStyles).toMatch(/@media \(max-width:640px\)\s*\{\s*\.profile-step-grip \{ display:none; \}\s*\.profile-step-row \{ display:flex; flex-wrap:wrap;/);
    expect(panelStyles).toContain('.profile-step-grip { display:none; }');
  });

  it('labels strategy tiles by lua-desync order, excluding payload and range rows', () => {
    expect(panelSource).toContain('const strategyNumbers = profileStrategyNumbers(profile.args)');
    expect(panelSource).toContain('const strategyNumber = strategyNumbers[argumentIndex] ?? 0');
    expect(panelSource).toContain('`Стратегия ${strategyNumber}`');
    expect(panelSource).not.toContain('badge.textContent = String(stepIndex + 1)');
  });

  it('gates live configuration warnings behind the appearance preference and navigates to their line', () => {
    expect(appSource).toContain('data-appearance-config-warnings');
    expect(appSource).toContain('id={"configWarningsPanel"}');
    expect(appSource).toContain('config-warnings-panel ui-card xl:col-span-12');
    expect(panelSource).toContain('setAppearanceConfigWarnings: (showConfigWarnings: boolean) => updateAppearance({ showConfigWarnings })');
    expect(panelSource).toContain('scheduleConfigWarnings();');
    expect(panelSource).toContain('nfqwsConfig.validate(content)');
    expect(panelSource).toContain("pluralize(warnings.length, 'предупреждение', 'предупреждения', 'предупреждений')");
    expect(panelSource).toContain('goToLine.textContent = `К строке ${warning.line}`');
  });

  it('uses the CodeMirror selection and syntax layer for grouped argument editors too', () => {
    expect(codeEditorSource).toContain('mountConfigArgumentEditor');
    expect(panelSource).toContain('mountConfigArgumentEditor(codeHost');
    expect(panelSource).toContain('field.dispatchEvent(new Event(\'change\', { bubbles: true }))');
    expect(panelSource).toContain('if (editor) editor.inert = value');
    expect(panelStyles).toContain('.config-argument-code-editor .cm-editor');
    expect(panelSource).not.toContain('config-argument-highlight');
    expect(panelSource).not.toContain('focus-within:border-blue-500 focus-within:ring-2 focus-within:ring-blue-500/20');
    expect(panelStyles).toContain('.config-argument-editor:focus-within { border-color:var(--accent-border');
    expect(codeEditorSource).toContain("backgroundColor: 'var(--cm-selection) !important'");
    expect(codeEditorSource).toContain("borderRight: 'none'");
    expect(panelStyles).toContain('.config-code-editor .cm-editor, .config-argument-code-editor .cm-editor, .strategy-custom-editor .cm-editor { --cm-selection:#2d4f8e; }');
    expect(panelStyles).toContain('[data-theme="light"] .config-code-editor .cm-editor, [data-theme="light"] .config-argument-code-editor .cm-editor, [data-theme="light"] .strategy-custom-editor .cm-editor { --cm-selection:#dbeafe; }');
    expect(panelStyles).toContain('.config-code-editor ::selection, .config-argument-code-editor ::selection, .strategy-custom-editor ::selection { background:transparent !important; color:inherit !important; }');
    expect(panelStyles).toContain('::selection { background:#2d4f8e; color:inherit; }');
  });

  it('uses the selected accent for custom select focus and active choices in both themes', () => {
    expect(panelStyles).toContain('[data-accent] .styled-select-trigger-open { border-color:var(--accent-border) !important;');
    expect(panelStyles).toContain("[data-accent] .styled-select-option[data-active='true']");
    expect(panelStyles).toContain("[data-accent] .styled-select-option:hover:not([aria-disabled='true']) { background:color-mix(in srgb,var(--accent-color) 15%,transparent); color:var(--accent-border); }");
    expect(panelStyles).toContain('[data-theme="light"][data-accent] .styled-select-option[data-active=\'true\']');
    expect(panelStyles).toContain('background:color-mix(in srgb,var(--accent-color) 13%,white); color:var(--accent-soft);');
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
