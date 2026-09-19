/**
 * Server lifecycle store. Wraps the `server_status` /
 * `start_server` / `stop_server` Tauri commands and exposes the
 * current snapshot via a Svelte store.
 *
 * When running under `vite dev` outside Tauri (the mock handlers
 * stub the commands), `running` stays `false` and the UI shows a
 * banner explaining the missing backend.
 */

import { writable, derived, get } from 'svelte/store';
import * as tauri from '$lib/api/tauri';
import type { ServerStatus } from '$lib/api/types';

export const server = writable<ServerStatus>({
  handle: 'embedded',
  bind: '',
  pid: 0,
  uptime_secs: null,
  running: false,
});

export const serverBusy = writable<boolean>(false);

export const isEmbedded = derived(server, ($s) => $s.running);

export async function refreshServerStatus(): Promise<void> {
  try {
    const next = await tauri.serverStatus();
    server.set(next);
  } catch (_err) {
    // The mock handlers intentionally fail in dev when no Rust
    // backend is wired up; the UI can continue without crashing.
  }
}

export async function startServer(bind?: string): Promise<void> {
  if (get(server).running) return;
  serverBusy.set(true);
  try {
    const next = await tauri.startServer(bind ?? null);
    server.set(next);
  } finally {
    serverBusy.set(false);
  }
}

export async function stopServer(): Promise<void> {
  const snap = get(server);
  if (!snap.running) return;
  serverBusy.set(true);
  try {
    await tauri.stopServer();
    // The Rust `stop_server` returns the *prior* snapshot (a
    // documented choice that lets callers log "what we just killed");
    // we want the *current* state in the store, so we follow up
    // with `server_status`.
    const next = await tauri.serverStatus();
    server.set(next);
  } finally {
    serverBusy.set(false);
  }
}
