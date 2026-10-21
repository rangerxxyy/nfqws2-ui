import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import App from './App';

const template = renderToStaticMarkup(React.createElement(App));
const panelSource = readFileSync(new URL('./panel.ts', import.meta.url), 'utf8');

describe('HTML-based panel shell without authentication', () => {
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

  it('keeps all styling and interactions local for the router CSP', () => {
    expect(template).not.toMatch(/\son(?:click|input|change|scroll|submit)=/i);
    expect(template).not.toContain('cdn.tailwindcss.com');
    expect(template).not.toContain('fonts.googleapis.com');
    expect(template).toMatch(/data-action(?:-(?:click|input|change|scroll|submit))=/);
    expect(panelSource).toContain("import './panel.css'");
    expect(panelSource).toContain("import '@fontsource-variable/inter'");
    expect(panelSource).toContain("import '@fontsource-variable/jetbrains-mono'");
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
