import { beforeEach, describe, expect, it } from 'vitest';
import { useToastsStore, useToasts } from '@/stores/toasts';

const reset = () => useToastsStore.getState().clear();

describe('useToastsStore', () => {
  beforeEach(reset);

  it('push appends and dismisses by id', () => {
    const id = useToastsStore.getState().push('success', 'hello');
    expect(useToastsStore.getState().toasts).toHaveLength(1);
    useToastsStore.getState().dismiss(id);
    expect(useToastsStore.getState().toasts).toHaveLength(0);
  });

  it('clear empties the list', () => {
    useToastsStore.getState().push('info', 'one');
    useToastsStore.getState().push('error', 'two');
    useToastsStore.getState().clear();
    expect(useToastsStore.getState().toasts).toHaveLength(0);
  });
});

describe('useToasts hook shape', () => {
  beforeEach(reset);

  it('exposes a push helper that accepts the shorthand shape', () => {
    const t = useToasts.getState();
    t.push({ kind: 'warning', message: 'careful' });
    expect(useToastsStore.getState().toasts[0]?.message).toBe('careful');
    expect(useToastsStore.getState().toasts[0]?.kind).toBe('warning');
  });
});