import { useEffect, useRef, useState } from 'react';
import { AppWindow, ArrowDown, ArrowUp, ArrowUpRight, CornerDownLeft, File, Folder, Grid2X2, RefreshCw, Search, X } from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { Results } from './components/Results';
import { useSearch } from './hooks/useSearch';
import * as api from './api';

const filters = [
  { id: 'all', label: 'All', icon: Grid2X2, placeholder: 'Search anything' },
  { id: 'app', label: 'Apps', icon: AppWindow, placeholder: 'Search applications' },
  { id: 'file', label: 'Files', icon: File, placeholder: 'Search files' },
  { id: 'folder', label: 'Folders', icon: Folder, placeholder: 'Search folders' },
];
export default function App() {
  const [query, setQuery] = useState(api.demo ? 'design' : '');
  const [kind, setKind] = useState('all');
  const [hidden, setHidden] = useState(false);
  const [focused, setFocused] = useState(true);
  const [opening, setOpening] = useState(false);
  const [notice, setNotice] = useState('');
  const [shortcutWarning, setShortcutWarning] = useState('');
  const input = useRef<HTMLInputElement>(null);
  const panel = useRef<HTMLElement>(null);
  const search = useSearch(query, kind, !hidden && focused);
  const { results, selected, setSelected, busy, error, setError, status } = search;
  const expanded = !!query.trim();
  const activeFilter = filters.find(filter => filter.id === kind)!;
  const focus = () => { input.current?.focus(); };
  const show = () => { setFocused(true); setHidden(false); requestAnimationFrame(focus); };
  const hide = () => {
    if (api.native) { setFocused(false); void getCurrentWindow().hide().catch(error => { setFocused(true); setError(String(error)); }); }
    else setHidden(true);
  };
  useEffect(() => {
    if (!api.native) return;
    const opened = listen('launcher-open', () => { setFocused(true); focus(); input.current?.select(); });
    const focusChanged = getCurrentWindow().onFocusChanged(event => setFocused(event.payload));
    const warning = listen<string>('shortcut-error', event => setShortcutWarning(event.payload));
    return () => { void opened.then(unlisten => unlisten()); void warning.then(unlisten => unlisten()); void focusChanged.then(unlisten => unlisten()); };
  }, []);
  useEffect(() => {
    const handle = (event: KeyboardEvent) => {
      if (!api.native && event.altKey && event.code === 'Space') { event.preventDefault(); setHidden(old => !old); requestAnimationFrame(focus); }
      if (event.ctrlKey && /^[1-4]$/.test(event.key)) { event.preventDefault(); setKind(filters[Number(event.key) - 1].id); show(); }
      if (event.ctrlKey && event.key.toLowerCase() === 'l') { event.preventDefault(); focus(); input.current?.select(); }
    };
    window.addEventListener('keydown', handle);
    return () => window.removeEventListener('keydown', handle);
  }, []);
  useEffect(() => {
    if (!api.native || !panel.current) return;
    // Fit the native window to the same compact/expanded surface rendered in the browser.
    let frame = 0;
    let lastHeight = 0;
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const height = Math.ceil(panel.current!.getBoundingClientRect().height + 40);
        if (height === lastHeight) return;
        lastHeight = height;
        void getCurrentWindow().setSize(new LogicalSize(820, height)).catch(error => setError(String(error)));
      });
    });
    observer.observe(panel.current);
    return () => { observer.disconnect(); cancelAnimationFrame(frame); };
  }, []);
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(''), 4000);
    return () => clearTimeout(timer);
  }, [notice]);
  const launch = async (file: api.Result) => {
    if (opening || busy) return;
    if (!api.native) { setNotice('This is a sample file. Open the Windows app to search and open your own files.'); return; }
    setOpening(true);
    try { await api.open(file.id); } catch (error) { setError(String(error)); } finally { setOpening(false); }
  };
  return <div className={api.native ? 'app-shell native-shell' : 'app-shell preview-shell'}>
    {!api.native && <div className="preview-toolbar"><span className="preview-brand"><Search size={15} />Lumen<span>for Windows</span></span><a href={api.demo ? '/' : '/?demo=1'}>{api.demo ? 'Exit sample preview' : 'Explore sample files'} <ArrowUpRight size={13} aria-hidden="true" /></a></div>}
    {hidden ? <button className="reopen-button" onClick={show}><Search size={18} />Open Lumen<kbd>Alt + Space</kbd></button> : <main ref={panel} className={`launcher${expanded ? ' is-expanded' : ''}`} aria-label="Lumen search" onKeyDown={event => {
      if (event.key === 'Escape') { event.preventDefault(); if (query) { setQuery(''); setNotice(''); focus(); } else hide(); }
    }}>
      <div className="drag-handle" data-tauri-drag-region aria-hidden="true" />
      <div className="search-row">
        <Search size={27} strokeWidth={1.7} className="search-icon" aria-hidden="true" />
        <input ref={input} autoFocus spellCheck={false} autoComplete="off" aria-label="Search files and apps" role="combobox" aria-autocomplete="list" aria-expanded={expanded} aria-controls={expanded ? 'results' : undefined} aria-activedescendant={results[selected] ? `result-${selected}` : undefined} placeholder={activeFilter.placeholder} value={query} maxLength={256}
          onChange={event => { setQuery(event.target.value); setNotice(''); }} onKeyDown={event => {
            if (event.nativeEvent.isComposing) return;
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); setSelected(value => results.length ? (value + (event.key === 'ArrowDown' ? 1 : -1) + results.length) % results.length : 0); }
            if (event.key === 'Enter' && results[selected]) { event.preventDefault(); void launch(results[selected]); }
          }} />
        {query ? <button className="icon-button clear-button" aria-label="Clear search" onClick={() => { setQuery(''); setNotice(''); focus(); }}><X size={16} /></button> : <button className="hide-button" onClick={hide} aria-label="Hide Lumen"><kbd>esc</kbd></button>}
      </div>
      <div className="filter-row">
        <nav aria-label="Result type">{filters.map((filter, index) => <button key={filter.id} className={`filter-button${kind === filter.id ? ' is-active' : ''}`} aria-pressed={kind === filter.id} aria-keyshortcuts={`Control+${index + 1}`} title={`${filter.label} · Ctrl + ${index + 1}`} onClick={() => { setKind(filter.id); focus(); }}><filter.icon size={14} strokeWidth={1.8} /><span>{filter.label}</span></button>)}</nav>
        <span className="filter-context">{busy ? 'Searching…' : expanded ? `${results.length === 40 ? '40+' : results.length} ${results.length === 1 ? 'result' : 'results'}` : <><span className="status-dot" />{status.indexing ? 'Indexing…' : 'On your computer'}</>}</span>
      </div>
      {(error || status.error || shortcutWarning) && <div className="message message--error" role="alert">{error || status.error || shortcutWarning}</div>}
      {expanded && <section className="result-surface" aria-label="Search results" aria-busy={busy}>
        {results.length ? <Results results={results} selected={selected} query={query} opening={opening} onSelect={setSelected} onOpen={file => void launch(file)} onError={setError} /> : <div className="empty-state">
          <span className="empty-search"><Search size={27} strokeWidth={1.4} /></span>
          <h1>{busy ? 'Looking for a match…' : !api.native && !api.demo ? 'Your computer, one search away.' : 'No matches found'}</h1>
          <p>{busy ? 'Searching your file index.' : !api.native && !api.demo ? 'Run the Windows app to search your files, or explore the sample preview.' : status.indexing ? 'Your index is still growing. More results will appear as files are found.' : <>Try a different filename{kind !== 'all' ? ' or choose All to broaden your search' : ''}.</>}</p>
          {!busy && (kind !== 'all' ? <button className="text-button" onClick={() => { setKind('all'); focus(); }}>Search all categories</button> : !api.native && !api.demo ? <a className="text-button" href="/?demo=1">Explore sample files <ArrowUpRight size={13} aria-hidden="true" /></a> : <span className="search-tip">Search matches filenames. Try at least 3 letters for matches anywhere in a name.</span>)}
          <ul id="results" role="listbox" aria-label="Matching files and apps" className="empty-list" />
        </div>}
      </section>}
      {notice && <div className="message message--notice" role="status">{notice}</div>}
      {expanded && <footer>
        <div className="index-status"><span className={`status-dot${status.indexing ? ' is-indexing' : ''}`} /><span title={status.skipped ? `${status.skipped.toLocaleString()} paths could not be read` : undefined}>{api.demo ? 'Sample files' : !api.native ? 'Browser preview' : status.indexing ? `Indexing · ${status.indexed.toLocaleString()} items` : `${status.indexed.toLocaleString()} items indexed`}</span><button className="icon-button refresh-button" aria-label="Refresh file index" title="Refresh file index" disabled={!api.native || status.indexing} onClick={() => void search.refresh()}><RefreshCw size={12} /></button></div>
        <div className="key-hints" aria-label="Keyboard shortcuts"><span><kbd><ArrowUp size={10} /><ArrowDown size={10} /></kbd>navigate</span><span><kbd><CornerDownLeft size={11} /></kbd>open</span><span className="escape-hint"><kbd>esc</kbd>clear</span></div>
      </footer>}
      <span className="sr-only" role="status" aria-live="polite">{busy ? 'Searching' : expanded ? `${results.length} results. ${results[selected]?.name || ''}` : ''}</span>
    </main>}
    {!api.native && <div className="preview-caption"><span>{api.demo ? 'Interactive preview · Sample files, not your system' : 'Desktop preview · System search is available in the Windows app'}</span><span><kbd>Alt</kbd><span>+</span><kbd>Space</kbd><span>to bring it into focus</span></span></div>}
  </div>;
}
