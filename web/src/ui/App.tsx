import { useCallback, useEffect, useMemo, useReducer, useState } from 'react';
import { Activity, Archive, CircleHelp, FileCog, FileText, Gauge, House, Languages, LoaderCircle, LogOut, Menu, Moon, RefreshCw, Shield, SquareTerminal, Sun, X } from 'lucide-react';
import { api, ApiError, setCsrfToken, type ConfigResponse, type Locale, type SessionResponse, type StatusResponse } from '../api';
import { actionToApplyConfig, configEditorReducer, formatBytes, normalizeStatus, percent, type ServiceAction } from '../domain';
import { t, type MessageKey } from '../i18n';

type Page = 'overview' | 'config' | 'logs' | 'backups' | 'settings';
const navigation: { id: Page; key: MessageKey; icon: typeof House }[] = [
  { id: 'overview', key: 'overview', icon: House },
  { id: 'config', key: 'config', icon: FileCog },
  { id: 'logs', key: 'logs', icon: SquareTerminal },
  { id: 'backups', key: 'backups', icon: Archive },
  { id: 'settings', key: 'settings', icon: Shield },
];
const localeNames: Record<Locale, string> = { ru: 'Русский', en: 'English', zh: '简体中文', tr: 'Türkçe' };

function App() {
  const [locale, setLocale] = useState<Locale>(() => {
    const saved = localStorage.getItem('nfqws2-locale');
    return saved === 'en' || saved === 'zh' || saved === 'tr' ? saved : 'ru';
  });
  const [page, setPage] = useState<Page>('overview');
  const [menuOpen, setMenuOpen] = useState(false);
  const [theme, setTheme] = useState<'dark' | 'light'>(() => localStorage.getItem('nfqws2-theme') === 'light' ? 'light' : 'dark');
  const [authenticated, setAuthenticated] = useState<boolean | null>(null);
  const [password, setPassword] = useState('');
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [statusError, setStatusError] = useState('');
  const [busy, setBusy] = useState('');
  const [config, setConfig] = useState<ConfigResponse | null>(null);
  const [configEditor, dispatchConfigEditor] = useReducer(configEditorReducer, { draft: '', editing: false });
  const { draft, editing: editingConfig } = configEditor;
  const [configError, setConfigError] = useState('');
  const [logs, setLogs] = useState<string | null>(null);
  const [backups, setBackups] = useState<{ id: string; createdAt: number; file: string }[]>([]);
  const [notice, setNotice] = useState('');
  const dirty = config !== null && draft !== config.content;
  const tr = useCallback((key: MessageKey) => t(locale, key), [locale]);

  useEffect(() => { document.documentElement.dataset.theme = theme; localStorage.setItem('nfqws2-theme', theme); }, [theme]);

  const fetchStatus = useCallback(async () => {
    try {
      const result = await api<StatusResponse>('/api/status');
      setStatus(result);
      setStatusError('');
    } catch (error) {
      setStatus(null);
      setStatusError(error instanceof Error ? error.message : tr('unavailable'));
      if (error instanceof ApiError && error.status === 401) {
        setCsrfToken('');
        setAuthenticated(false);
      }
    }
  }, [tr]);

  useEffect(() => {
    let active = true;
    api<SessionResponse>('/api/session').then((session) => {
      if (!active) return;
      setAuthenticated(session.authenticated);
      if (session.csrfToken) setCsrfToken(session.csrfToken);
    }).catch((error) => { if (active) { setStatusError(error instanceof Error ? error.message : tr('unavailable')); setAuthenticated(false); } });
    return () => { active = false; };
  }, [tr]);

  useEffect(() => {
    if (!authenticated) return;
    void fetchStatus();
    const timer = window.setInterval(() => {
      if (document.visibilityState === 'visible') void fetchStatus();
    }, 5000);
    const visibility = () => { if (document.visibilityState === 'visible') void fetchStatus(); };
    document.addEventListener('visibilitychange', visibility);
    return () => { window.clearInterval(timer); document.removeEventListener('visibilitychange', visibility); };
  }, [authenticated, fetchStatus]);

  useEffect(() => {
    if (!dirty) return;
    const guard = (event: BeforeUnloadEvent) => { event.preventDefault(); event.returnValue = ''; };
    window.addEventListener('beforeunload', guard);
    return () => window.removeEventListener('beforeunload', guard);
  }, [dirty]);

  useEffect(() => {
    if (!authenticated) return;
    if (page === 'config') {
      api<ConfigResponse>('/api/config?file=nfqws2.conf').then((result) => { setConfig(result); dispatchConfigEditor({ type: 'loaded', content: result.content }); setConfigError(''); }).catch((error) => setConfigError(error instanceof Error ? error.message : String(error)));
    }
    if (page === 'logs') api<{ content: string }>('/api/logs?source=nfqws2&tail=200').then((result) => setLogs(result.content)).catch((error) => setLogs(error instanceof Error ? error.message : String(error)));
    if (page === 'backups') api<{ backups: typeof backups }>('/api/backups').then((result) => setBackups(result.backups)).catch((error) => setNotice(error instanceof Error ? error.message : String(error)));
  }, [authenticated, page]);

  const statusClass = useMemo(() => `state-${normalizeStatus(status?.service.state)}`, [status]);

  async function signIn(event: React.FormEvent) {
    event.preventDefault(); setBusy('login'); setStatusError('');
    try {
      const session = await api<SessionResponse>('/api/login', { method: 'POST', body: JSON.stringify({ password }) });
      if (session.csrfToken) setCsrfToken(session.csrfToken);
      setPassword(''); setAuthenticated(true);
    } catch (error) { setStatusError(error instanceof Error ? error.message : tr('signInFailed')); }
    finally { setBusy(''); }
  }

  async function signOut() {
    try { await api('/api/logout', { method: 'POST', body: '{}' }); } catch { /* session may already have expired */ }
    setCsrfToken(''); setAuthenticated(false); setStatus(null);
  }

  async function serviceAction(action: ServiceAction) {
    if (action === 'stop' && !window.confirm(tr('confirmStop'))) return;
    if (action === 'restart' && !window.confirm(tr('confirmRestart'))) return;
    setBusy(action); setNotice('');
    try {
      const result = await api<{ message?: string }>('/api/service', { method: 'POST', body: JSON.stringify({ action }) });
      setNotice(result.message || tr('actionDone'));
      await fetchStatus();
    } catch (error) { setNotice(error instanceof Error ? error.message : tr('actionFailed')); }
    finally { setBusy(''); }
  }

  async function saveConfig(): Promise<boolean> {
    if (!config) return false;
    setBusy('save'); setConfigError('');
    try {
      const result = await api<ConfigResponse>('/api/config?file=nfqws2.conf', { method: 'PUT', body: JSON.stringify({ content: draft, revision: config.revision }) });
      setConfig(result); dispatchConfigEditor({ type: 'saved', content: result.content }); setNotice(tr('configSaved'));
      return true;
    } catch (error) { setConfigError(error instanceof Error ? error.message : tr('configSaveFailed')); return false; }
    finally { setBusy(''); }
  }

  async function saveAndApplyConfig() {
    if (!dirty || !(await saveConfig())) return;
    const action = actionToApplyConfig(normalizeStatus(status?.service.state));
    if (!action) { setNotice(tr('applyNeedsKnownStatus')); return; }
    await serviceAction(action);
  }

  async function restoreBackup(id: string) {
    if (!window.confirm(tr('restoreConfirm'))) return;
    setBusy(`restore:${id}`);
    try { await api(`/api/backups/${encodeURIComponent(id)}/restore`, { method: 'POST', body: '{}' }); setNotice(tr('backupRestored')); await fetchStatus(); }
    catch (error) { setNotice(error instanceof Error ? error.message : tr('restoreFailed')); }
    finally { setBusy(''); }
  }

  function changeLocale(value: string) {
    if (value !== 'ru' && value !== 'en' && value !== 'zh' && value !== 'tr') return;
    setLocale(value); localStorage.setItem('nfqws2-locale', value);
  }

  function openPage(nextPage: Page) {
    if (dirty && !window.confirm(tr('unsavedConfirm'))) return;
    if (dirty) {
      setConfig(null);
      dispatchConfigEditor({ type: 'reset' });
    } else {
      dispatchConfigEditor({ type: 'cancel', content: config?.content ?? draft });
    }
    setPage(nextPage);
    setMenuOpen(false);
  }

  if (authenticated === null) return <div className="loading-screen"><LoaderCircle className="spin" /> {tr('unavailable')}</div>;
  if (!authenticated) return <main className="login-shell"><div className="login-card"><Brand label={tr('brand')} /><p className="eyebrow">NFQWS2 CONTROL</p><h1>{tr('signIn')}</h1><form onSubmit={signIn}><label htmlFor="password">{tr('password')}</label><input id="password" type="password" autoComplete="current-password" value={password} onChange={(event) => setPassword(event.target.value)} required minLength={12} /><button className="button primary" disabled={busy === 'login'}>{busy ? <LoaderCircle className="spin" size={16} /> : null}{tr('signIn')}</button></form>{statusError && <p className="error-text">{statusError}</p>}<div className="login-note">{tr('loginNote')}</div></div></main>;

  const renderContent = () => {
    if (page === 'overview') return <>
      <section className={`status-strip ${statusClass}`}>
        <div className="status-copy"><span className="status-dot" /><div><strong>{tr('status')}: {tr(normalizeStatus(status?.service.state))}</strong><small>{statusError || status?.service.detail || tr('noData')}</small><small>{status?.service.checkedAt ? `${tr('recentCheck')}: ${new Date(status.service.checkedAt * 1000).toLocaleTimeString()}` : tr('noData')}</small></div></div>
        <Metric label={tr('ram')} value={status ? `${formatBytes(status.system.memoryUsedBytes)} / ${formatBytes(status.system.memoryTotalBytes)}` : '—'} amount={status?.system.memoryTotalBytes ? (status.system.memoryUsedBytes ?? 0) / status.system.memoryTotalBytes * 100 : null} />
        <Metric label={tr('cpu')} value={percent(status?.system.cpuPercent) === null ? '—' : `${percent(status?.system.cpuPercent)}%`} amount={percent(status?.system.cpuPercent)} />
        <button className="button subdued" onClick={() => void fetchStatus()} disabled={!!busy}><RefreshCw size={15} />{tr('refresh')}</button>
        <ActionButtons state={normalizeStatus(status?.service.state)} busy={busy} onAction={serviceAction} tr={tr} />
      </section>
      <p className="hint-line"><CircleHelp size={15} />{tr('dashboardHint')}</p>
      {notice && <div className="notice" role="status">{notice}<button aria-label={tr('closeNotice')} onClick={() => setNotice('')}><X size={15} /></button></div>}
      <div className="dashboard-grid">
        <article className="card service-card"><SectionTitle icon={<Activity />} title={tr('service')} /><div className={`large-state ${statusClass}`}>{tr(normalizeStatus(status?.service.state))}</div><p>{statusError || status?.service.detail || tr('noData')}</p><ActionButtons state={normalizeStatus(status?.service.state)} busy={busy} onAction={serviceAction} tr={tr} /></article>
        <article className="card"><SectionTitle icon={<Gauge />} title={tr('panel')} /><div className="card-value">0.0.0.0:{status?.panel.port ?? '—'}</div><p>{tr('panelAddress')}</p><div className="subtle">{tr('panelVersion')}: {status?.panel.version ?? '—'}</div></article>
        <article className="card"><SectionTitle icon={<Shield />} title={tr('security')} /><div className="card-value">{tr('securityState')}</div><p>{tr('securityDescription')}</p><button className="button subdued" onClick={() => void signOut()}><LogOut size={15} />{tr('logout')}</button></article>
        <article className="card wide-card"><SectionTitle icon={<FileCog />} title={tr('config')} /><p>{tr('configDescription')}</p><button className="button subdued" onClick={() => openPage('config')}>{tr('openEditor')}</button></article>
        <article className="card"><SectionTitle icon={<FileText />} title={tr('logs')} /><p>{tr('logsDescription')}</p><button className="button subdued" onClick={() => openPage('logs')}>{tr('openLogs')}</button></article>
      </div>
    </>;
    if (page === 'config') return <section className="card editor-card"><SectionTitle icon={<FileCog />} title={tr('config')} /><p className="muted">/opt/etc/nfqws2/nfqws2.conf · {tr('configHint')} {tr('configRestartHint')}</p>{configError && <p className="error-text">{configError}</p>}<div className="config-toolbar"><span className="muted">{editingConfig ? (dirty ? tr('editorDirty') : tr('editorClean')) : tr('viewConfig')}</span>{editingConfig ? <button className="button subdued" disabled={!!busy} onClick={() => { if (dirty && !window.confirm(tr('cancelEditConfirm'))) return; dispatchConfigEditor({ type: 'cancel', content: config?.content ?? '' }); }}>{tr('cancelEdit')}</button> : <button className="button subdued" disabled={!config || !!busy} onClick={() => dispatchConfigEditor({ type: 'edit' })}>{tr('editConfig')}</button>}</div>{editingConfig ? <textarea className="code-editor" spellCheck={false} aria-label="nfqws2.conf" value={draft} onChange={(event) => dispatchConfigEditor({ type: 'change', value: event.target.value })} placeholder={tr('editorLoading')} disabled={!config || !!busy} /> : <pre className="code-preview" aria-label="nfqws2.conf">{config ? config.content : tr('editorLoading')}</pre>}{editingConfig && <div className="editor-footer"><span>{dirty ? tr('editorDirty') : tr('editorClean')}</span><div className="actions"><button className="button subdued" disabled={!dirty || !!busy} onClick={() => void saveConfig()}>{busy === 'save' ? <LoaderCircle className="spin" size={15} /> : null}{tr('save')}</button><button className="button primary" disabled={!dirty || !!busy} onClick={() => void saveAndApplyConfig()}>{busy === 'save' || busy === 'start' || busy === 'restart' ? <LoaderCircle className="spin" size={15} /> : null}{tr('saveApply')}</button></div></div>}<p className="muted">{tr('configApplyHint')}</p>{notice && <div className="notice" role="status">{notice}<button aria-label={tr('closeNotice')} onClick={() => setNotice('')}><X size={15} /></button></div>}</section>;
    if (page === 'logs') return <section className="card editor-card"><SectionTitle icon={<SquareTerminal />} title={tr('logs')} /><p className="muted">{tr('logsHint')}</p><button className="button subdued" onClick={() => api<{ content: string }>('/api/logs?source=nfqws2&tail=200').then((result) => setLogs(result.content)).catch((error) => setLogs(String(error)))}><RefreshCw size={15} />{tr('update')}</button><pre className="log-view">{logs ?? tr('loading')}</pre></section>;
    if (page === 'backups') return <section className="card"><SectionTitle icon={<Archive />} title={tr('backups')} /><p className="muted">{tr('backupHint')}</p>{backups.length ? <div className="backup-list">{backups.map((backup) => <div className="backup-row" key={backup.id}><div><strong>{backup.file}</strong><small>{new Date(backup.createdAt * 1000).toLocaleString()}</small></div><button className="button subdued" disabled={!!busy} onClick={() => void restoreBackup(backup.id)}>{tr('restore')}</button></div>)}</div> : <div className="empty-state">{tr('noBackups')}</div>}</section>;
    return <section className="card settings-card"><SectionTitle icon={<Languages />} title={tr('settings')} /><label className="setting-row"><span>{tr('language')}</span><select value={locale} onChange={(event) => changeLocale(event.target.value)}>{Object.entries(localeNames).map(([key, name]) => <option key={key} value={key}>{name}</option>)}</select></label><button className="button subdued" onClick={() => void signOut()}><LogOut size={15} />{tr('logout')}</button><p className="muted">{tr('settingsHint')}</p></section>;
  };

  return <div className="app-shell">
    <aside className={`sidebar ${menuOpen ? 'mobile-open' : ''}`}><div className="sidebar-head"><Brand label={tr('brand')} /><button className="icon-button close-menu" aria-label={tr('close')} onClick={() => setMenuOpen(false)}><X /></button></div><nav aria-label={tr('menu')}>{navigation.map(({ id, key, icon: Icon }) => <button key={id} className={`nav-item ${page === id ? 'active' : ''}`} onClick={() => openPage(id)}><Icon size={18} strokeWidth={1.8} />{tr(key)}</button>)}</nav><div className="sidebar-bottom"><span className="online-dot" />{tr('localControl')}<small>{tr('panelVersion')} {status?.panel.version ?? '—'}</small></div></aside>
    {menuOpen && <button className="scrim" aria-label={tr('close')} onClick={() => setMenuOpen(false)} />}
    <main className="main-area"><header className="topbar"><button className="icon-button mobile-menu" aria-label={tr('menu')} onClick={() => setMenuOpen(true)}><Menu /></button><div><p className="eyebrow">{tr('eyebrow')}</p><h1>{tr(navigation.find((item) => item.id === page)?.key ?? 'overview')}</h1></div><span className="header-spacer" /><button className="button theme-toggle" aria-label={tr('theme')} onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}>{theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}{theme === 'dark' ? tr('lightTheme') : tr('darkTheme')}</button><select className="locale-select" aria-label={tr('language')} value={locale} onChange={(event) => changeLocale(event.target.value)}>{Object.entries(localeNames).map(([key, name]) => <option key={key} value={key}>{name}</option>)}</select><button className="icon-button header-logout" aria-label={tr('logout')} title={tr('logout')} onClick={() => void signOut()}><LogOut size={18} /></button></header>
      {renderContent()}
      <footer className="app-footer"><span>nfqws2-ui</span><span>·</span><span>{tr('footer')}</span></footer>
    </main>
  </div>;
}

function Brand({ label }: { label: string }) { return <div className="brand"><div className="brand-mark">N</div><div><strong>nfqws2</strong><small>{label}</small></div></div>; }
function SectionTitle({ icon, title }: { icon: React.ReactNode; title: string }) { return <div className="section-title"><span>{icon}</span><h2>{title}</h2></div>; }
function Metric({ label, value, amount }: { label: string; value: string; amount: number | null | undefined }) { return <div className="metric"><div className="metric-label"><span>{label}</span><strong>{value}</strong></div><div className="meter"><i style={{ transform: `scaleX(${(percent(amount) ?? 0) / 100})` }} /></div></div>; }
function ActionButtons({ state, busy, onAction, tr }: { state: ReturnType<typeof normalizeStatus>; busy: string; onAction: (action: ServiceAction) => void; tr: (key: MessageKey) => string }) { return <div className="actions"><button className="button primary" disabled={!!busy || state === 'running'} onClick={() => onAction('start')}>{busy === 'start' ? <LoaderCircle className="spin" size={15} /> : null}{tr('start')}</button><button className="button subdued" disabled={!!busy} onClick={() => onAction('restart')}>{busy === 'restart' ? <LoaderCircle className="spin" size={15} /> : null}{tr('restart')}</button><button className="button danger" disabled={!!busy || state === 'stopped'} onClick={() => onAction('stop')}>{busy === 'stop' ? <LoaderCircle className="spin" size={15} /> : null}{tr('stop')}</button></div>; }

export default App;
