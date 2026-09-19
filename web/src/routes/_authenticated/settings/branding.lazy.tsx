import { createLazyFileRoute } from '@tanstack/react-router'
import { BrandingSettings } from '@/features/settings/branding'

export const Route = createLazyFileRoute('/_authenticated/settings/branding')({
  component: BrandingSettings,
})
