import { basicSetup } from 'codemirror';
import { EditorView, Decoration, ViewPlugin, placeholder, type DecorationSet, type ViewUpdate } from '@codemirror/view';
import { Compartment, EditorState, type Range } from '@codemirror/state';
import { highlightConfigLine } from './config-syntax';

let configView: EditorView | null = null;
const wrapping = new Compartment();

function tokenDecorations(view: EditorView): DecorationSet {
  if (view.state.doc.length > 200_000) return Decoration.none;
  const decorations: Range<Decoration>[] = [];
  const parser = new DOMParser();
  for (let lineNo = 1; lineNo <= view.state.doc.lines; lineNo += 1) {
    const line = view.state.doc.line(lineNo);
    const doc = parser.parseFromString(`<span>${highlightConfigLine(line.text)}</span>`, 'text/html');
    let offset = line.from;
    const visit = (node: Node) => {
      if (node.nodeType === Node.TEXT_NODE) { offset += node.textContent?.length ?? 0; return; }
      const start = offset;
      for (const child of [...node.childNodes]) visit(child);
      const className = node instanceof HTMLElement ? [...node.classList].find((name) => name.startsWith('conf-token-')) : undefined;
      if (className && offset > start) decorations.push(Decoration.mark({ class: className }).range(start, offset));
    };
    for (const child of [...doc.body.firstElementChild?.childNodes ?? []]) visit(child);
  }
  return Decoration.set(decorations, true);
}

const syntaxPlugin = ViewPlugin.fromClass(class {
  decorations: DecorationSet;
  constructor(view: EditorView) { this.decorations = tokenDecorations(view); }
  update(update: ViewUpdate) { if (update.docChanged) this.decorations = tokenDecorations(update.view); }
}, { decorations: (plugin) => plugin.decorations });

interface CodeEditorOptions {
  compact?: boolean;
  ariaLabel?: string;
  placeholderText?: string;
}

function createCodeEditor(host: HTMLElement, initialValue: string, onChange: (value: string) => void, options: CodeEditorOptions = {}): EditorView {
  const compact = options.compact === true;
  const fontSize = compact ? '12px' : '14px';
  const padding = compact ? '.5rem .75rem' : '1rem 1.25rem';
  const extensions = [
    basicSetup,
    compact ? EditorView.lineWrapping : wrapping.of(EditorView.lineWrapping),
    syntaxPlugin,
    options.placeholderText ? placeholder(options.placeholderText) : [],
    EditorView.contentAttributes.of({ 'aria-label': options.ariaLabel ?? 'Редактор конфигурации nfqws2', 'aria-multiline': 'true' }),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) onChange(update.state.doc.toString());
      else if (update.selectionSet) host.dispatchEvent(new Event('configselection'));
    }),
    EditorView.theme({
      '&': { height: '100%', minHeight: compact ? '4.5rem' : '0', color: 'var(--text-primary, #e2e8f0)', backgroundColor: compact ? '#05080f' : '#060a12', fontSize },
      '.cm-scroller': { fontFamily: 'var(--font-mono, "JetBrains Mono", monospace)', lineHeight: compact ? '1.25rem' : '1.68', overflow: 'auto' },
      '.cm-content': { padding, caretColor: 'var(--accent-color, #60a5fa)' },
      '.cm-gutters': { backgroundColor: '#05080f', color: '#64748b', borderRight: '1px solid rgb(30 41 59 / 80%)' },
      '.cm-activeLineGutter': { backgroundColor: 'transparent' },
      '.cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection': { backgroundColor: 'color-mix(in srgb, var(--accent-color, #60a5fa) 32%, transparent) !important' },
      '&.cm-focused': { outline: 'none' },
      '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--accent-color, #60a5fa)' },
    }),
  ];
  return new EditorView({
    parent: host,
    state: EditorState.create({
      doc: initialValue,
      extensions,
    }),
  });
}

export function mountConfigCodeEditor(host: HTMLElement, initialValue: string, onChange: (value: string) => void): EditorView {
  configView?.destroy();
  configView = createCodeEditor(host, initialValue, onChange);
  return configView;
}

export function mountConfigArgumentEditor(host: HTMLElement, initialValue: string, onChange: (value: string) => void, label: string, placeholderText: string): EditorView {
  return createCodeEditor(host, initialValue, onChange, { compact: true, ariaLabel: label, placeholderText });
}

export function getConfigCodeEditor(): EditorView | null { return configView; }

export function setConfigCodeEditorContent(value: string): void {
  if (!configView || configView.state.doc.toString() === value) return;
  configView.dispatch({ changes: { from: 0, to: configView.state.doc.length, insert: value } });
}

export function setConfigCodeEditorWrapping(enabled: boolean): void {
  if (!configView) return;
  configView.dispatch({ effects: wrapping.reconfigure(enabled ? EditorView.lineWrapping : []) });
}
