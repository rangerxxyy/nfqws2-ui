import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';

const mount = document.getElementById('root');
if (!mount) throw new Error('UI root element is missing');
document.documentElement.lang = 'ru';
document.documentElement.classList.add('dark');
document.body.className = 'bg-slate-950 text-slate-100 font-sans antialiased h-screen flex flex-col overflow-hidden selection:bg-blue-500/30 selection:text-blue-200';
document.title = 'nfqws2-UI — Панель управления';
createRoot(mount).render(React.createElement(App));
