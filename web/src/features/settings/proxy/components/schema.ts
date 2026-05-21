import { z } from 'zod'

export const proxyFormSchema = z.object({
  url: z
    .string()
    .min(1, 'Proxy address cannot be empty')
    .superRefine((value, ctx) => {
      if (value.length === 0) {
        return
      }

      let url: URL
      try {
        url = new URL(value)
      } catch (_e) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Invalid URL format',
          path: [],
        })
        return
      }

      if (url.protocol !== 'socks5:' && url.protocol !== 'http:') {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'URL must start with http:// or socks5://',
          path: [],
        })
      }

      if (!/^[a-zA-Z0-9\-\.]+$/.test(url.hostname)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Hostname contains invalid characters',
          path: [],
        })
      }

      const port = parseInt(url.port || '1080')
      if (port <= 0 || port > 65535) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Port must be between 1-65535',
          path: [],
        })
      }

      if (url.username && !url.password) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Password cannot be empty when username is provided',
          path: [],
        })
      } else if (url.password && url.password.length < 8) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Password must be at least 8 characters',
          path: [],
        })
      }
    }),
})

export type ProxyFormValues = z.infer<typeof proxyFormSchema>
