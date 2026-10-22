import { useEffect, useRef, useState } from 'react';
import * as api from '../api';
import { LatestSearchQueue } from '../searchQueue';

type Request = { query: string; kind: string; changed: boolean };
const sameResults = (left: api.Result[], right: api.Result[]) => left.length === right.length && left.every((file, i) => {
  const other = right[i];
  return file.id === other.id && file.name === other.name && file.path === other.path && file.kind === other.kind;
});
const sameStatus = (left: api.Status, right: api.Status) => left.indexing === right.indexing && left.indexed === right.indexed && left.skipped === right.skipped && left.error === right.error;

export function useSearch(query: string, kind: string, active = true) {
  const [results, setResults] = useState<api.Result[]>([]);
  const [selected, setSelected] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [status, setStatus] = useState<api.Status>({ indexing: false, indexed: 0, skipped: 0, error: null });
  const [visible, setVisible] = useState(!document.hidden);
  const mounted = useRef(true);
  const selectedId = useRef<number | undefined>(undefined);
  selectedId.current = results[selected]?.id;
  const lastRequest = useRef('');
  const queue = useRef<LatestSearchQueue<Request, api.Result[]> | null>(null);
  if (!queue.current) queue.current = new LatestSearchQueue(
    request => api.search(request.query, request.kind),
    (items, request) => {
      if (!mounted.current) return;
      const previous = selectedId.current;
      setResults(old => sameResults(old, items) ? old : items);
      setSelected(request.changed ? 0 : Math.max(0, items.findIndex(item => item.id === previous)));
      setError('');
    },
    error => { if (mounted.current) setError(String(error)); },
    () => { if (mounted.current) setBusy(false); },
  );

  useEffect(() => {
    mounted.current = true;
    const visibility = () => setVisible(!document.hidden);
    document.addEventListener('visibilitychange', visibility);
    return () => {
      mounted.current = false;
      queue.current!.clear();
      document.removeEventListener('visibilitychange', visibility);
    };
  }, []);
  useEffect(() => {
    if (!active || !visible) return;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      let delay = 10000;
      try {
        const value = await api.status();
        if (cancelled) return;
        setStatus(old => sameStatus(old, value) ? old : value);
        delay = value.indexing ? 2000 : 10000;
      } catch (error) { if (!cancelled) setError(String(error)); }
      if (!cancelled) timer = setTimeout(() => void poll(), delay);
    };
    void poll();
    return () => { cancelled = true; clearTimeout(timer); };
  }, [active, visible]);
  useEffect(() => {
    const request = `${kind}\0${query}`;
    const changed = request !== lastRequest.current;
    lastRequest.current = request;
    if (changed || !query.trim()) { setResults(old => old.length ? [] : old); setSelected(0); }
    if (!query.trim() || !active || !visible) {
      queue.current!.clear();
      setBusy(false);
      return;
    }
    if (changed || selectedId.current === undefined) setBusy(true);
    queue.current!.submit({ query, kind, changed });
    return () => queue.current!.clear();
  }, [query, kind, active, visible, status.indexed, status.indexing]);
  useEffect(() => {
    if (active && visible) document.getElementById(`result-${selected}`)?.scrollIntoView({ block: 'nearest' });
  }, [selected, active, visible]);
  const refresh = async () => {
    try { await api.refresh(); setStatus(old => ({ ...old, indexing: true })); }
    catch (error) { setError(String(error)); }
  };
  return { results, selected, setSelected, busy, error, setError, status, refresh };
}
