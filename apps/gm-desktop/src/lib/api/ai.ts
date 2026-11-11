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
 * `[FACT]` The mid-flight case needed a real fix. The five `listen` calls
 * are sequential, so if, say, the third rejects, the `await` threw out of
 * this function before the closure was built — and the two listeners
 * already attached stayed attached for the lifetime of the process, with
 * no handle the caller could ever use to release them. The comment here
 * used to claim "a mid-flight failure still detaches what was already
 * open"; that was aspirational. It is now what the code does.
 */
export async function listenReviewEvents(
  handler: (event: ReviewEventDto) => void
): Promise<UnlistenFn> {
  const attached: UnlistenFn[] = [];
  const detachAll = (): void => {
    // Detach everything, even if one of them throws. A listener that
    // fails to release must not strand the rest, and on the failure path
    // the throw here would replace the error the caller actually needs.
    for (const unlisten of attached.splice(0)) {
      try {
        unlisten();
      } catch {
        // Best effort.
      }
    }
  };

  try {
    for (const name of REVIEW_EVENTS) {
      const unlisten = await listen<ReviewEventDto>(name, (e) => handler(e.payload));
      attached.push(unlisten);
    }
  } catch (e) {
    detachAll();
    throw e;
  }

  return detachAll;
}
