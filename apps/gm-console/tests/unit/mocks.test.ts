import { describe, expect, it, beforeEach } from 'vitest';
import { mockStore } from '@/mocks/data';

describe('mockStore seed data', () => {
  beforeEach(() => {
    // Mutating tests below reset the key they touch; this is the
    // smoke check that the seed itself is internally consistent.
  });

  it('every seed repo has a matching detail entry', () => {
    for (const r of mockStore.repos) {
      expect(mockStore.repoDetails[r.name]).toBeDefined();
      expect(mockStore.repoRefs[r.name]).toBeDefined();
      expect(mockStore.repoLog[r.name]).toBeDefined();
    }
  });

  it('vault versions are ordered ascending by version', () => {
    for (const [, versions] of Object.entries(mockStore.vaultVersions)) {
      for (let i = 1; i < versions.length; i++) {
        expect(versions[i]!.version).toBeGreaterThan(versions[i - 1]!.version);
      }
    }
  });

  it('current_version on the detail matches the last entry in versions', () => {
    for (const [, detail] of Object.entries(mockStore.vaultDetails)) {
      const versions = mockStore.vaultVersions[detail.key] ?? [];
      const last = versions[versions.length - 1];
      if (last && detail.current_version != null) {
        expect(detail.current_version).toBe(last.version);
      }
    }
  });
});