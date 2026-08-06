//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// Audit log route (Pro edition).
//

import { createLazyFileRoute } from '@tanstack/react-router'
import AuditLog from '@/features/audit-log'

export const Route = createLazyFileRoute('/_authenticated/audit-log')({
  component: AuditLog,
})
