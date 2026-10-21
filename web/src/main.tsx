import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';

const mount = document.getElementById('root');
if (!mount) throw new Error('UI root element is missing');
createRoot(mount).render(React.createElement(App));
