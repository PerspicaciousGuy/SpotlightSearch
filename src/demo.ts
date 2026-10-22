import type { Result } from './api';

// Explicit browser-preview fixtures. The desktop build always queries the Rust index.
export const demoFiles: Result[] = [
  { id: 1, name: 'Design system.fig', path: 'C:\\Users\\Alex\\Documents\\Gleam\\Design system.fig', kind: 'file' },
  { id: 2, name: 'Design direction.pdf', path: 'C:\\Users\\Alex\\Documents\\Gleam\\Design direction.pdf', kind: 'file' },
  { id: 3, name: 'Design', path: 'C:\\Users\\Alex\\Documents\\Design', kind: 'folder' },
  { id: 4, name: 'Design review.docx', path: 'C:\\Users\\Alex\\Documents\\Gleam\\Design review.docx', kind: 'file' },
  { id: 5, name: 'Design references', path: 'C:\\Users\\Alex\\Pictures\\Design references', kind: 'folder' },
  { id: 6, name: 'Visual Studio Code.lnk', path: 'C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Visual Studio Code.lnk', kind: 'app' },
  { id: 7, name: 'Calculator.lnk', path: 'C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Calculator.lnk', kind: 'app' },
];
