import { getClient } from './client';
import type { HealthResponse } from './types';

/** Probe the server's `/api/health` for liveness + vault status. */
export async function getHealth(): Promise<HealthResponse> {
  const resp = await getClient().get<HealthResponse>('/health');
  return resp.data;
}