/**
 * Toast / notification store. The Svelte layer surfaces success and
 * error toasts here; the underlying `tauri-plugin-notification`
 * additionally fires OS notifications when the app is backgrounded
 * (driven by individual call sites; the store itself just keeps an
 * in-app ledger).
 */

import { writable } from 'svelte/store';

export type ToastKind = 'info' | 'success' | 'warn' | 'error';
export interface Toast {
  id: string;
  kind: ToastKind;
  message: string;
  /** Auto-dismiss in milliseconds; 0 means "sticky". */
  ttl_ms: number;
  created_at: number;
}

export const toasts = writable<Toast[]>([]);

function nextId(): string {
  return `t_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;
}

export function pushToast(kind: ToastKind, message: string, ttl_ms = 4000): string {
  const id = nextId();
  const toast: Toast = { id, kind, message, ttl_ms, created_at: Date.now() };
  toasts.update((prev) => [...prev, toast]);
  if (ttl_ms > 0) {
    setTimeout(() => dismissToast(id), ttl_ms);
  }
  return id;
}

export function dismissToast(id: string): void {
  toasts.update((prev) => prev.filter((t) => t.id !== id));
}

export function clearToasts(): void {
  toasts.set([]);
}
