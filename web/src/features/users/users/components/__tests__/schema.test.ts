import { describe, it, expect } from 'vitest'
import { getCreateUserSchema, getUpdateUserSchema } from '../schema'

const t = (key: string) => key

const validBaseUser = {
  username: 'johndoe',
  email: 'john@example.com',
  global_roles: [1],
}

const validCreateUser = {
  ...validBaseUser,
  password: 'securePassword123',
}

const invalidCases = [
  { desc: 'empty username', data: { ...validCreateUser, username: '' } },
  {
    desc: 'username shorter than 3',
    data: { ...validCreateUser, username: 'ab' },
  },
  {
    desc: 'username longer than 32',
    data: { ...validCreateUser, username: 'a'.repeat(33) },
  },
  { desc: 'empty email', data: { ...validCreateUser, email: '' } },
  {
    desc: 'invalid email format',
    data: { ...validCreateUser, email: 'not-an-email' },
  },
  {
    desc: 'empty global_roles',
    data: { ...validCreateUser, global_roles: [] },
  },
  {
    desc: 'empty password on create',
    data: { ...validBaseUser, password: '' },
  },
  {
    desc: 'short password on create',
    data: { ...validBaseUser, password: 'short' },
  },
  {
    desc: 'password longer than 256 on create',
    data: { ...validBaseUser, password: 'a'.repeat(257) },
  },
]

describe('Create User Schema', () => {
  const schema = getCreateUserSchema(t)

  it('accepts valid user data', () => {
    const result = schema.safeParse(validCreateUser)
    expect(result.success).toBe(true)
  })

  it.each(invalidCases)('rejects $desc', ({ data }) => {
    const result = schema.safeParse(data)
    expect(result.success).toBe(false)
  })

  it('accepts username of exactly 3 characters', () => {
    const result = schema.safeParse({
      ...validCreateUser,
      username: 'abc',
    })
    expect(result.success).toBe(true)
  })

  it('accepts username of exactly 32 characters', () => {
    const result = schema.safeParse({
      ...validCreateUser,
      username: 'a'.repeat(32),
    })
    expect(result.success).toBe(true)
  })

  it('accepts password of exactly 8 characters', () => {
    const result = schema.safeParse({
      ...validBaseUser,
      password: '12345678',
    })
    expect(result.success).toBe(true)
  })

  it('accepts password of exactly 256 characters', () => {
    const result = schema.safeParse({
      ...validBaseUser,
      password: 'a'.repeat(256),
    })
    expect(result.success).toBe(true)
  })
})

describe('Update User Schema', () => {
  const schema = getUpdateUserSchema(t)

  it('accepts empty password (keep current)', () => {
    const result = schema.safeParse({
      ...validBaseUser,
      password: '',
    })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.password).toBeUndefined()
    }
  })

  it('accepts undefined password', () => {
    const result = schema.safeParse(validBaseUser)
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.password).toBeUndefined()
    }
  })

  it('rejects short password when provided', () => {
    const result = schema.safeParse({
      ...validBaseUser,
      password: 'short',
    })
    expect(result.success).toBe(false)
  })

  it('accepts valid password when provided', () => {
    const result = schema.safeParse({
      ...validBaseUser,
      password: 'newPassword123',
    })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.password).toBe('newPassword123')
    }
  })
})

describe('User Schema - ACL', () => {
  const schema = getCreateUserSchema(t)

  describe('ip_whitelist validation', () => {
    it('accepts valid IPv4 addresses', () => {
      const result = schema.safeParse({
        ...validCreateUser,
        acl: { ip_whitelist: '192.168.1.1\n10.0.0.1' },
      })
      expect(result.success).toBe(true)
    })

    it('accepts valid IPv6 address', () => {
      const result = schema.safeParse({
        ...validCreateUser,
        acl: { ip_whitelist: '2001:0db8:85a3:0000:0000:8a2e:0370:7334' },
      })
      expect(result.success).toBe(true)
    })

    it('rejects invalid IP format', () => {
      const result = schema.safeParse({
        ...validCreateUser,
        acl: { ip_whitelist: 'not-an-ip' },
      })
      expect(result.success).toBe(false)
    })

    it('rejects invalid IP with too many octets', () => {
      const result = schema.safeParse({
        ...validCreateUser,
        acl: { ip_whitelist: '192.168.1.1.1' },
      })
      expect(result.success).toBe(false)
    })

    it('rejects IP with octet > 255', () => {
      const result = schema.safeParse({
        ...validCreateUser,
        acl: { ip_whitelist: '300.1.1.1' },
      })
      expect(result.success).toBe(false)
    })

    it('accepts empty ACL (no security policies)', () => {
      const result = schema.safeParse(validCreateUser)
      expect(result.success).toBe(true)
    })
  })

  describe('rate_limit validation', () => {
    it('accepts valid rate_limit', () => {
      const result = schema.safeParse({
        ...validCreateUser,
        acl: {
          rate_limit: { quota: 100, interval: 60 },
        },
      })
      expect(result.success).toBe(true)
    })

    it('transforms ACL with only rate_limit', () => {
      const result = schema.safeParse({
        ...validCreateUser,
        acl: {
          rate_limit: { quota: 100, interval: 60 },
        },
      })
      expect(result.success).toBe(true)
      if (result.success && result.data.acl) {
        expect(result.data.acl.rate_limit).toBeDefined()
        expect(result.data.acl.rate_limit!.quota).toBe(100)
        expect(result.data.acl.ip_whitelist).toBeUndefined()
      }
    })

    it('returns undefined for ACL with only empty ip_whitelist', () => {
      const result = schema.safeParse({
        ...validCreateUser,
        acl: { ip_whitelist: '' },
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.acl).toBeUndefined()
      }
    })

    it('returns undefined for ACL with no data', () => {
      const result = schema.safeParse({
        ...validCreateUser,
        acl: { ip_whitelist: '\n\n' },
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.acl).toBeUndefined()
      }
    })
  })
})

describe('User Schema - account_access_entries', () => {
  const schema = getCreateUserSchema(t)

  it('accepts empty account_access_entries (defaults to [])', () => {
    const result = schema.safeParse(validCreateUser)
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.account_access_entries).toEqual([])
    }
  })

  it('accepts valid account access entries', () => {
    const result = schema.safeParse({
      ...validCreateUser,
      account_access_entries: [
        { accountId: 1, roleId: 2 },
        { accountId: 3, roleId: 4 },
      ],
    })
    expect(result.success).toBe(true)
  })

  it('rejects entry with accountId 0', () => {
    const result = schema.safeParse({
      ...validCreateUser,
      account_access_entries: [{ accountId: 0, roleId: 1 }],
    })
    expect(result.success).toBe(false)
  })

  it('rejects entry with roleId 0', () => {
    const result = schema.safeParse({
      ...validCreateUser,
      account_access_entries: [{ accountId: 1, roleId: 0 }],
    })
    expect(result.success).toBe(false)
  })
})

describe('User Schema - description', () => {
  const schema = getCreateUserSchema(t)

  it('accepts undefined description', () => {
    const result = schema.safeParse(validCreateUser)
    expect(result.success).toBe(true)
  })

  it('accepts empty string description', () => {
    const result = schema.safeParse({
      ...validCreateUser,
      description: '',
    })
    expect(result.success).toBe(true)
  })

  it('accepts valid description', () => {
    const result = schema.safeParse({
      ...validCreateUser,
      description: 'A test user account',
    })
    expect(result.success).toBe(true)
  })

  it('rejects description longer than 256 characters', () => {
    const result = schema.safeParse({
      ...validCreateUser,
      description: 'a'.repeat(257),
    })
    expect(result.success).toBe(false)
  })
})
