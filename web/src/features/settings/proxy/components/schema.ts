import { z } from 'zod'

// Parse a proxy URL into components. Supports two formats:
//   Standard:       socks5://[user:pass@]host:port
//   Non-standard:   socks5://host:port:user:pass  (some proxy providers)
function parseProxyUrl(value: string): {
  scheme: string
  host: string
  port: number
  username?: string
  password?: string
} | null {
  // Strip scheme
  let stripped: string
  let scheme: string
  const lower = value.toLowerCase()
  if (lower.startsWith('socks5://')) {
    scheme = 'socks5'
    stripped = value.slice('socks5://'.length)
  } else if (lower.startsWith('http://')) {
    scheme = 'http'
    stripped = value.slice('http://'.length)
  } else {
    return null
  }

  if (!stripped) return null

  // Standard format: user:pass@host:port
  const atIdx = stripped.lastIndexOf('@')
  if (atIdx >= 0) {
    const userinfo = stripped.slice(0, atIdx)
    const hostport = stripped.slice(atIdx + 1)

    // Parse userinfo
    let username: string | undefined
    let password: string | undefined
    if (userinfo) {
      const colonIdx = userinfo.indexOf(':')
      if (colonIdx >= 0) {
        username = userinfo.slice(0, colonIdx)
        password = userinfo.slice(colonIdx + 1)
      } else {
        username = userinfo
      }
    }

    // Parse host:port
    const { host, port } = splitHostPort(hostport)
    if (!host || !port) return null

    return { scheme, host, port, username, password }
  }

  // Non-standard format: host:port[:user[:pass]]
  const parts = stripped.split(':')
  if (parts.length === 1) {
    // host only, default port to 1080
    const host = parts[0]
    if (!host) return null
    return { scheme, host, port: 1080 }
  }
  if (parts.length === 2) {
    // host:port, no auth
    const host = parts[0]
    const port = parseInt(parts[1], 10)
    if (!host || isNaN(port)) return null
    return { scheme, host, port }
  }
  if (parts.length >= 4) {
    // host:port:username:password (and possibly more colons in user/pass)
    // Last part = password, second-to-last = username, rest = host:port
    const password = parts[parts.length - 1]
    const username = parts[parts.length - 2]
    const hostport = parts.slice(0, parts.length - 2).join(':')
    const { host, port } = splitHostPort(hostport)
    if (!host || !port || !username || !password) return null
    return { scheme, host, port, username, password }
  }

  return null
}

function splitHostPort(hostport: string): { host: string; port: number | null } {
  if (!hostport) return { host: '', port: null }

  // IPv6: [::1]:1080 or [::1]
  if (hostport.startsWith('[')) {
    const close = hostport.indexOf(']')
    if (close < 0) return { host: '', port: null }
    const host = hostport.slice(1, close)
    const after = hostport.slice(close + 1)
    if (!after.startsWith(':')) {
      // No port specified, default to 1080
      return { host, port: 1080 }
    }
    const port = parseInt(after.slice(1), 10)
    return { host, port: isNaN(port) ? null : port }
  }

  const lastColon = hostport.lastIndexOf(':')
  if (lastColon < 0) {
    // No port specified, default to 1080
    return { host: hostport, port: 1080 }
  }
  const host = hostport.slice(0, lastColon)
  const port = parseInt(hostport.slice(lastColon + 1), 10)
  return { host, port: isNaN(port) ? null : port }
}

export const proxyFormSchema = z.object({
  url: z
    .string()
    .min(1, 'Proxy address cannot be empty')
    .superRefine((value, ctx) => {
      if (value.length === 0) return

      // Try our custom parser first (handles both standard and non-standard)
      const parsed = parseProxyUrl(value)

      if (!parsed) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Invalid format. Expected socks5://[user:pass@]host:port or socks5://host:port:user:pass',
          path: [],
        })
        return
      }

      if (parsed.scheme !== 'socks5' && parsed.scheme !== 'http') {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'URL must start with http:// or socks5://',
          path: [],
        })
        return
      }

      if (!/^[a-zA-Z0-9\-\.]+$/.test(parsed.host)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Hostname contains invalid characters',
          path: [],
        })
        return
      }

      if (parsed.port <= 0 || parsed.port > 65535) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Port must be between 1-65535',
          path: [],
        })
        return
      }

      if (parsed.username && !parsed.password) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Password cannot be empty when username is provided',
          path: [],
        })
        return
      }

      if (parsed.password && parsed.password.length < 8) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Password must be at least 8 characters',
          path: [],
        })
        return
      }
    }),
})

export type ProxyFormValues = z.infer<typeof proxyFormSchema>
