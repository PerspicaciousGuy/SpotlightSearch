import { invoke, isTauri } from '@tauri-apps/api/core';
import { demoFiles } from './demo';
export type Result = { id: number; name: string; path: string; kind: string };
export type Status = { indexing: boolean; indexed: number; skipped: number; error: string | null };
export const native = isTauri();
export const demo = !native && new URLSearchParams(window.location.search).get('demo') === '1';

export async function search(query: string, kind: string): Promise<Result[]> {
  if (native) return invoke('search', { query, kind });
  if (!demo || !query.trim()) return [];
  const term = query.trim().toLowerCase();
  return demoFiles.filter(file => (kind === 'all' || file.kind === kind) && file.name.toLowerCase().includes(term));
}
export const status = () => native ? invoke<Status>('index_status') : Promise.resolve({ indexing: false, indexed: demo ? demoFiles.length : 0, skipped: 0, error: null });
export const refresh = () => invoke('rebuild_index');
export const open = (id: number) => invoke('open_result', { id });
