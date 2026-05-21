import { z } from 'zod'

export const accountAccessEntry = (t: (key: string) => string) =>
  z.object({
    accountId: z.number().min(1, t('users.actions.schema.account_required')),
    roleId: z.number().min(1, t('users.actions.schema.role_required')),
  })

const isValidIPv4 = (ip: string): boolean => {
  const parts = ip.split('.')
  if (parts.length !== 4) return false
  return parts.every((part) => {
    const num = Number(part)
    return part === String(num) && num >= 0 && num <= 255
  })
}

const isValidIP = (ip: string) => {
  const ipv6 = /^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$/
  return isValidIPv4(ip) || ipv6.test(ip)
}

export const getBaseUserSchema = (t: (key: string) => string) =>
  z.object({
    username: z
      .string()
      .min(1, t('users.actions.schema.username_required'))
      .min(3, t('users.actions.schema.username_min'))
      .max(32, t('users.actions.schema.username_max')),
    email: z
      .string()
      .min(1, t('users.actions.schema.email_required'))
      .email(t('users.actions.schema.email_invalid')),
    global_roles: z
      .array(z.number())
      .min(1, t('users.actions.schema.global_role_required')),
    account_access_entries: z
      .array(accountAccessEntry(t))
      .optional()
      .default([]),
    description: z
      .string()
      .max(256, t('users.actions.schema.description_max'))
      .optional()
      .or(z.literal('')),
    acl: z
      .object({
        ip_whitelist: z.string().optional(),
        rate_limit: z
          .object({
            quota: z.number().positive().optional(),
            interval: z.number().positive().optional(),
          })
          .optional(),
      })
      .optional()
      .transform((data) => {
        if (!data) return undefined
        const ips =
          data.ip_whitelist
            ?.split('\n')
            .map((v) => v.trim())
            .filter(Boolean) || []
        const finalRateLimit =
          data.rate_limit?.quota && data.rate_limit?.interval
            ? data.rate_limit
            : undefined
        if (ips.length === 0 && !finalRateLimit) return undefined
        return {
          ip_whitelist: ips.length > 0 ? ips.join('\n') : undefined,
          rate_limit: finalRateLimit,
        }
      })
      .refine(
        (data) => {
          if (!data?.ip_whitelist) return true
          return data.ip_whitelist.split('\n').every(isValidIP)
        },
        {
          message: t('users.actions.schema.ip_invalid'),
          path: ['ip_whitelist'],
        }
      ),
  })

export const getCreateUserSchema = (t: (key: string) => string) =>
  getBaseUserSchema(t).extend({
    password: z
      .string()
      .min(1, t('users.actions.schema.password_required'))
      .min(8, t('users.actions.schema.password_min'))
      .max(256, t('users.actions.schema.password_max')),
  })

export const getUpdateUserSchema = (t: (key: string) => string) =>
  getBaseUserSchema(t).extend({
    password: z
      .string()
      .min(8, t('users.actions.schema.password_min'))
      .max(256, t('users.actions.schema.password_max'))
      .or(z.literal(''))
      .optional()
      .transform((v) => v || undefined),
  })

export type UserFormValues = z.infer<ReturnType<typeof getCreateUserSchema>>
