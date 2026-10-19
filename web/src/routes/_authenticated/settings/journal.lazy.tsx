import { createLazyFileRoute } from '@tanstack/react-router'
import { JournalSettings } from '@/features/settings/journal'

export const Route = createLazyFileRoute('/_authenticated/settings/journal')({
  component: JournalSettings,
})
