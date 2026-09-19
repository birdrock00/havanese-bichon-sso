import { createLazyFileRoute } from '@tanstack/react-router'
import { MfaSettings } from '@/features/settings/mfa'

export const Route = createLazyFileRoute('/_authenticated/settings/mfa')({
  component: MfaSettings,
})