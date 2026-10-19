import { describe, it, expect } from 'vitest'
import { getOAuth2Schema, getImportTokensSchema } from '../schema'

const t = (key: string) => key

describe('OAuth2 Form Schema', () => {
  const schema = getOAuth2Schema(t)

  const validData = {
    client_id: 'my-client-id',
    client_type: 'Confidential' as const,
    auth_url: 'https://accounts.example.com/o/oauth2/auth',
    token_url: 'https://oauth2.example.com/token',
    redirect_uri: 'https://myapp.example.com/oauth2/callback',
    enabled: true,
  }

  describe('client_id field', () => {
    it('rejects empty client_id', () => {
      const result = schema.safeParse({ ...validData, client_id: '' })
      expect(result.success).toBe(false)
    })

    it('accepts valid client_id', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })
  })

  describe('client_type field', () => {
    const firstMessage = (result: ReturnType<typeof schema.safeParse>) =>
      result.success ? undefined : result.error.issues[0].message

    it('accepts Confidential', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('accepts Public', () => {
      const result = schema.safeParse({ ...validData, client_type: 'Public' })
      expect(result.success).toBe(true)
    })

    it('rejects a missing client_type', () => {
      const result = schema.safeParse({ ...validData, client_type: undefined })
      expect(result.success).toBe(false)
      expect(firstMessage(result)).toBe('oauth2.clientTypeIsRequired')
    })

    it('rejects an unknown client type', () => {
      const result = schema.safeParse({ ...validData, client_type: 'public' })
      expect(result.success).toBe(false)
    })
  })

  describe('client_secret field', () => {
    it('accepts undefined client_secret', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('accepts provided client_secret', () => {
      const result = schema.safeParse({
        ...validData,
        client_secret: 'my-secret',
      })
      expect(result.success).toBe(true)
    })
  })

  describe('auth_url field', () => {
    it('rejects empty auth_url', () => {
      const result = schema.safeParse({ ...validData, auth_url: '' })
      expect(result.success).toBe(false)
    })

    it('rejects invalid URL format for auth_url', () => {
      const result = schema.safeParse({
        ...validData,
        auth_url: 'not-a-url',
      })
      expect(result.success).toBe(false)
    })

    it('accepts valid auth_url', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })
  })

  describe('token_url field', () => {
    it('rejects empty token_url', () => {
      const result = schema.safeParse({ ...validData, token_url: '' })
      expect(result.success).toBe(false)
    })

    it('rejects invalid URL format for token_url', () => {
      const result = schema.safeParse({
        ...validData,
        token_url: 'not-a-url',
      })
      expect(result.success).toBe(false)
    })
  })

  describe('redirect_uri field', () => {
    it('rejects empty redirect_uri', () => {
      const result = schema.safeParse({ ...validData, redirect_uri: '' })
      expect(result.success).toBe(false)
    })

    it('rejects invalid URL format for redirect_uri', () => {
      const result = schema.safeParse({
        ...validData,
        redirect_uri: 'not-a-url',
      })
      expect(result.success).toBe(false)
    })
  })

  describe('scopes field', () => {
    it('accepts empty scopes array', () => {
      const result = schema.safeParse({ ...validData, scopes: [] })
      expect(result.success).toBe(true)
    })

    it('accepts valid scopes', () => {
      const result = schema.safeParse({
        ...validData,
        scopes: [{ value: 'https://mail.google.com/' }],
      })
      expect(result.success).toBe(true)
    })

    it('rejects scope with empty value', () => {
      const result = schema.safeParse({
        ...validData,
        scopes: [{ value: '' }],
      })
      expect(result.success).toBe(false)
    })
  })

  describe('extra_params field', () => {
    it('accepts empty extra_params array', () => {
      const result = schema.safeParse({ ...validData, extra_params: [] })
      expect(result.success).toBe(true)
    })

    it('accepts valid extra_params', () => {
      const result = schema.safeParse({
        ...validData,
        extra_params: [{ key: 'access_type', value: 'offline' }],
      })
      expect(result.success).toBe(true)
    })

    it('rejects param with empty key', () => {
      const result = schema.safeParse({
        ...validData,
        extra_params: [{ key: '', value: 'offline' }],
      })
      expect(result.success).toBe(false)
    })

    it('rejects param with empty value', () => {
      const result = schema.safeParse({
        ...validData,
        extra_params: [{ key: 'access_type', value: '' }],
      })
      expect(result.success).toBe(false)
    })
  })

  describe('enabled field', () => {
    it('accepts enabled: true', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('accepts enabled: false', () => {
      const result = schema.safeParse({ ...validData, enabled: false })
      expect(result.success).toBe(true)
    })
  })

  describe('description field', () => {
    it('rejects description longer than 255 characters', () => {
      const result = schema.safeParse({
        ...validData,
        description: 'a'.repeat(256),
      })
      expect(result.success).toBe(false)
    })

    it('accepts description of exactly 255 characters', () => {
      const result = schema.safeParse({
        ...validData,
        description: 'a'.repeat(255),
      })
      expect(result.success).toBe(true)
    })
  })

  describe('use_proxy field', () => {
    it('accepts undefined use_proxy', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('accepts numeric use_proxy', () => {
      const result = schema.safeParse({ ...validData, use_proxy: 1 })
      expect(result.success).toBe(true)
    })
  })
})

describe('Import Tokens Form Schema', () => {
  const schema = getImportTokensSchema(t)

  const validData = {
    account_id: 42,
    access_token: 'ya29.a0AfH6SMBexample',
    refresh_token: '1//0exampleRefreshToken',
  }

  const firstMessage = (result: ReturnType<typeof schema.safeParse>) =>
    result.success ? undefined : result.error.issues[0].message

  it('accepts a complete payload', () => {
    expect(schema.safeParse(validData).success).toBe(true)
  })

  describe('account_id field', () => {
    it('rejects a missing account_id', () => {
      const result = schema.safeParse({
        access_token: validData.access_token,
        refresh_token: validData.refresh_token,
      })
      expect(result.success).toBe(false)
      expect(firstMessage(result)).toBe('oauth2.accountIsRequired')
    })

    it('rejects an undefined account_id', () => {
      const result = schema.safeParse({ ...validData, account_id: undefined })
      expect(result.success).toBe(false)
      expect(firstMessage(result)).toBe('oauth2.accountIsRequired')
    })

    it('rejects a non-numeric account_id', () => {
      const result = schema.safeParse({ ...validData, account_id: '42' })
      expect(result.success).toBe(false)
      expect(firstMessage(result)).toBe('oauth2.accountIsRequired')
    })
  })

  describe('access_token field', () => {
    it('rejects an empty access_token', () => {
      const result = schema.safeParse({ ...validData, access_token: '' })
      expect(result.success).toBe(false)
      expect(firstMessage(result)).toBe('oauth2.accessTokenCannotBeEmpty')
    })

    it('rejects a whitespace-only access_token', () => {
      const result = schema.safeParse({ ...validData, access_token: '   ' })
      expect(result.success).toBe(false)
      expect(firstMessage(result)).toBe('oauth2.accessTokenCannotBeEmpty')
    })

    it('trims surrounding whitespace off the submitted value', () => {
      const result = schema.safeParse({
        ...validData,
        access_token: '  ya29.token  ',
      })
      expect(result.success).toBe(true)
      expect(result.success && result.data.access_token).toBe('ya29.token')
    })
  })

  describe('refresh_token field', () => {
    it('rejects an empty refresh_token', () => {
      const result = schema.safeParse({ ...validData, refresh_token: '' })
      expect(result.success).toBe(false)
      expect(firstMessage(result)).toBe('oauth2.refreshTokenCannotBeEmpty')
    })

    it('rejects a whitespace-only refresh_token', () => {
      const result = schema.safeParse({ ...validData, refresh_token: '\n\t ' })
      expect(result.success).toBe(false)
      expect(firstMessage(result)).toBe('oauth2.refreshTokenCannotBeEmpty')
    })

    it('trims surrounding whitespace off the submitted value', () => {
      const result = schema.safeParse({
        ...validData,
        refresh_token: '\n1//0token\n',
      })
      expect(result.success).toBe(true)
      expect(result.success && result.data.refresh_token).toBe('1//0token')
    })
  })
})
