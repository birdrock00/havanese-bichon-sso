//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// Audit log page (Pro edition) — query who did what, when.
//

import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Main } from '@/components/layout/main'
import { FixedHeader } from '@/components/layout/fixed-header'
import { TablePagination } from '@/components/pagination'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { TableSkeleton } from '@/components/table-skeleton'
import { DatePicker } from '@/components/date-picker'
import { VirtualizedSelect } from '@/components/virtualized-select'
import { list_audit_log, type AuditRecord } from '@/api/audit/api'
import { list_minimal_users } from '@/api/users/api'
import { minimal_account_list } from '@/api/account/api'
import { useEdition } from '@/hooks/use-edition'
import { useCurrentUser } from '@/hooks/use-current-user'

const PAGE_SIZE = 50

const EVENT_TYPES = [
  'email.viewed',
  'email.deleted',
  'email.exported',
  'email.restored',
  'email.tagged',
  'attachment.downloaded',
  'attachment.previewed',
  'attachment.tagged',
  'user.login',
  'user.created',
  'user.updated',
  'user.removed',
  'role.created',
  'role.updated',
  'role.removed',
  'account.created',
  'account.updated',
  'account.removed',
  'account.download_started',
  'account.download_stopped',
  'account.role_assigned',
  'access_token.created',
  'access_token.removed',
  'oauth2.created',
  'oauth2.updated',
  'oauth2.removed',
  'oauth2.token_stored',
  'import.performed',
  'mailbox.removed',
  'proxy.created',
  'proxy.updated',
  'proxy.removed',
  'sso.login',
  'sso.logout',
  'license.uploaded',
  'search.performed',
  'settings.changed',
] as const

function eventTypeLabel(t: (key: string, defaultValue: string) => string, et: string): string {
  const labels: Record<string, string> = {
    'email.viewed': t('audit.eventTypes.emailViewed', 'Email viewed'),
    'email.deleted': t('audit.eventTypes.emailDeleted', 'Email deleted'),
    'email.exported': t('audit.eventTypes.emailExported', 'Email exported'),
    'email.restored': t('audit.eventTypes.emailRestored', 'Email restored'),
    'email.tagged': t('audit.eventTypes.emailTagged', 'Email tags changed'),
    'attachment.downloaded': t('audit.eventTypes.attachmentDownloaded', 'Attachment downloaded'),
    'attachment.previewed': t('audit.eventTypes.attachmentPreviewed', 'Attachment previewed'),
    'attachment.tagged': t('audit.eventTypes.attachmentTagged', 'Attachment tags changed'),
    'user.login': t('audit.eventTypes.userLogin', 'User login'),
    'user.created': t('audit.eventTypes.userCreated', 'User created'),
    'user.updated': t('audit.eventTypes.userUpdated', 'User updated'),
    'user.removed': t('audit.eventTypes.userRemoved', 'User removed'),
    'role.created': t('audit.eventTypes.roleCreated', 'Role created'),
    'role.updated': t('audit.eventTypes.roleUpdated', 'Role updated'),
    'role.removed': t('audit.eventTypes.roleRemoved', 'Role removed'),
    'account.created': t('audit.eventTypes.accountCreated', 'Account created'),
    'account.updated': t('audit.eventTypes.accountUpdated', 'Account updated'),
    'account.removed': t('audit.eventTypes.accountRemoved', 'Account removed'),
    'account.download_started': t(
      'audit.eventTypes.accountDownloadStarted',
      'Account sync started',
    ),
    'account.download_stopped': t(
      'audit.eventTypes.accountDownloadStopped',
      'Account sync stopped',
    ),
    'account.role_assigned': t('audit.eventTypes.accountRoleAssigned', 'Account access assigned'),
    'access_token.created': t('audit.eventTypes.accessTokenCreated', 'Access token created'),
    'access_token.removed': t('audit.eventTypes.accessTokenRemoved', 'Access token removed'),
    'oauth2.created': t('audit.eventTypes.oauth2Created', 'OAuth2 config created'),
    'oauth2.updated': t('audit.eventTypes.oauth2Updated', 'OAuth2 config updated'),
    'oauth2.removed': t('audit.eventTypes.oauth2Removed', 'OAuth2 config removed'),
    'oauth2.token_stored': t('audit.eventTypes.oauth2TokenStored', 'OAuth2 token stored'),
    'import.performed': t('audit.eventTypes.importPerformed', 'Import performed'),
    'mailbox.removed': t('audit.eventTypes.mailboxRemoved', 'Mailbox removed'),
    'proxy.created': t('audit.eventTypes.proxyCreated', 'Proxy created'),
    'proxy.updated': t('audit.eventTypes.proxyUpdated', 'Proxy updated'),
    'proxy.removed': t('audit.eventTypes.proxyRemoved', 'Proxy removed'),
    'sso.login': t('audit.eventTypes.ssoLogin', 'SSO login'),
    'sso.logout': t('audit.eventTypes.ssoLogout', 'SSO logout'),
    'license.uploaded': t('audit.eventTypes.licenseUploaded', 'License uploaded'),
    'search.performed': t('audit.eventTypes.searchPerformed', 'Search performed'),
    'settings.changed': t('audit.eventTypes.settingsChanged', 'Settings changed'),
  }
  return labels[et] ?? et
}

function formatTime(ts: number): string {
  const d = new Date(ts)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

function describeEvent(rec: AuditRecord): string {
  const p = rec.payload ?? {}
  switch (rec.event_type) {
    case 'email.viewed':
      return typeof p.subject === 'string' ? p.subject : rec.email_id ?? ''
    case 'email.deleted':
      return typeof p.subject === 'string' ? p.subject : rec.email_id ?? ''
    case 'email.exported':
    case 'email.restored':
      return typeof p.subject === 'string' ? p.subject : rec.email_id ?? ''
    case 'email.tagged':
      return typeof p.count === 'number' ? `${p.count} email(s)` : ''
    case 'attachment.downloaded':
    case 'attachment.previewed':
      return typeof p.filename === 'string' ? p.filename : rec.content_hash ?? ''
    case 'attachment.tagged':
      return typeof p.count === 'number' ? `${p.count} attachment(s)` : ''
    case 'search.performed':
      return typeof p.query === 'string' ? `"${p.query}"` : ''
    case 'user.created':
      return typeof p.new_user === 'string' ? p.new_user : ''
    case 'user.updated':
    case 'user.removed':
      return typeof p.target_user === 'string' ? p.target_user : ''
    case 'role.created':
    case 'role.updated':
    case 'role.removed':
      return typeof p.role === 'string' ? p.role : ''
    case 'account.created':
    case 'account.updated':
    case 'account.removed':
      return typeof p.email === 'string' ? p.email : ''
    case 'account.download_started':
      return typeof p.run_gap_fill === 'boolean'
        ? `gap_fill=${p.run_gap_fill}`
        : ''
    case 'account.role_assigned':
      return typeof p.target_user === 'string' ? p.target_user : ''
    case 'access_token.created':
    case 'access_token.removed':
      return typeof p.name === 'string' && p.name
        ? p.name
        : typeof p.target_user === 'string'
          ? p.target_user
          : ''
    case 'oauth2.created':
    case 'oauth2.updated':
    case 'oauth2.removed':
      return typeof p.name === 'string' && p.name ? p.name : ''
    case 'import.performed':
      return typeof p.total === 'number'
        ? `total=${p.total} success=${p.success} failed=${p.failed}`
        : ''
    case 'mailbox.removed':
      return rec.mailbox_id !== null && rec.mailbox_id !== undefined
        ? `mailbox ${rec.mailbox_id}`
        : ''
    case 'proxy.created':
    case 'proxy.updated':
    case 'proxy.removed':
      return typeof p.url === 'string' ? p.url : ''
    case 'license.uploaded':
      return typeof p.email === 'string' ? p.email : ''
    default:
      return ''
  }
}

function PayloadView({ record }: { record: AuditRecord }) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  if (!record.payload || Object.keys(record.payload).length === 0) return null
  return (
    <div className='mt-2'>
      <Button variant='ghost' size='sm' onClick={() => setOpen((v) => !v)}>
        {open ? t('audit.hideDetails', 'Hide details') : t('audit.showDetails', 'Show details')}
      </Button>
      {open && (
        <pre className='mt-2 max-h-64 overflow-auto rounded border p-2 text-xs'>
          {JSON.stringify(record.payload, null, 2)}
        </pre>
      )}
    </div>
  )
}

export default function AuditLog() {
  const { t } = useTranslation()
  const { isPro } = useEdition()
  const { require_any_permission } = useCurrentUser()
  const [page, setPage] = useState(1)
  const [userFilter, setUserFilter] = useState('all')
  const [eventType, setEventType] = useState('all')
  const [accountFilter, setAccountFilter] = useState('all')
  const [startDate, setStartDate] = useState<Date | undefined>()
  const [endDate, setEndDate] = useState<Date | undefined>()
  const [applied, setApplied] = useState({
    user: 'all',
    type: 'all',
    account: 'all',
    start: undefined as Date | undefined,
    end: undefined as Date | undefined,
  })

  const { data: users, isLoading: isUsersLoading } = useQuery({
    queryKey: ['audit-log-users'],
    queryFn: () => list_minimal_users(),
    staleTime: 60_000,
  })

  const { data: accounts, isLoading: isAccountsLoading } = useQuery({
    queryKey: ['audit-log-accounts'],
    queryFn: () => minimal_account_list(),
    staleTime: 60_000,
  })

  const userOptions = [
    { value: 'all', label: t('audit.allTypes', 'All') },
    ...(users ?? []).map((u) => ({
      value: u.username,
      label: `${u.username}${u.email ? ` · ${u.email}` : ''}`,
    })),
  ]

  const accountOptions = [
    { value: 'all', label: t('audit.allTypes', 'All') },
    ...(accounts ?? []).map((a) => ({
      value: String(a.id),
      label: a.email,
    })),
  ]

  const { data, isLoading, isFetching } = useQuery({
    queryKey: ['audit-log', page, applied],
    queryFn: () =>
      list_audit_log({
        page,
        page_size: PAGE_SIZE,
        user: applied.user === 'all' || !applied.user ? undefined : applied.user,
        event_type: applied.type === 'all' || !applied.type ? undefined : applied.type,
        account_id: applied.account === 'all' || !applied.account ? undefined : Number(applied.account),
        start_ms: applied.start ? applied.start.getTime() : undefined,
        end_ms: applied.end
          ? new Date(
            applied.end.getFullYear(),
            applied.end.getMonth(),
            applied.end.getDate() + 1,
          ).getTime()
          : undefined,
      }),
    placeholderData: (prev) => prev,
  })

  const applyFilters = () => {
    setPage(1)
    setApplied({
      user: userFilter,
      type: eventType,
      account: accountFilter,
      start: startDate,
      end: endDate,
    })
  }

  const resetFilters = () => {
    setUserFilter('all')
    setEventType('all')
    setAccountFilter('all')
    setStartDate(undefined)
    setEndDate(undefined)
    setPage(1)
    setApplied({ user: 'all', type: 'all', account: 'all', start: undefined, end: undefined })
  }

  const canView = isPro && require_any_permission(['system:root', 'user:manage', 'data:read:all'])

  if (!canView) {
    return (
      <>
        <FixedHeader />
        <Main>
          <div className='mx-auto w-full max-w-7xl px-4 py-16 text-center text-muted-foreground'>
            {t('audit.forbidden', 'Audit log is available in the Pro edition only.')}
          </div>
        </Main>
      </>
    )
  }

  return (
    <>
      <FixedHeader />
      <Main>
        <div className='mx-auto w-full max-w-7xl px-4'>
          <h1 className='mb-4 text-xl font-semibold'>
            {t('audit.title', 'Audit Log')}
          </h1>

          {/* Filters */}
          <div className='mb-4 flex flex-wrap items-end gap-2'>
            <div className='flex flex-col gap-1'>
              <label className='text-xs text-muted-foreground'>
                {t('audit.user', 'User')}
              </label>
              <VirtualizedSelect
                options={userOptions}
                value={userFilter}
                onSelectOption={(values) => setUserFilter(values[0])}
                placeholder={t('audit.userPlaceholder', 'username')}
                isLoading={isUsersLoading}
                className='h-9 w-52 justify-start text-sm font-normal'
                noItemsComponent={
                  <span className='text-xs'>{t('audit.noUsers', 'No users found')}</span>
                }
              />
            </div>
            <div className='flex flex-col gap-1'>
              <label className='text-xs text-muted-foreground'>
                {t('audit.eventType', 'Event type')}
              </label>
              <Select value={eventType} onValueChange={setEventType}>
                <SelectTrigger className='h-9 w-52 text-sm'>
                  <SelectValue placeholder={t('audit.allTypes', 'All')} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value='all'>{t('audit.allTypes', 'All')}</SelectItem>
                  {EVENT_TYPES.map((et) => (
                    <SelectItem key={et} value={et}>
                      {eventTypeLabel(t, et)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className='flex flex-col gap-1'>
              <label className='text-xs text-muted-foreground'>
                {t('audit.account', 'Account')}
              </label>
              <VirtualizedSelect
                options={accountOptions}
                value={accountFilter}
                onSelectOption={(values) => setAccountFilter(values[0])}
                placeholder={t('audit.accountPlaceholder', 'Select account')}
                isLoading={isAccountsLoading}
                height='240px'
                className='h-9 w-52 justify-start text-sm font-normal'
                noItemsComponent={
                  <span className='text-xs'>{t('audit.noAccounts', 'No accounts found')}</span>
                }
              />
            </div>
            <div className='flex flex-col gap-1'>
              <label className='text-xs text-muted-foreground'>
                {t('audit.startDate', 'Start date')}
              </label>
              <DatePicker
                placeholder={t('audit.startDate', 'Start date')}
                selected={startDate}
                onSelect={setStartDate}
                className='w-44'
              />
            </div>
            <div className='flex flex-col gap-1'>
              <label className='text-xs text-muted-foreground'>
                {t('audit.endDate', 'End date')}
              </label>
              <DatePicker
                placeholder={t('audit.endDate', 'End date')}
                selected={endDate}
                onSelect={setEndDate}
                className='w-44'
              />
            </div>
            <div className='ms-auto flex items-end gap-2'>
              <Button onClick={applyFilters}>{t('audit.apply', 'Apply')}</Button>
              <Button variant='outline' onClick={resetFilters}>
                {t('audit.reset', 'Reset')}
              </Button>
            </div>
          </div>

          {isLoading ? (
            <TableSkeleton columns={6} rows={10} />
          ) : (
            <>
              <div className='overflow-x-auto rounded-md border'>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className='text-xs'>{t('audit.time', 'Time')}</TableHead>
                      <TableHead className='text-xs'>{t('audit.user', 'User')}</TableHead>
                      <TableHead className='text-xs'>{t('audit.eventType', 'Event type')}</TableHead>
                      <TableHead className='text-xs'>{t('audit.detail', 'Detail')}</TableHead>
                      <TableHead className='text-xs'>
                        {t('audit.ip', 'IP')}
                      </TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data?.items?.map((rec) => (
                      <TableRow key={rec.id}>
                        <TableCell className='whitespace-nowrap text-sm'>
                          {formatTime(rec.ts_ms)}
                        </TableCell>
                        <TableCell className='text-sm'>{rec.user}</TableCell>
                        <TableCell>
                          <span className='rounded bg-muted px-1.5 py-0.5 text-sm'>
                            {eventTypeLabel(t, rec.event_type)}
                          </span>
                        </TableCell>
                        <TableCell className='max-w-md'>
                          <div className='truncate text-sm'>{describeEvent(rec) || '—'}</div>
                          <PayloadView record={rec} />
                        </TableCell>
                        <TableCell className='text-sm'>{rec.ip ?? '—'}</TableCell>
                      </TableRow>
                    ))}
                    {data?.items?.length === 0 && (
                      <TableRow>
                        <TableCell colSpan={5} className='py-8 text-center text-muted-foreground'>
                          {t('audit.empty', 'No audit events found')}
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </div>
              {data && data.total > 0 && <div className='mt-2'>
                <TablePagination
                  totalItems={data?.total ?? 0}
                  pageIndex={page - 1}
                  pageSize={PAGE_SIZE}
                  hasNextPage={() => (data?.total ?? 0) > page * PAGE_SIZE}
                  setPageIndex={(i) => setPage(i + 1)}
                  setPageSize={() => { }}
                />
              </div>}
              {isFetching && (
                <div className='mt-2 text-xs text-muted-foreground'>
                  {t('audit.loading', 'Loading…')}
                </div>
              )}
            </>
          )}
        </div>
      </Main>
    </>
  )
}
