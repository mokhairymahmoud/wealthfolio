const BASE_URL = process.env.WEALTHFOLIO_API_URL || "http://localhost:8080";
const API_PREFIX = "/api/v1";

export async function apiGet<T>(
  path: string,
  params?: Record<string, string | undefined>,
): Promise<T> {
  const url = new URL(`${API_PREFIX}${path}`, BASE_URL);
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined) {
        url.searchParams.set(key, value);
      }
    }
  }
  const res = await fetch(url, {
    method: "GET",
    signal: AbortSignal.timeout(30_000),
  });
  if (!res.ok) {
    const msg = await res.text().catch(() => res.statusText);
    throw new Error(`GET ${path} failed (${res.status}): ${msg}`);
  }
  return res.json() as Promise<T>;
}

export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const url = new URL(`${API_PREFIX}${path}`, BASE_URL);
  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(30_000),
  });
  if (!res.ok) {
    const msg = await res.text().catch(() => res.statusText);
    throw new Error(`POST ${path} failed (${res.status}): ${msg}`);
  }
  return res.json() as Promise<T>;
}
