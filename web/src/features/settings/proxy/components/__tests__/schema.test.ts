import { describe, expect, it } from 'vitest'
import { proxyFormSchema } from '../schema'

describe('Proxy Form Schema', () => {
  it('rejects empty URL', () => {
    const result = proxyFormSchema.safeParse({ url: '' })
    expect(result.success).toBe(false)
  })

  it('leaves proxy URL validation to the server', () => {
    const result = proxyFormSchema.safeParse({
      url: 'socks5://proxy.example.com:8080:customer-zone-us:secret',
    })
    expect(result.success).toBe(true)
  })
})
