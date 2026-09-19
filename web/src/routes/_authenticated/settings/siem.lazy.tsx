import { createLazyFileRoute } from '@tanstack/react-router'
import { SiemSettings } from '@/features/settings/siem'

export const Route = createLazyFileRoute('/_authenticated/settings/siem')({
  component: SiemSettings,
})
