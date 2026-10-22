import { useEffect, useRef, useState } from 'react';
import { AppWindow, ArrowDown, ArrowUp, ArrowUpRight, CornerDownLeft, File, Folder, Grid2X2, RefreshCw, Search, X } from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { Results } from './components/Results';
import { Suggestions, type Suggestion } from './components/Suggestions';
import { addRecent, isRecentCommand, loadRecent, matchingCommands, parseCommand, saveRecent, type Recent } from './commands';
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
  const [blur, setBlur] = useState(false);
  const [recent, setRecent] = useState<Recent[]>(loadRecent);
  const [suggestionIndex, setSuggestionIndex] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const panel = useRef<HTMLElement>(null);
  // What the panel lists: search results, recent searches (empty box or "/r"), or "/" command hints.
  const kindRecent = recent.filter(item => kind === 'all' || item.kind === kind);
  const mode = isRecentCommand(query) ? 'recent' : query.startsWith('/') ? 'hints' : query.trim() ? 'results' : kindRecent.length ? 'recent' : 'none';
  const search = useSearch(mode === 'results' ? query : '', kind, !hidden && focused);
  const { results, selected, setSelected, busy, error, setError, status } = search;
  const expanded = mode !== 'none';
  const activeFilter = filters.find(filter => filter.id === kind)!;
  const focus = () => { input.current?.focus(); };
  const kindLabel = (id: string) => filters.find(filter => filter.id === id)?.label ?? 'All';
  const suggestions: Suggestion[] = mode === 'hints'
    ? matchingCommands(query).map(command => ({ id: `command-${command.key}`, title: command.hint, subtitle: command.label, badge: `/${command.key}`,
      run: () => { if (command.kind) { setKind(command.kind); setQuery(''); } else setQuery('/r'); focus(); } }))
    : mode === 'recent'
      ? kindRecent.map((item, index) => ({ id: `recent-${index}`, title: item.query, subtitle: kindLabel(item.kind), run: () => { setKind(item.kind); setQuery(item.query); focus(); } }))
      : [];
  const suggestionSelected = Math.min(suggestionIndex, Math.max(suggestions.length - 1, 0));
  const clearRecent = () => { setRecent([]); saveRecent([]); focus(); };
  const remember = () => { const next = addRecent(recent, { query, kind }); setRecent(next); saveRecent(next); };
  // Leaving the launcher while results are showing also counts as a search worth keeping.
  useEffect(() => { if (!focused && mode === 'results' && results.length) remember(); }, [focused]);
  useEffect(() => { setSuggestionIndex(0); }, [mode, kind, query]);
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
    // The tray menu toggles blur at runtime; the startup event can arrive before this listener exists.
    const blurChanged = listen<boolean>('blur-changed', event => setBlur(event.payload));
    void api.blurEnabled().then(setBlur).catch(() => {});
    return () => { void opened.then(unlisten => unlisten()); void warning.then(unlisten => unlisten()); void focusChanged.then(unlisten => unlisten()); void blurChanged.then(unlisten => unlisten()); };
  }, []);
  useEffect(() => { document.documentElement.dataset.blur = blur ? 'on' : 'off'; }, [blur]);
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
        // The shell's padding is the transparent margin around the panel; blur mode removes it.
        const margin = parseFloat(getComputedStyle(panel.current!.parentElement!).paddingTop) || 0;
        const height = Math.ceil(panel.current!.getBoundingClientRect().height + margin * 2);
        if (height === lastHeight) return;
        lastHeight = height;
        void getCurrentWindow().setSize(new LogicalSize(820, height)).catch(error => setError(String(error)));
      });
    });
    observer.observe(panel.current);
    return () => { observer.disconnect(); cancelAnimationFrame(frame); };
  }, [blur]);
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(''), 4000);
    return () => clearTimeout(timer);
  }, [notice]);
  const launch = async (file: api.Result) => {
    if (opening || busy) return;
    if (!api.native) { setNotice('This is a sample file. Open the Windows app to search and open your own files.'); return; }
    setOpening(true);
    // Saved before opening: a successful open hides the window right away.
    remember();
    try { await api.open(file.id); } catch (error) { setError(String(error)); } finally { setOpening(false); }
  };
  return <div className={api.native ? 'app-shell native-shell' : 'app-shell preview-shell'}>
    {!api.native && <div className="preview-toolbar"><span className="preview-brand"><Search size={15} />Lumen<span>for Windows</span></span><a href={api.demo ? '/' : '/?demo=1'}>{api.demo ? 'Exit sample preview' : 'Explore sample files'} <ArrowUpRight size={13} aria-hidden="true" /></a></div>}
    {hidden ? <button className="reopen-button" onClick={show}><Search size={18} />Open Lumen<kbd>Alt + Space</kbd></button> : <main ref={panel} className={`launcher${expanded ? ' is-expanded' : ''}`} aria-label="Lumen search" onKeyDown={event => {
      if (event.key === 'Escape') { event.preventDefault(); if (query || kind !== 'all') { setQuery(''); setKind('all'); setNotice(''); focus(); } else hide(); }
    }}>
      <div className="drag-handle" data-tauri-drag-region aria-hidden="true" />
      <div className="search-row">
        <Search size={27} strokeWidth={1.7} className="search-icon" aria-hidden="true" />
        {kind !== 'all' && <span className="kind-chip">{activeFilter.label}<button aria-label={`Remove ${activeFilter.label} filter`} onMouseDown={event => event.preventDefault()} onClick={() => { setKind('all'); focus(); }}><X size={12} strokeWidth={2} /></button></span>}
        <input ref={input} autoFocus spellCheck={false} autoComplete="off" aria-label="Search files and apps" role="combobox" aria-autocomplete="list" aria-expanded={expanded} aria-controls={expanded ? 'results' : undefined} aria-activedescendant={mode === 'results' ? (results[selected] ? `result-${selected}` : undefined) : suggestions[suggestionSelected] ? `suggestion-${suggestionSelected}` : undefined} placeholder={activeFilter.placeholder} value={query} maxLength={256}
          onChange={event => {
            const parsed = parseCommand(event.target.value);
            if (parsed) { setKind(parsed.kind); setQuery(parsed.rest); } else setQuery(event.target.value);
            setNotice('');
          }} onKeyDown={event => {
            if (event.nativeEvent.isComposing) return;
            if (event.key === 'Backspace' && !query && kind !== 'all') { event.preventDefault(); setKind('all'); return; }
            const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0;
            if (step && mode === 'results') { event.preventDefault(); setSelected(value => results.length ? (value + step + results.length) % results.length : 0); }
            else if (step) { event.preventDefault(); setSuggestionIndex(() => suggestions.length ? (suggestionSelected + step + suggestions.length) % suggestions.length : 0); }
            if (event.key === 'Enter' || (event.key === 'Tab' && mode === 'hints')) {
              if (mode !== 'results' && suggestions[suggestionSelected]) { event.preventDefault(); suggestions[suggestionSelected].run(); }
              else if (event.key === 'Enter' && results[selected]) { event.preventDefault(); void launch(results[selected]); }
            }
          }} />
        {query ? <button className="icon-button clear-button" aria-label="Clear search" onClick={() => { setQuery(''); setNotice(''); focus(); }}><X size={16} /></button> : <button className="hide-button" onClick={hide} aria-label="Hide Lumen"><kbd>esc</kbd></button>}
      </div>
      <div className="filter-row">
        <nav aria-label="Result type">{filters.map((filter, index) => <button key={filter.id} className={`filter-button${kind === filter.id ? ' is-active' : ''}`} aria-pressed={kind === filter.id} aria-keyshortcuts={`Control+${index + 1}`} title={`${filter.label} · Ctrl + ${index + 1}`} onClick={() => { setKind(filter.id); focus(); }}><filter.icon size={14} strokeWidth={1.8} /><span>{filter.label}</span></button>)}</nav>
        <span className="filter-context">{mode === 'recent' ? 'Recent searches' : mode === 'hints' ? 'Commands' : busy ? 'Searching…' : mode === 'results' ? `${results.length === 40 ? '40+' : results.length} ${results.length === 1 ? 'result' : 'results'}` : <><span className="status-dot" />{status.indexing ? 'Indexing…' : 'On your computer'}</>}</span>
      </div>
      {(error || status.error || shortcutWarning) && <div className="message message--error" role="alert">{error || status.error || shortcutWarning}</div>}
      {(mode === 'recent' || mode === 'hints') && <section className="result-surface result-surface--list" aria-label={mode === 'recent' ? 'Recent searches' : 'Commands'}>
        <Suggestions items={suggestions} selected={suggestionSelected} heading={mode === 'recent' ? 'Recent searches' : 'Commands'} empty="No recent searches yet. Open a result and its search appears here."
          onSelect={setSuggestionIndex} onClear={mode === 'recent' ? clearRecent : undefined} />
      </section>}
      {mode === 'results' && <section className="result-surface" aria-label="Search results" aria-busy={busy}>
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
      <span className="sr-only" role="status" aria-live="polite">{busy ? 'Searching' : mode === 'results' ? `${results.length} results. ${results[selected]?.name || ''}` : mode === 'none' ? '' : `${suggestions.length} suggestions`}</span>
    </main>}
    {!api.native && <div className="preview-caption"><span>{api.demo ? 'Interactive preview · Sample files, not your system' : 'Desktop preview · System search is available in the Windows app'}</span><span><kbd>Alt</kbd><span>+</span><kbd>Space</kbd><span>to bring it into focus</span></span></div>}
  </div>;
}
