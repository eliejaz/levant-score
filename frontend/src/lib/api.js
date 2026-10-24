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
  const headers = {
    Accept: 'application/json',
    ...(options.body ? { 'Content-Type': 'application/json' } : {}),
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...options.headers
  }
  const attempts = !options.method || options.method === 'GET' ? 3 : 1
  let response
  for (let attempt = 0; attempt < attempts; attempt++) {
    try {
      response = await fetch(`${BASE}${path}`, { ...options, headers })
    } catch {
      if (attempt + 1 === attempts) throw new ApiError('Unable to reach the server', 0, null)
    }
    if (response && ![502, 503, 504].includes(response.status)) break
    if (attempt + 1 < attempts) await delay(600 * (attempt + 1))
  }
  const isJson = response.headers.get('content-type')?.includes('application/json')
  const body = response.status === 204 ? null : isJson ? await response.json().catch(() => null) : null
  if (!response.ok) {
    if (response.status === 401 && token) window.dispatchEvent(new Event('levant:unauthorized'))
    throw new ApiError(body?.message || body?.detail || 'Something went wrong', response.status, body)
  }
  if (response.status !== 204 && !isJson) throw new ApiError('The server is still waking up. Please try again.', 503, null)
  return body
}

export function warmApi() {
  const origin = BASE.replace(/\/api$/, '')
  fetch(`${origin}/actuator/health`, { mode: 'no-cors' }).catch(() => {})
}

const delay = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds))

