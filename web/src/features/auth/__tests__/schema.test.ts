import { describe, it, expect } from 'vitest'
import { getFormSchema } from '../schema'

// Simple mock t function that returns the key
const t = (key: string, _options?: Record<string, any>) => key

describe('Login Form Schema', () => {
  const schema = getFormSchema(t)

  describe('username field', () => {
    it('rejects empty username', () => {
      const result = schema.safeParse({ username: '', password: 'abcd' })
      expect(result.success).toBe(false)
      if (!result.success) {
        const usernameErrors = result.error.issues.filter(
          (i) => i.path[0] === 'username'
        )
        expect(usernameErrors.length).toBeGreaterThan(0)
      }
    })

    it('accepts valid username with password', () => {
      const result = schema.safeParse({ username: 'admin', password: 'pass1234' })
      expect(result.success).toBe(true)
    })

    it('accepts email as username', () => {
      const result = schema.safeParse({
        username: 'user@example.com',
        password: 'mypassword',
      })
      expect(result.success).toBe(true)
    })
  })

  describe('password field', () => {
    it('rejects empty password', () => {
      const result = schema.safeParse({ username: 'admin', password: '' })
      expect(result.success).toBe(false)
      if (!result.success) {
        const passwordErrors = result.error.issues.filter(
          (i) => i.path[0] === 'password'
        )
        expect(passwordErrors.length).toBeGreaterThan(0)
      }
    })

    it('rejects password shorter than 4 characters', () => {
      const result = schema.safeParse({ username: 'admin', password: 'ab' })
      expect(result.success).toBe(false)
    })

    it('accepts password of exactly 4 characters', () => {
      const result = schema.safeParse({
        username: 'admin',
        password: 'abcd',
      })
      expect(result.success).toBe(true)
    })

    it('accepts long password', () => {
      const result = schema.safeParse({
        username: 'admin',
        password: 'a'.repeat(256),
      })
      expect(result.success).toBe(true)
    })
  })

  describe('missing fields', () => {
    it('rejects empty object', () => {
      const result = schema.safeParse({})
      expect(result.success).toBe(false)
    })

    it('rejects object with only username', () => {
      const result = schema.safeParse({ username: 'admin' })
      expect(result.success).toBe(false)
    })
  })
})
