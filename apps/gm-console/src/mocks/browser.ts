/**
 * MSW browser worker. Returns the setup promise so the caller (main.tsx)
 * can `await worker.start()` before rendering the app — guarantees the
 * mock is intercepting by the time the first request is dispatched.
 */
import { setupWorker } from 'msw/browser';
import { handlers } from './handlers';

export const worker = setupWorker(...handlers);