// Real app icons come from the Rust side one at a time. Keep requests few and remember answers,
// including "no icon", so scrolling or retyping never asks the shell for the same icon twice.
import * as api from './api';

const MAX_CACHED = 400;
const MAX_PARALLEL = 3;
const cache = new Map<number, string | null>();
const inflight = new Map<number, Promise<string | null>>();
const waiting: Array<() => void> = [];
let running = 0;

export const peekAppIcon = (id: number) => cache.get(id);

function remember(id: number, value: string | null) {
  cache.set(id, value);
  if (cache.size > MAX_CACHED) cache.delete(cache.keys().next().value as number);
}

export function loadAppIcon(id: number): Promise<string | null> {
  if (cache.has(id)) return Promise.resolve(cache.get(id) ?? null);
  const pending = inflight.get(id);
  if (pending) return pending;
  const task = new Promise<string | null>(resolve => {
    const run = () => {
      running++;
      api.appIcon(id).then(value => value ?? null, () => null).then(value => {
        remember(id, value);
        inflight.delete(id);
        resolve(value);
        running--;
        waiting.shift()?.();
      });
    };
    if (running < MAX_PARALLEL) run(); else waiting.push(run);
  });
  inflight.set(id, task);
  return task;
}
