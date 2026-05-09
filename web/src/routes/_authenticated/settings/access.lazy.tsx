import { AccountAccessList } from '@/features/settings/access'
import { createLazyFileRoute } from '@tanstack/react-router'

export const Route = createLazyFileRoute('/_authenticated/settings/access')({
    component: AccountAccessList,
})
