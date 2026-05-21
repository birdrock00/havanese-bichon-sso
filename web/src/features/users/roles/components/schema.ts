import { z } from 'zod'

export const getRoleFormSchema = (t: (key: string) => string) =>
  z.object({
    name: z.string().min(1, t('roles.validation.name_required')),
    role_type: z.enum(['Global', 'Account']),
    permissions: z.array(z.string()).min(1, t('roles.validation.perm_required')),
    description: z.string().optional(),
  })

export type RoleFormValues = z.infer<ReturnType<typeof getRoleFormSchema>>
