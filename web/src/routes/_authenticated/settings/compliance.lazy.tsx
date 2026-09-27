import { createLazyFileRoute } from '@tanstack/react-router'
import { ComplianceSettings } from '@/features/settings/compliance'

export const Route = createLazyFileRoute('/_authenticated/settings/compliance')({
  component: ComplianceSettings,
})
