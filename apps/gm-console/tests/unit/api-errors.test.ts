import { describe, expect, it } from 'vitest';
import { ApiError, NetworkError } from '@/api/errors';

describe('ApiError', () => {
  it('carries status, message and defaults code/body to null', () => {
    const err = new ApiError(404, 'not found');
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe('ApiError');
    expect(err.status).toBe(404);
    expect(err.message).toBe('not found');
    expect(err.code).toBeNull();
    expect(err.body).toBeNull();
  });

  it('retains an explicit code and body', () => {
    const body = { error: 'boom', code: 'vault_error' };
    const err = new ApiError(502, 'boom', 'vault_error', body);
    expect(err.code).toBe('vault_error');
    expect(err.body).toBe(body);
  });

  describe('isUnauthenticated', () => {
    it('is true on status 401', () => {
      expect(new ApiError(401, 'x').isUnauthenticated).toBe(true);
    });

    it('is true on the unauthenticated code regardless of status', () => {
      expect(new ApiError(403, 'x', 'unauthenticated').isUnauthenticated).toBe(true);
    });

    it('is false for an unrelated 4xx', () => {
      expect(new ApiError(404, 'x', 'not_found').isUnauthenticated).toBe(false);
    });
  });

  describe('isBadRequest', () => {
    it('is true on status 400', () => {
      expect(new ApiError(400, 'x').isBadRequest).toBe(true);
    });

    it('is true on the bad_request code regardless of status', () => {
      expect(new ApiError(422, 'x', 'bad_request').isBadRequest).toBe(true);
    });

    it('is false for a server error', () => {
      expect(new ApiError(500, 'x', 'internal').isBadRequest).toBe(false);
    });
  });

  describe('isVaultError', () => {
    it('is true on status 502', () => {
      expect(new ApiError(502, 'x').isVaultError).toBe(true);
    });

    it('is true on the vault_error code regardless of status', () => {
      expect(new ApiError(500, 'x', 'vault_error').isVaultError).toBe(true);
    });

    it('is false for a plain 4xx', () => {
      expect(new ApiError(400, 'x').isVaultError).toBe(false);
    });
  });

  describe('isServer', () => {
    it('is true for 500 and above', () => {
      expect(new ApiError(500, 'x').isServer).toBe(true);
      expect(new ApiError(503, 'x').isServer).toBe(true);
    });

    it('is false below 500', () => {
      expect(new ApiError(404, 'x').isServer).toBe(false);
      expect(new ApiError(200, 'x').isServer).toBe(false);
    });
  });
});

describe('NetworkError', () => {
  it('retains the message and the original cause', () => {
    const cause = { code: 'ECONNREFUSED' };
    const err = new NetworkError('connection refused', cause);
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe('NetworkError');
    expect(err.message).toBe('connection refused');
    expect(err.cause).toBe(cause);
  });
});
