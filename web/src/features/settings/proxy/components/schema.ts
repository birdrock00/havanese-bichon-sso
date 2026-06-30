import { z } from 'zod'

export const proxyFormSchema = z.object({
  url: z.string().min(1, 'Proxy address cannot be empty'),
})

export type ProxyFormValues = z.infer<typeof proxyFormSchema>
