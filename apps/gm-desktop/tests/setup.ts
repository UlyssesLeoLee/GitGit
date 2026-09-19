/**
 * Vitest setup file. Runs in the jsdom environment before every
 * unit-test file. Installs the local mock invoke bridge so tests
 * can exercise store / component paths without the real Rust
 * backend.
 */
import { installMock } from '../src/mocks/handlers';

beforeAll(() => {
  installMock();
});
