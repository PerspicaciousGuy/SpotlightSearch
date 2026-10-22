// Slash commands (/a, /f, /o, /r) and recent searches. Kept free of React so it can be tested alone.
export type Kind = 'all' | 'app' | 'file' | 'folder';
export type Command = { key: string; kind?: Exclude<Kind, 'all'>; label: string; hint: string };
export type Recent = { query: string; kind: string };

export const commands: Command[] = [
  { key: 'a', kind: 'app', label: 'Apps', hint: 'Search applications' },
  { key: 'f', kind: 'file', label: 'Files', hint: 'Search files' },
  { key: 'o', kind: 'folder', label: 'Folders', hint: 'Search folders' },
  { key: 'r', label: 'Recent', hint: 'Show recent searches' },
];

// "/a " (a letter and a space) turns into a category chip; the rest is the query. Windows
// filenames cannot contain "/", so a leading slash never collides with a real name search.
export function parseCommand(text: string): { kind: Exclude<Kind, 'all'>; rest: string } | null {
  const match = /^\/([afo])\s(.*)$/i.exec(text);
  if (!match) return null;
  const command = commands.find(item => item.key === match[1].toLowerCase());
  return command?.kind ? { kind: command.kind, rest: match[2].replace(/^\s+/, '') } : null;
}

export const isRecentCommand = (text: string) => /^\/r\s*$/i.test(text);

// Commands to list while the text still starts with "/" and no command is complete.
export function matchingCommands(text: string): Command[] {
  const typed = text.slice(1).trim().toLowerCase();
  const found = typed ? commands.filter(item => item.key.startsWith(typed)) : commands;
  return found.length ? found : commands;
}

const STORAGE_KEY = 'lumen.recent';
export const MAX_RECENT = 10;

// Newest first, one entry per query and category, capped at MAX_RECENT.
export function addRecent(list: Recent[], entry: Recent, max = MAX_RECENT): Recent[] {
  const query = entry.query.trim();
  if (!query) return list;
  const same = (item: Recent) => item.kind === entry.kind && item.query.toLowerCase() === query.toLowerCase();
  return [{ query, kind: entry.kind }, ...list.filter(item => !same(item))].slice(0, max);
}

const valid = (item: unknown): item is Recent => {
  const value = item as Recent;
  return !!value && typeof value.query === 'string' && !!value.query.trim() && typeof value.kind === 'string';
};
export function loadRecent(): Recent[] {
  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]');
    return Array.isArray(stored) ? stored.filter(valid).slice(0, MAX_RECENT) : [];
  } catch { return []; }
}
export function saveRecent(list: Recent[]) {
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(list)); } catch { /* storage can be unavailable */ }
}
