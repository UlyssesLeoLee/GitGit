import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { AxiosError } from 'axios';
import {
  __resetClientForTests,
  configureClient,
  getClient,
  resolveApiBaseUrl,
  toApiError,
} from '@/api/client';
import { ApiError, NetworkError } from '@/api/errors';

/** Build a genuine axios error so `axios.isAxiosError` recognises it. */
function axiosError(opts: {
  message?: string;
  response?: { status: number; data: unknown; statusText?: string };
}) {
  return new AxiosError(
    opts.message ?? 'request failed',
    'ERR_BAD_REQUEST',
    undefined,
    undefined,
    opts.response as never,
  );
}

beforeEach(() => {
  __resetClientForTests();
});

afterEach(() => {
  __resetClientForTests();
});

describe('resolveApiBaseUrl', () => {
  it('falls back to the /api proxy path when no explicit base is set', () => {
    // VITE_API_BASE_URL is unset in the test environment.
    expect(resolveApiBaseUrl()).toBe('/api');
  });
});

describe('toApiError', () => {
  it('passes an existing ApiError through untouched', () => {
    const original = new ApiError(404, 'nope');
    expect(toApiError(original)).toBe(original);
  });

  it('passes an existing NetworkError through untouched', () => {
    const original = new NetworkError('offline', null);
    expect(toApiError(original)).toBe(original);
  });

  it('maps an axios error with no response to a NetworkError', () => {
    const err = toApiError(axiosError({ message: 'timeout of 15000ms exceeded' }));
    expect(err).toBeInstanceOf(NetworkError);
    expect((err as NetworkError).message).toBe('timeout of 15000ms exceeded');
  });

  it('falls back to a generic message when an axios error has no message', () => {
    const err = toApiError(axiosError({ message: '' }));
    expect(err).toBeInstanceOf(NetworkError);
    expect((err as NetworkError).message).toBe('network error');
  });

  it('extracts status, message and code from a structured error body', () => {
    const body = { error: 'vault exploded', code: 'vault_error' };
    const err = toApiError(
      axiosError({ response: { status: 502, data: body, statusText: 'Bad Gateway' } }),
    );
    expect(err).toBeInstanceOf(ApiError);
    const apiErr = err as ApiError;
    expect(apiErr.status).toBe(502);
    expect(apiErr.message).toBe('vault exploded');
    expect(apiErr.code).toBe('vault_error');
    expect(apiErr.body).toBe(body);
  });

  it('nulls the code when the body carries a non-string code', () => {
    const body = { error: 'bad input', code: 42 };
    const err = toApiError(axiosError({ response: { status: 400, data: body } }));
    expect((err as ApiError).code).toBeNull();
    expect((err as ApiError).message).toBe('bad input');
  });

  it('falls back to the axios message when the error field is empty', () => {
    const body = { error: '' };
    const err = toApiError(
      axiosError({ message: 'Request failed', response: { status: 400, data: body } }),
    );
    expect((err as ApiError).message).toBe('Request failed');
  });

  it('synthesises "<status> <statusText>" for an unstructured body', () => {
    const err = toApiError(
      axiosError({
        response: { status: 502, data: '<html>Bad Gateway</html>', statusText: 'Bad Gateway' },
      }),
    );
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).message).toBe('502 Bad Gateway');
    expect((err as ApiError).code).toBeNull();
    expect((err as ApiError).body).toBeNull();
  });

  it('synthesises a message when the response has no statusText', () => {
    const err = toApiError(axiosError({ response: { status: 500, data: 'oops' } }));
    expect((err as ApiError).message).toBe('500 request failed');
  });

  it('maps a plain Error to a NetworkError', () => {
    const err = toApiError(new TypeError('boom'));
    expect(err).toBeInstanceOf(NetworkError);
    expect((err as NetworkError).message).toBe('boom');
  });

  it('maps a non-Error throwable to a generic NetworkError', () => {
    const err = toApiError('just a string');
    expect(err).toBeInstanceOf(NetworkError);
    expect((err as NetworkError).message).toBe('unknown error');
    expect((err as NetworkError).cause).toBe('just a string');
  });
});

describe('getClient', () => {
  it('builds an axios instance pointed at the resolved base URL', () => {
    const client = getClient();
    expect(client.defaults.baseURL).toBe('/api');
    expect(client.defaults.timeout).toBe(15000);
    expect(client.defaults.headers['Content-Type']).toBe('application/json');
    expect(client.defaults.headers.Accept).toBe('application/json');
  });

  it('does not throw on non-2xx so every error can flow through toApiError', () => {
    const validate = getClient().defaults.validateStatus;
    expect(validate?.(404)).toBe(true);
    expect(validate?.(500)).toBe(true);
    expect(validate?.(200)).toBe(true);
  });

  it('is a singleton across calls', () => {
    expect(getClient()).toBe(getClient());
  });

  it('is rebuilt after a test reset', () => {
    const first = getClient();
    __resetClientForTests();
    expect(getClient()).not.toBe(first);
  });
});

describe('configureClient', () => {
  it('merges the patch into the shared instance defaults and returns it', () => {
    const returned = configureClient({ timeout: 42 });
    expect(getClient().defaults.timeout).toBe(42);
    expect(returned).toBe(getClient());
  });
});
