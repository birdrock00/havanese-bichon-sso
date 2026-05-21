import { describe, it, expect } from 'vitest'
import { getRoleFormSchema } from '../schema'

const t = (key: string) => key

describe('Role Form Schema', () => {
  const schema = getRoleFormSchema(t)

  describe('name field', () => {
    it('rejects empty name', () => {
      const result = schema.safeParse({
        name: '',
        role_type: 'Account',
        permissions: ['data:read'],
      })
      expect(result.success).toBe(false)
    })

    it('accepts valid name', () => {
      const result = schema.safeParse({
        name: 'Viewer',
        role_type: 'Account',
        permissions: ['data:read'],
      })
      expect(result.success).toBe(true)
    })
  })

  describe('role_type field', () => {
    it('accepts Global role type', () => {
      const result = schema.safeParse({
        name: 'Admin',
        role_type: 'Global',
        permissions: ['system:access'],
      })
      expect(result.success).toBe(true)
    })

    it('accepts Account role type', () => {
      const result = schema.safeParse({
        name: 'Viewer',
        role_type: 'Account',
        permissions: ['data:read'],
      })
      expect(result.success).toBe(true)
    })

    it('rejects invalid role type', () => {
      const result = schema.safeParse({
        name: 'Test',
        role_type: 'Invalid',
        permissions: ['data:read'],
      })
      expect(result.success).toBe(false)
    })
  })

  describe('permissions field', () => {
    it('rejects empty permissions array', () => {
      const result = schema.safeParse({
        name: 'Viewer',
        role_type: 'Account',
        permissions: [],
      })
      expect(result.success).toBe(false)
    })

    it('accepts single permission', () => {
      const result = schema.safeParse({
        name: 'Viewer',
        role_type: 'Account',
        permissions: ['data:read'],
      })
      expect(result.success).toBe(true)
    })

    it('accepts multiple permissions', () => {
      const result = schema.safeParse({
        name: 'Manager',
        role_type: 'Account',
        permissions: ['data:read', 'data:manage', 'account:manage'],
      })
      expect(result.success).toBe(true)
    })

    it('accepts all available permissions', () => {
      const result = schema.safeParse({
        name: 'Super Admin',
        role_type: 'Global',
        permissions: [
          'system:access',
          'system:root',
          'user:manage',
          'user:view',
          'token:manage',
          'account:create',
          'account:manage:all',
          'data:read:all',
          'data:manage:all',
          'data:raw:download:all',
          'data:delete:all',
          'data:export:batch:all',
        ],
      })
      expect(result.success).toBe(true)
    })
  })

  describe('description field', () => {
    it('accepts undefined description', () => {
      const result = schema.safeParse({
        name: 'Viewer',
        role_type: 'Account',
        permissions: ['data:read'],
      })
      expect(result.success).toBe(true)
    })

    it('accepts description string', () => {
      const result = schema.safeParse({
        name: 'Viewer',
        role_type: 'Account',
        permissions: ['data:read'],
        description: 'Read-only access to data',
      })
      expect(result.success).toBe(true)
    })
  })
})
