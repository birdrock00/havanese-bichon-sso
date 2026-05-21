import { describe, it, expect } from 'vitest'
import { profileSchema } from '../schema'

const t = (key: string) => key

describe('Profile Form Schema', () => {
  const schema = profileSchema(t)

  describe('username field', () => {
    it('rejects empty username', () => {
      const result = schema.safeParse({
        username: '',
        email: 'user@example.com',
        password: '',
      })
      expect(result.success).toBe(false)
      if (!result.success) {
        const errors = result.error.issues.filter(
          (i) => i.path[0] === 'username'
        )
        expect(errors.length).toBeGreaterThan(0)
      }
    })

    it('rejects username shorter than 3 characters', () => {
      const result = schema.safeParse({
        username: 'ab',
        email: 'user@example.com',
        password: '',
      })
      expect(result.success).toBe(false)
    })

    it('accepts username of exactly 3 characters', () => {
      const result = schema.safeParse({
        username: 'abc',
        email: 'user@example.com',
        password: '',
      })
      expect(result.success).toBe(true)
    })

    it('rejects username longer than 32 characters', () => {
      const result = schema.safeParse({
        username: 'a'.repeat(33),
        email: 'user@example.com',
        password: '',
      })
      expect(result.success).toBe(false)
    })

    it('accepts username of exactly 32 characters', () => {
      const result = schema.safeParse({
        username: 'a'.repeat(32),
        email: 'user@example.com',
        password: '',
      })
      expect(result.success).toBe(true)
    })
  })

  describe('email field', () => {
    it('rejects empty email', () => {
      const result = schema.safeParse({
        username: 'validuser',
        email: '',
        password: '',
      })
      expect(result.success).toBe(false)
    })

    it('rejects invalid email format', () => {
      const result = schema.safeParse({
        username: 'validuser',
        email: 'not-an-email',
        password: '',
      })
      expect(result.success).toBe(false)
    })

    it('rejects email without domain', () => {
      const result = schema.safeParse({
        username: 'validuser',
        email: 'user@',
        password: '',
      })
      expect(result.success).toBe(false)
    })

    it('accepts valid email', () => {
      const result = schema.safeParse({
        username: 'validuser',
        email: 'user@example.com',
        password: '',
      })
      expect(result.success).toBe(true)
    })
  })

  describe('password field', () => {
    it('accepts empty password (keep current)', () => {
      const result = schema.safeParse({
        username: 'validuser',
        email: 'user@example.com',
        password: '',
      })
      expect(result.success).toBe(true)
      if (result.success) {
        // Empty password should be transformed to undefined
        expect(result.data.password).toBeUndefined()
      }
    })

    it('rejects password shorter than 8 characters when provided', () => {
      const result = schema.safeParse({
        username: 'validuser',
        email: 'user@example.com',
        password: 'short',
      })
      expect(result.success).toBe(false)
    })

    it('accepts password of exactly 8 characters', () => {
      const result = schema.safeParse({
        username: 'validuser',
        email: 'user@example.com',
        password: '12345678',
      })
      expect(result.success).toBe(true)
    })

    it('rejects password longer than 256 characters', () => {
      const result = schema.safeParse({
        username: 'validuser',
        email: 'user@example.com',
        password: 'a'.repeat(257),
      })
      expect(result.success).toBe(false)
    })

    it('transforms non-empty password to the string value', () => {
      const result = schema.safeParse({
        username: 'validuser',
        email: 'user@example.com',
        password: 'myNewPassword123',
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.password).toBe('myNewPassword123')
      }
    })
  })
})
