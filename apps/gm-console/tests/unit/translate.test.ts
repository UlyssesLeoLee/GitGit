import { describe, expect, it } from 'vitest';
import { translate } from '@/i18n/runtime';

describe('translate', () => {
  it('resolves dotted keys against the active locale', () => {
    expect(translate('zh-CN', 'app.title')).toBe('gm-console');
    expect(translate('zh-CN', 'app.actions.copy')).toBeTruthy();
  });

  it('falls back to a __key__ marker when the key is missing', () => {
    expect(translate('zh-CN', 'does.not.exist')).toBe('__does.not.exist__');
  });

  it('invokes function-valued translations with positional args', () => {
    const out = translate('zh-CN', 'home.repoCount', [5]);
    expect(out).toContain('5');
  });
});