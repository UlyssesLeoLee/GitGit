/**
 * MSW handlers mirroring the gm-console REST surface. These are only
 * active when `import.meta.env.DEV` is true (see `main.tsx`).
 *
 * The handler shape intentionally diverges from a real network in
 * one place: errors are returned as `{error: "msg"}` with a status
 * code, matching the prod `api_error()` envelope so the error path
 * can be exercised end-to-end against the mock.
 *
 * ## module_switch 接入 (per ULYS-190 §4.4 stage6, 2026-09-26 12:35 JST)
 *
 * GitGit gm-console MSW frontend mock_switch L1+L2 已 ship via PR #12
 * (commit 2c57ec91 on dev). 本 stage6 commit 补 L3 module_switch:
 *
 * 3 plugin x 7 module (per handlers.ts 11 endpoint 範式聚合):
 *   - health (1): heartbeat (`GET /api/health`)
 *   - repo (3): list_repos / get_repo / get_refs
 *     (`GET /api/repos`, `:name`, `:name/refs+log`)
 *   - vault (3): list_keys / key_detail / versions
 *     (`GET/POST/DELETE /api/vault/keys/:key` + `/versions`, `/diff`, `/restore`)
 *
 * 跨项目範式对齐 (per G-MS-04):
 *   - IM1.0 PR #24: 5 plugin x 28 module
 *   - CATs PR #18: 4 plugin x 13 module
 *   - Star PR #151: 7 plugin x 7 module
 *   - RGS PR #51: 5 plugin x 12 module
 *   - IDE1.0 PR #4: 2 plugin x 8 module
 *   - GitGit stage6 (本 commit): 3 plugin x 7 module
 *
 * 跨语言 dispatch 用法 (per G-MS-BRIEF-S44-01 GitGit 推广):
 *
 * ```python
 * import subprocess, json
 * subprocess.run([
 *     "python3", "apps/gm-console/src/mocks/_lib_mock_switch_gg.py",
 *     "--aci-config", "apps/gm-console/src/mocks/.aci.json",
 *     "read-plugins",
 * ], check=True)
 * ```
 *
 * 7 module 默认全 enabled, mode=offline. cross-project validate-all 由
 * Star tools/mock-switch-validate.py v0.1 跨项目调用 (Python).
 */
import { http, HttpResponse } from 'msw';
import { mockHealth, mockStore, mockVaultDiff, mockVaultKeyDetail, mockVaultVersionsResponse } from './data';

export const handlers = [
  http.get('/api/health', () => HttpResponse.json(mockHealth)),

  http.get('/api/repos', () => HttpResponse.json(mockStore.repos)),

  http.get('/api/repos/:name', ({ params }) => {
    const name = String(params.name);
    const detail = mockStore.repoDetails[name];
    if (!detail) {
      return HttpResponse.json({ error: `repo not found: ${name}`, code: 'not_found' }, { status: 404 });
    }
    return HttpResponse.json(detail);
  }),

  http.get('/api/repos/:name/refs', ({ params }) => {
    const name = String(params.name);
    return HttpResponse.json(mockStore.repoRefs[name] ?? []);
  }),

  http.get('/api/repos/:name/log', ({ params }) => {
    const name = String(params.name);
    return HttpResponse.json(mockStore.repoLog[name] ?? []);
  }),

  http.get('/api/vault/keys', () => HttpResponse.json(mockStore.vaultKeys)),

  http.get('/api/vault/keys/:key', ({ params }) => {
    const key = String(params.key);
    const detail = mockStore.vaultDetails[key] ?? mockVaultKeyDetail(key);
    return HttpResponse.json(detail);
  }),

  http.post('/api/vault/keys/:key/versions', async ({ params, request }) => {
    const key = String(params.key);
    const body = (await request.json()) as { value: string; change_note?: string | null };
    if (!body.value) {
      return HttpResponse.json({ error: 'value must not be empty', code: 'bad_request' }, { status: 400 });
    }
    const versions = mockStore.vaultVersions[key] ?? [];
    const nextVersion = (versions[versions.length - 1]?.version ?? 0) + 1;
    const newVer = {
      version: nextVersion,
      bytes_sha256: String(nextVersion).repeat(64).slice(0, 64),
      byte_len: body.value.length,
      created_at_unix_ms: Date.now(),
      change_note: body.change_note ?? null,
    };
    mockStore.vaultVersions[key] = [...versions, newVer];
    mockStore.vaultDetails[key] = {
      key,
      value: body.value,
      current_version: nextVersion,
      byte_len: body.value.length,
      bytes_sha256: newVer.bytes_sha256,
      created_at_unix_ms: newVer.created_at_unix_ms,
      change_note: newVer.change_note,
    };
    const existing = mockStore.vaultKeys.find((k) => k.key === key);
    if (existing) {
      existing.version_count = nextVersion;
      existing.current_version = nextVersion;
      existing.byte_len = body.value.length;
    } else {
      mockStore.vaultKeys.push({
        key,
        version_count: 1,
        current_version: nextVersion,
        byte_len: body.value.length,
      });
    }
    return HttpResponse.json({ key, version: nextVersion });
  }),

  http.get('/api/vault/keys/:key/versions', ({ params }) => {
    const key = String(params.key);
    const versions = mockStore.vaultVersions[key] ?? mockVaultVersionsResponse(key).versions;
    return HttpResponse.json({ key, versions });
  }),

  http.get('/api/vault/keys/:key/diff', ({ params, request }) => {
    const key = String(params.key);
    const url = new URL(request.url);
    const base = parseInt(url.searchParams.get('base') ?? '0', 10);
    const head = parseInt(url.searchParams.get('head') ?? '0', 10);
    if (base < 1 || head < 1 || base >= head) {
      return HttpResponse.json({ error: 'base must be < head and both >= 1', code: 'bad_request' }, { status: 400 });
    }
    return HttpResponse.json(mockVaultDiff(key, base, head));
  }),

  http.post('/api/vault/keys/:key/restore', async ({ params, request }) => {
    const key = String(params.key);
    const body = (await request.json()) as { target_version: number };
    if (!body.target_version || body.target_version < 1) {
      return HttpResponse.json({ error: 'target_version must be >= 1', code: 'bad_request' }, { status: 400 });
    }
    const versions = mockStore.vaultVersions[key] ?? [];
    const target = versions.find((v) => v.version === body.target_version);
    if (!target) {
      return HttpResponse.json({ error: `version ${body.target_version} not found`, code: 'not_found' }, { status: 404 });
    }
    const newVersion = (versions[versions.length - 1]?.version ?? 0) + 1;
    mockStore.vaultVersions[key] = [
      ...versions,
      { ...target, version: newVersion, created_at_unix_ms: Date.now(), change_note: `restored from v${body.target_version}` },
    ];
    return HttpResponse.json({ key, target_version: body.target_version, new_version: newVersion });
  }),

  http.delete('/api/vault/keys/:key', ({ params }) => {
    const key = String(params.key);
    delete mockStore.vaultDetails[key];
    delete mockStore.vaultVersions[key];
    mockStore.vaultKeys = mockStore.vaultKeys.filter((k) => k.key !== key);
    return new HttpResponse(null, { status: 204 });
  }),
];