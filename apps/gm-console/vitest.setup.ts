import '@testing-library/jest-dom/vitest';
import { afterEach, beforeEach } from 'vitest';
import { cleanup } from '@testing-library/react';

/**
 * `[FACT]` React refuses to run `act` — and says so on stderr — unless the
 * environment declares itself as one. Testing Library sets the flag around
 * its own helpers, which is why `render` and `userEvent` worked, but a
 * direct `act(async () => …)` around a promise the component started
 * needs it set for the whole run. Declaring it here is what lets a test
 * await a submit handler without the warning the flag would otherwise
 * replace.
 */
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/**
 * Fail a test that writes to `console` while it is running.
 *
 * `[FACT]` Vitest reports a passing run as green even when a test floods
 * stderr. That is the failure mode this guards: a green CI log full of
 * `not wrapped in act(...)` and `No routes matched location` trains
 * everyone to skim past the output, which is exactly how a real one gets
 * through. The gate makes the noise a build failure instead of wallpaper.
 *
 * `[FACT]` Both classes it currently catches are real:
 *
 *   - `An update to X inside a test was not wrapped in act(...)` — a
 *     store write in an `afterEach` that lands outside React's act
 *     environment while the component is still mounted
 *   - `No routes matched location "/404"` — `App.tsx` registers `/404`
 *     and `*` in the content `<Routes>` but **not** in the topbar
 *     `<Routes>`, so every unmatched path leaves the topbar with no
 *     match
 *
 * `[FACT]` An expected error is a real thing (this app renders failures
 * into the UI and into a live region). Those call sites already have
 * somewhere better to say so, so there is no blanket exemption and no
 * filter: a test that needs to assert on a console call has to say so
 * here, where the exemption is visible in review.
 */
const violations: string[] = [];
const original = { error: console.error, warn: console.warn };

function record(level: 'error' | 'warn', args: unknown[]): void {
  const head = typeof args[0] === 'string' ? args[0] : String(args[0] ?? '');
  violations.push(`${level}: ${head.split('\n')[0].slice(0, 160)}`);
}

console.error = (...args: unknown[]): void => {
  record('error', args);
  original.error(...args);
};
console.warn = (...args: unknown[]): void => {
  record('warn', args);
  original.warn(...args);
};

beforeEach(() => {
  violations.length = 0;
  // Re-asserted per test, not just at module load: Testing Library
  // toggles this flag around its own helpers, and a value set once at
  // import time was already false by the time a direct `act()` ran.
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
});

afterEach(() => {
  cleanup();
  if (violations.length === 0) return;
  const report = violations.map((v) => `  - ${v}`).join('\n');
  violations.length = 0;
  throw new Error(`Unexpected console output during a test:\n${report}`);
});
