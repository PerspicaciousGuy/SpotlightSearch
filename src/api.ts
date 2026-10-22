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
export const blurEnabled = () => native ? invoke<boolean>('blur_enabled') : Promise.resolve(false);
// Browser preview only: a plainly fake letter tile so the icon path can be seen without Windows.
const sampleIcon = (id: number) => {
  const file = demoFiles.find(item => item.id === id);
  if (!file || file.kind !== 'app') return null;
  const hue = (id * 67) % 360;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" rx="14" fill="hsl(${hue} 55% 52%)"/><text x="32" y="43" font-size="32" font-family="sans-serif" text-anchor="middle" fill="white">${file.name[0]}</text></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
};
export const appIcon = (id: number): Promise<string | null> => native ? invoke<string | null>('app_icon', { id }) : Promise.resolve(demo ? sampleIcon(id) : null);
