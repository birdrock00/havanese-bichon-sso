import { createLazyFileRoute } from '@tanstack/react-router'
import { ApprovalSettings } from '@/features/settings/approvals'

export const Route = createLazyFileRoute('/_authenticated/settings/approvals')({
  component: ApprovalSettings,
})
