/**
 * Tauri bridge for the streaming AI review (V0 task T9).
 *
 * Two commands and one event subscription. The subscription is the
 * interesting half: the review text arrives as `ai-review://token`
 * events rather than as the command's return value, because the
 * command returns as soon as the request is accepted and the acceptance
 * criterion is that tokens visibly stream.
 *
 * Event names are mirrored from `commands/ai.rs`. If a name changes
 * there it must change here, which is why they are named constants
 * rather than string literals at the call sites.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ReviewEventDto, ReviewStartedDto } from './types';

export const REVIEW_EVENT_TOKEN = 'ai-review://token';
export const REVIEW_EVENT_MODEL = 'ai-review://model';
export const REVIEW_EVENT_DONE = 'ai-review://done';
export const REVIEW_EVENT_FAILED = 'ai-review://failed';
export const REVIEW_EVENT_CANCELLED = 'ai-review://cancelled';

const REVIEW_EVENTS = [
  REVIEW_EVENT_TOKEN,
  REVIEW_EVENT_MODEL,
  REVIEW_EVENT_DONE,
  REVIEW_EVENT_FAILED,
  REVIEW_EVENT_CANCELLED,
] as const;

export interface ReviewRequest {
  diff: string;
  provider: string | null;
  model: string | null;
  baseUrl: string | null;
}

export async function aiReviewStart(req: ReviewRequest): Promise<ReviewStartedDto> {
  return await tauriInvoke('ai_review_start', {
    diff: req.diff,
    provider: req.provider,
    model: req.model,
    baseUrl: req.baseUrl,
  });
}

export async function aiReviewCancel(): Promise<string> {
  return await tauriInvoke('ai_review_cancel');
}

/**
 * Subscribe to every review event. Returns one unlisten function that
 * detaches all five listeners, so a caller cannot leak a subscription
 * by forgetting one of them.
 *
 * Each `listen` resolves with its own unlisten; the returned closure
 * runs whichever have attached so far, so a mid-flight failure still
 * detaches what was already open.
 */
export async function listenReviewEvents(
  handler: (event: ReviewEventDto) => void
): Promise<UnlistenFn> {
  const attached: UnlistenFn[] = [];
  for (const name of REVIEW_EVENTS) {
    const unlisten = await listen<ReviewEventDto>(name, (e) => handler(e.payload));
    attached.push(unlisten);
  }
  return () => {
    for (const unlisten of attached.splice(0)) unlisten();
  };
}
