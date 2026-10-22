import { ArrowUpRight, Check, Copy, CornerDownLeft } from 'lucide-react';
import { useEffect, useState } from 'react';
import type { Result } from '../api';
import { FileIcon, fileType, parentPath } from './FileIcon';

export function Results({ results, selected, query, opening, onSelect, onOpen, onError }: {
  results: Result[]; selected: number; query: string; opening: boolean;
  onSelect: (index: number) => void; onOpen: (file: Result) => void; onError: (message: string) => void;
}) {
  const current = results[selected];
  const [copied, setCopied] = useState(false);
  useEffect(() => { setCopied(false); }, [current?.id]);
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 1800);
    return () => clearTimeout(timer);
  }, [copied]);
  if (!current) return null;
  const copy = async () => {
    try { await navigator.clipboard.writeText(current.path); setCopied(true); }
    catch { onError('Could not copy this path. You can select it in the Location field.'); }
  };
  return <div className="results-layout">
    <div className="result-list-pane" tabIndex={0} role="region" aria-label="Navigate matching results" onKeyDown={event => {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); onSelect((selected + (event.key === 'ArrowDown' ? 1 : -1) + results.length) % results.length); }
      if (event.key === 'Enter') { event.preventDefault(); onOpen(current); }
    }}>
      <ul id="results" role="listbox" aria-label="Matching files and apps">
        {results.map((file, index) => <li key={file.id} className="result-item" role="presentation">
          {index < 2 && <div className="group-label" aria-hidden="true">{index === 0 ? 'Top hit' : 'Other matches'}</div>}
          <div id={`result-${index}`} role="option" aria-selected={index === selected} className={`result-row${index === selected ? ' is-selected' : ''}`}
            onMouseMove={() => onSelect(index)} onMouseDown={event => event.preventDefault()} onClick={() => onOpen(file)}>
            <FileIcon file={file} loadIcon={index < 12} />
            <div className="result-copy"><span className="result-name"><Highlight text={file.name} query={query} /></span><span className="result-subtitle">{fileType(file)}</span></div>
            <CornerDownLeft size={14} className="result-enter" aria-hidden="true" />
          </div>
        </li>)}
      </ul>
    </div>
    <aside className="detail-pane" aria-label="Selected item details">
      <div className="detail-content">
        <FileIcon file={current} large />
        <h2>{current.name}</h2><p className="detail-type">{fileType(current)}</p>
        <dl><div><dt>Location</dt><dd title={current.path}>{parentPath(current)}</dd></div><div><dt>Kind</dt><dd>{fileType(current)}</dd></div></dl>
      </div>
      <div className="detail-actions">
        <button className="open-button" disabled={opening} onClick={() => onOpen(current)}>{opening ? 'Opening…' : current.kind === 'folder' ? 'Open folder' : current.kind === 'app' ? 'Open application' : 'Open file'}<ArrowUpRight size={15} /></button>
        <button className="copy-button" onClick={() => void copy()} aria-label={copied ? 'Path copied' : 'Copy file path'} title={copied ? 'Path copied' : 'Copy file path'}>{copied ? <Check size={15} /> : <Copy size={15} />}</button>
      </div>
      <span className="sr-only" role="status">{copied ? 'File path copied' : ''}</span>
    </aside>
  </div>;
}
function Highlight({ text, query }: { text: string; query: string }) {
  const start = text.toLowerCase().indexOf(query.trim().toLowerCase());
  if (start < 0 || !query.trim()) return <>{text}</>;
  const end = start + query.trim().length;
  return <>{text.slice(0, start)}<mark>{text.slice(start, end)}</mark>{text.slice(end)}</>;
}
