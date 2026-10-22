import { Clock } from 'lucide-react';
import { useEffect } from 'react';

export type Suggestion = { id: string; title: string; subtitle: string; badge?: string; run: () => void };

// The list shown instead of results when the box is empty (recent searches) or holds a "/" command.
export function Suggestions({ items, selected, heading, empty, onSelect, onClear }: {
  items: Suggestion[]; selected: number; heading: string; empty: string;
  onSelect: (index: number) => void; onClear?: () => void;
}) {
  useEffect(() => { document.getElementById(`suggestion-${selected}`)?.scrollIntoView({ block: 'nearest' }); }, [selected]);
  return <div className="suggestion-pane">
    <div className="suggestion-head"><span>{heading}</span>{onClear && items.length > 0 && <button className="text-link" onMouseDown={event => event.preventDefault()} onClick={onClear}>Clear</button>}</div>
    {items.length ? <ul id="results" role="listbox" aria-label={heading}>
      {items.map((item, index) => <li key={item.id} role="presentation">
        <div id={`suggestion-${index}`} role="option" aria-selected={index === selected} className={`result-row suggestion-row${index === selected ? ' is-selected' : ''}`}
          onMouseMove={() => onSelect(index)} onMouseDown={event => event.preventDefault()} onClick={item.run}>
          <span className="suggestion-icon" aria-hidden="true">{item.badge ? <kbd>{item.badge}</kbd> : <Clock size={16} strokeWidth={1.6} />}</span>
          <div className="result-copy"><span className="result-name">{item.title}</span><span className="result-subtitle">{item.subtitle}</span></div>
        </div>
      </li>)}
    </ul> : <p className="suggestion-empty">{empty}</p>}
  </div>;
}
