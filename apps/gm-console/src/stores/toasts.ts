import { create } from 'zustand';

export type ToastKind = 'success' | 'error' | 'info' | 'warning';

export interface Toast {
  id: string;
  kind: ToastKind;
  message: string;
  /** When set, auto-dismiss after N ms. `0` = sticky. */
  duration: number;
}

interface ToastsState {
  toasts: Toast[];
  push: (kind: ToastKind, message: string, opts?: { duration?: number }) => string;
  dismiss: (id: string) => void;
  clear: () => void;
}

function nextId(): string {
  return `t_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;
}

const DEFAULT_DURATION_MS = 4000;

/**
 * Toast store. Subscribers (the `<Toasts />` viewport) read the
 * current list; callers push messages via `push(kind, msg)`.
 */
export const useToastsStore = create<ToastsState>((set, get) => ({
  toasts: [],
  push(kind, message, opts) {
    const id = nextId();
    const duration = opts?.duration ?? DEFAULT_DURATION_MS;
    set((s) => ({ toasts: [...s.toasts, { id, kind, message, duration }] }));
    if (duration > 0 && typeof window !== 'undefined') {
      window.setTimeout(() => {
        if (get().toasts.some((t) => t.id === id)) get().dismiss(id);
      }, duration);
    }
    return id;
  },
  dismiss(id) {
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
  },
  clear() {
    set({ toasts: [] });
  },
}));

/**
 * Ergonomic helper: returns the store's `push` function bound to the
 * current state. Caller-friendly shape: `const toasts = useToasts();
 *  toasts.push({ kind: 'success', message: 'OK' });`
 */
export function useToasts() {
  const push = useToastsStore((s) => s.push);
  return {
    push: (input: { kind: ToastKind; message: string; duration?: number }) =>
      push(input.kind, input.message, input.duration != null ? { duration: input.duration } : undefined),
    dismiss: useToastsStore((s) => s.dismiss),
    clear: useToastsStore((s) => s.clear),
  };
}