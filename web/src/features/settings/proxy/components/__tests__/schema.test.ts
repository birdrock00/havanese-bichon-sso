import { describe, it, expect } from 'vitest'
import { proxyFormSchema } from '../schema'

describe('Proxy Form Schema', () => {
  describe('url field - basic validation', () => {
    it('rejects empty URL', () => {
      const result = proxyFormSchema.safeParse({ url: '' })
      expect(result.success).toBe(false)
    })

    it('accepts valid socks5 URL', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://127.0.0.1:1080',
      })
      expect(result.success).toBe(true)
    })

    it('accepts valid http URL', () => {
      const result = proxyFormSchema.safeParse({
        url: 'http://proxy.example.com:8080',
      })
      expect(result.success).toBe(true)
    })
  })

  describe('url field - protocol validation', () => {
    it('rejects https protocol', () => {
      const result = proxyFormSchema.safeParse({
        url: 'https://proxy.example.com:443',
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(
          result.error.issues.some((i) =>
            i.message?.includes('http:// or socks5://')
          )
        ).toBe(true)
      }
    })

    it('rejects ftp protocol', () => {
      const result = proxyFormSchema.safeParse({
        url: 'ftp://files.example.com',
      })
      expect(result.success).toBe(false)
    })

    it('rejects URL without protocol', () => {
      const result = proxyFormSchema.safeParse({
        url: '127.0.0.1:1080',
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(
          result.error.issues.some((i) =>
            i.message?.includes('Invalid URL format')
          )
        ).toBe(true)
      }
    })
  })

  describe('url field - port validation', () => {
    it('rejects port 0', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://127.0.0.1:0',
      })
      expect(result.success).toBe(false)
    })

    it('rejects port > 65535', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://127.0.0.1:99999',
      })
      expect(result.success).toBe(false)
    })

    it('accepts port 65535', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://127.0.0.1:65535',
      })
      expect(result.success).toBe(true)
    })

    it('accepts port 1', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://127.0.0.1:1',
      })
      expect(result.success).toBe(true)
    })

    it('defaults to port 1080 when no port specified', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://127.0.0.1',
      })
      expect(result.success).toBe(true)
    })
  })

  describe('url field - hostname validation', () => {
    it('accepts IP address hostname', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://192.168.1.1:1080',
      })
      expect(result.success).toBe(true)
    })

    it('accepts domain hostname', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://proxy.internal:1080',
      })
      expect(result.success).toBe(true)
    })

    it('rejects hostname with invalid characters', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://proxy_host:1080',
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(
          result.error.issues.some((i) =>
            i.message?.includes('Hostname contains invalid characters')
          )
        ).toBe(true)
      }
    })
  })

  describe('url field - auth validation', () => {
    it('rejects username without password', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://user@127.0.0.1:1080',
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(
          result.error.issues.some((i) =>
            i.message?.includes('Password cannot be empty')
          )
        ).toBe(true)
      }
    })

    it('rejects short password when username provided', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://user:short@127.0.0.1:1080',
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(
          result.error.issues.some((i) =>
            i.message?.includes('Password must be at least 8')
          )
        ).toBe(true)
      }
    })

    it('accepts valid auth credentials', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://user:password123@127.0.0.1:1080',
      })
      expect(result.success).toBe(true)
    })

    it('accepts URL without auth (no credentials)', () => {
      const result = proxyFormSchema.safeParse({
        url: 'socks5://127.0.0.1:1080',
      })
      expect(result.success).toBe(true)
    })
  })
})
