const DEFAULT_API_BASE = import.meta.env.VITE_API_URL || '';

async function apiFetch(path, options = {}, apiBaseUrl = DEFAULT_API_BASE) {
  const response = await fetch(`${apiBaseUrl}${path}`, {
    headers: {
      'Content-Type': 'application/json',
      ...(options.headers || {})
    },
    ...options
  });

  const payload = await response.json().catch(() => ({}));

  if (!response.ok) {
    const message = payload?.error || payload?.message || `Request failed (${response.status})`;
    const error = new Error(message);
    error.status = response.status;
    error.payload = payload;
    throw error;
  }

  return payload;
}

export function listMarkets({
  apiBaseUrl = DEFAULT_API_BASE,
  status,
  creator,
  limit = 50,
  offset = 0
} = {}) {
  const params = new URLSearchParams();
  if (status) params.set('status', status);
  if (creator) params.set('creator', creator);
  if (limit) params.set('limit', String(limit));
  if (offset) params.set('offset', String(offset));

  const query = params.toString();
  return apiFetch(`/api/futarchy/markets${query ? `?${query}` : ''}`, {}, apiBaseUrl);
}

export function getMarket(id, { apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch(`/api/futarchy/markets/${id}`, {}, apiBaseUrl);
}

export function getMarketPositions(id, { apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch(`/api/futarchy/markets/${id}/positions`, {}, apiBaseUrl);
}

export function getBettorPositions(bettor, { apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch(`/api/futarchy/positions/${bettor}`, {}, apiBaseUrl);
}

export function placeBet(marketId, payload, { apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch(`/api/futarchy/markets/${marketId}/bet`, {
    method: 'POST',
    body: JSON.stringify(payload)
  }, apiBaseUrl);
}

export function validateCreateMarket(payload, { apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch('/api/futarchy/markets/validate-and-build', {
    method: 'POST',
    body: JSON.stringify(payload)
  }, apiBaseUrl);
}

export function validatePlaceBet(marketId, payload, { apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch(`/api/futarchy/markets/${marketId}/bet/validate-and-build`, {
    method: 'POST',
    body: JSON.stringify(payload)
  }, apiBaseUrl);
}

export function validateSettleMarket(marketId, payload, { apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch(`/api/futarchy/markets/${marketId}/settle/validate-and-build`, {
    method: 'POST',
    body: JSON.stringify(payload)
  }, apiBaseUrl);
}

export function validateClaimPayout(marketId, payload, { apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch(`/api/futarchy/markets/${marketId}/claim/validate-and-build`, {
    method: 'POST',
    body: JSON.stringify(payload)
  }, apiBaseUrl);
}

export function getFutarchyStats({ apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch('/api/futarchy/stats', {}, apiBaseUrl);
}

export function prepareBet(payload, { apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch('/api/futarchy/bet/prepare', {
    method: 'POST',
    body: JSON.stringify(payload)
  }, apiBaseUrl);
}

export function submitBet(payload, { apiBaseUrl = DEFAULT_API_BASE } = {}) {
  return apiFetch('/api/futarchy/bet/submit', {
    method: 'POST',
    body: JSON.stringify(payload)
  }, apiBaseUrl);
}
