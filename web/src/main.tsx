import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { initializeAppearance } from './appearance';

const mount = document.getElementById('root');
if (!mount) throw new Error('UI root element is missing');
mount.className = 'flex flex-col h-full min-h-0';
document.documentElement.lang = 'ru';
document.documentElement.classList.add('dark');
initializeAppearance();
document.body.className = 'bg-slate-950 text-slate-100 font-sans antialiased h-screen flex flex-col overflow-hidden';
document.title = 'nfqws2-UI';
createRoot(mount).render(React.createElement(App));
