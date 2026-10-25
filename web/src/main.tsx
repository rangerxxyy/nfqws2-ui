import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { initializeAppearance } from './appearance';
import { nfqwsAuth } from './api';

function AuthGate({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<'loading' | 'open' | 'login' | 'error'>('loading');
  const [password, setPassword] = useState('');
  const [remember, setRemember] = useState(false);
  const [rememberDays, setRememberDays] = useState(30);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const handleSessionExpired = () => {
      setPassword('');
      setRemember(false);
      setBusy(false);
      setError('Сеанс истёк. Войдите снова.');
      setState('login');
    };
    window.addEventListener('nfqws-auth-expired', handleSessionExpired);
    return () => window.removeEventListener('nfqws-auth-expired', handleSessionExpired);
  }, []);
  useEffect(() => {
    void nfqwsAuth.status().then((auth) => { setRememberDays(auth.rememberDays); setState(auth.enabled && !auth.authenticated ? 'login' : 'open'); })
      .catch(() => setState('error'));
  }, []);
  if (state === 'loading') return <main className="grid min-h-screen place-items-center bg-slate-950 text-slate-200">Загрузка панели…</main>;
  if (state === 'error') return <main className="grid min-h-screen place-items-center bg-slate-950 px-4 text-slate-100"><div className="max-w-sm rounded-2xl border border-slate-800 bg-slate-900 p-6"><h1 className="font-bold">Нет связи с панелью</h1><p className="mt-2 text-sm text-slate-400">Не удалось проверить доступ. Обновите страницу и попробуйте ещё раз.</p><button onClick={() => window.location.reload()} className="mt-4 rounded-lg bg-blue-600 px-4 py-2 font-semibold">Повторить</button></div></main>;
  if (state === 'open') return children;
  const submit = async (event: React.FormEvent) => {
    event.preventDefault(); setBusy(true); setError('');
    try { await nfqwsAuth.login(password, remember); setState('open'); }
    catch (failure) { setError(failure instanceof Error ? failure.message : 'Не удалось войти.'); }
    finally { setBusy(false); }
  };
  return <main className="grid min-h-screen place-items-center bg-slate-950 px-4 text-slate-100">
    <form onSubmit={(event) => void submit(event)} className="w-full max-w-sm space-y-5 rounded-2xl border border-slate-800 bg-slate-900 p-6 shadow-2xl">
      <div><h1 className="text-xl font-bold">Вход в nfqws2-UI</h1><p className="mt-1 text-sm text-slate-400">Введите пароль панели.</p></div>
      <label className="block text-sm text-slate-300">Пароль<input autoFocus required type="password" value={password} onChange={(event) => setPassword(event.target.value)} className="mt-2 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2.5 text-white" /></label>
      <label className="flex items-center gap-2 text-sm text-slate-300"><input type="checkbox" checked={remember} onChange={(event) => setRemember(event.target.checked)} className="accent-blue-500" /> Запомнить меня</label>
      <p className="text-xs text-slate-500">{remember ? (rememberDays === 0 ? 'Отмеченный вход не будет иметь срока истечения.' : `Отмеченный вход сохранится на ${rememberDays} ${rememberDays === 1 ? 'день' : 'дней'}.`) : 'Без галочки вход действует 24 часа.'}</p>
      {error && <p role="alert" className="text-sm text-rose-300">{error}</p>}
      <button disabled={busy} className="w-full rounded-lg bg-blue-600 px-4 py-2.5 font-semibold hover:bg-blue-500 disabled:opacity-50">{busy ? 'Входим…' : 'Войти'}</button>
    </form>
  </main>;
}

const mount = document.getElementById('root');
if (!mount) throw new Error('UI root element is missing');
mount.className = 'flex flex-col h-full min-h-0';
document.documentElement.lang = 'ru';
document.documentElement.classList.add('dark');
initializeAppearance();
document.body.className = 'bg-slate-950 text-slate-100 font-sans antialiased h-screen flex flex-col overflow-hidden';
document.title = 'nfqws2-UI';
createRoot(mount).render(React.createElement(AuthGate, null, React.createElement(App)));
