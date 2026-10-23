const configuredBase = import.meta.env.VITE_API_URL?.replace(/\/+$/, '')
const BASE = configuredBase
  ? configuredBase.endsWith('/api') ? configuredBase : `${configuredBase}/api`
  : 'http://localhost:27310/api'

export class ApiError extends Error {
  constructor(message, status, body) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.body = body
  }
}

export async function api(path, options = {}) {
  const token = localStorage.getItem('levant-token')
  let response
  try {
    response = await fetch(`${BASE}${path}`, {
      ...options,
      headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}), ...options.headers }
    })
  } catch {
    throw new ApiError('Unable to reach the server', 0, null)
  }
  const body = response.status === 204 ? null : await response.json().catch(() => null)
  if (!response.ok) {
    if (response.status === 401 && token) window.dispatchEvent(new Event('levant:unauthorized'))
    throw new ApiError(body?.message || body?.detail || 'Something went wrong', response.status, body)
  }
  return body
}

