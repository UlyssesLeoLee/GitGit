/**
 * Application entrypoint. Mounts the Svelte 5 root component under
 * `#app` after wiring the global CSS. The order is important:
 * `app.css` must be loaded before any component renders so the
 * data-theme attribute resolves to the right palette on first paint.
 */
import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';

const target = document.getElementById('app');
if (!target) {
  throw new Error('mount target #app not found — index.html is missing the element');
}

// `mount()` is the Svelte 5 idiomatic API for non-hydrated mounts.
// The empty props bag is intentional: every global concern (locale,
// theme, error boundary) is reached through stores, not props.
const app = mount(App, {
  target,
  props: {},
});

export default app;
