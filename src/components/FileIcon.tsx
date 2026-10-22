import { AppWindow, File, FileCode2, FileImage, FileText, Folder, PenTool } from 'lucide-react';
import type { Result } from '../api';

export function extension(file: Result) {
  return file.name.includes('.') ? file.name.split('.').pop()!.toLowerCase() : '';
}
export function fileType(file: Result) {
  if (file.kind === 'app') return 'Application';
  if (file.kind === 'folder') return 'Folder';
  const ext = extension(file);
  const types: Record<string, string> = { pdf: 'PDF document', fig: 'Figma document', docx: 'Word document', doc: 'Word document', xlsx: 'Excel workbook', txt: 'Text document', md: 'Markdown document', png: 'PNG image', jpg: 'JPEG image', jpeg: 'JPEG image', svg: 'SVG image', zip: 'ZIP archive' };
  return types[ext] || (ext ? `${ext.toUpperCase()} file` : 'File');
}
export function parentPath(file: Result) {
  if (file.path.startsWith('shell:AppsFolder')) return 'Installed app';
  const separator = file.path.includes('\\') ? '\\' : '/';
  return file.path.slice(0, file.path.lastIndexOf(separator)) || separator;
}
export function FileIcon({ file, large = false }: { file: Result; large?: boolean }) {
  const ext = extension(file);
  const kind = file.kind === 'folder' ? 'folder' : file.kind === 'app' ? 'app' : ext === 'pdf' ? 'pdf' : ext === 'fig' ? 'design' : ['png', 'jpg', 'jpeg', 'svg', 'webp'].includes(ext) ? 'image' : ['js', 'ts', 'tsx', 'rs', 'json', 'html'].includes(ext) ? 'code' : 'document';
  const Icon = { folder: Folder, app: AppWindow, pdf: FileText, design: PenTool, image: FileImage, code: FileCode2, document: File }[kind];
  return <span className={`file-icon file-icon--${kind}${large ? ' file-icon--large' : ''}`} aria-hidden="true">
    <Icon strokeWidth={large ? 1.15 : 1.6} />
    {large && file.kind === 'file' && ext && <span className="file-extension">{ext.slice(0, 5).toUpperCase()}</span>}
  </span>;
}
