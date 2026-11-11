/**
 * Toast store. The Svelte layer surfaces success and error toasts here;
 * the store keeps an in-app ledger and nothing else.
 *
 * There is no OS notification. The desktop app does not register
 * `tauri-plugin-notification` and grants it no capability
 * (`tests/unit/capability-surface-gate.test.ts` asserts that), and no call
 * site asks for one.
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
