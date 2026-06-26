import { createFileRoute } from '@tanstack/react-router'
import ImportPage from '@/features/import'

export const Route = createFileRoute('/_authenticated/import/')({
  component: ImportPage,
})
