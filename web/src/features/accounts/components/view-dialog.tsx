//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// Creating and editing a View account.
//
// A view stores nothing; it is a read-only, filter-scoped window onto one or
// more other accounts. The form therefore collects only what defines that
// window: the sources and, per source, the filter that is ANDed with whatever
// the viewer searches for. Granting someone access is a separate step (assign
// `data:read` on the view in the access dialog), which is why this dialog says
// so rather than implying that creating the view shares it.
import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { AxiosError } from 'axios'
import { Loader2, Plus, Trash2 } from 'lucide-react'
import { toast } from '@/hooks/use-toast'
import { useCurrentUser } from '@/hooks/use-current-user'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { AddressMultiSelect } from '@/components/address-multi-select'
import { ScrollArea } from '@/components/ui/scroll-area'
import { cn } from '@/lib/utils'
import {
  create_account,
  list_accounts,
  update_account,
  type AccountModel,
} from '@/api/account/api'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentRow?: AccountModel | null
}

interface SourceDraft {
  account_id: string
  any_participant: string[]
  any_recipient: string[]
}

const emptySource = (): SourceDraft => ({
  account_id: '',
  any_participant: [],
  any_recipient: [],
})

const errorMessage = (error: unknown): string =>
  (error as AxiosError<{ message?: string }>)?.response?.data?.message ??
  (error as Error)?.message ??
  'Unknown error'

export function ViewAccountDialog({ open, onOpenChange, currentRow }: Props) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { require_any_permission } = useCurrentUser()
  const isEdit = !!currentRow

  const { data: accountList } = useQuery({
    queryKey: ['account-list'],
    queryFn: list_accounts,
  })

  // Every account except views: a view over a view would chain filters and
  // make the effective scope impossible to reason about (the backend refuses
  // it too).
  const candidates = useMemo(
    () =>
      (accountList?.items ?? [])
        .filter((a) => a.account_type !== 'View' && a.id !== currentRow?.id)
        .map((a) => ({
          account: a,
          // The backend requires `account:manage` on every source. Accounts
          // the operator cannot administer stay visible but disabled, so the
          // reason is on screen rather than in a 403 after saving.
          manageable: require_any_permission(
            ['system:root', 'account:manage', 'account:manage:all'],
            a.id
          ),
        })),
    [accountList, currentRow, require_any_permission]
  )

  const initial = useMemo<SourceDraft[]>(() => {
    const sources = currentRow?.view?.sources ?? []
    if (!sources.length) return [emptySource()]
    return sources.map((s) => ({
      account_id: String(s.account_id),
      any_participant: s.any_participant ?? [],
      any_recipient: s.any_recipient ?? [],
    }))
  }, [currentRow])

  const [email, setEmail] = useState(currentRow?.email ?? '')
  const [name, setName] = useState(currentRow?.account_name ?? '')
  const [enabled, setEnabled] = useState(currentRow?.enabled ?? true)
  const [sources, setSources] = useState<SourceDraft[]>(initial)

  // `currentRow` changes after the list loads; reset the form when the dialog
  // opens for a different row rather than keeping the previous draft.
  const [loadedFor, setLoadedFor] = useState<string | null>(null)
  const rowKey = currentRow ? `edit-${currentRow.id}` : 'create'
  if (open && loadedFor !== rowKey) {
    setLoadedFor(rowKey)
    setEmail(currentRow?.email ?? '')
    setName(currentRow?.account_name ?? '')
    setEnabled(currentRow?.enabled ?? true)
    setSources(initial)
  }

  const updateSource = (index: number, patch: Partial<SourceDraft>) =>
    setSources((prev) => prev.map((s, i) => (i === index ? { ...s, ...patch } : s)))

  const cleanedSources = sources
    .filter((s) => s.account_id !== '')
    .map((s) => ({
      account_id: Number(s.account_id),
      any_participant: s.any_participant,
      any_recipient: s.any_recipient,
    }))

  const canSave =
    email.trim().length > 0 &&
    cleanedSources.length > 0 &&
    !sources.some((s) => s.account_id === '')

  const save = useMutation({
    mutationFn: async () => {
      const payload: Record<string, unknown> = {
        email: email.trim(),
        account_name: name.trim() || null,
        enabled,
        // Required by `AccountCreateRequest` (a plain `bool`, not an
        // `Option`): the parser reports a missing one as "boolean, found null".
        use_dangerous: false,
        view: { sources: cleanedSources },
      }
      if (isEdit) {
        return update_account(currentRow!.id, payload)
      }
      return create_account({ ...payload, account_type: 'View' })
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['account-list'] })
      toast({
        title: isEdit
          ? t('accounts.accountUpdated')
          : t('accounts.accountCreated'),
        description: isEdit
          ? t('accounts.accountUpdatedDesc')
          : t('accounts.accountCreatedDesc'),
      })
      onOpenChange(false)
    },
    onError: (error) => {
      toast({
        variant: 'destructive',
        title: isEdit
          ? t('accounts.accountUpdateFailed')
          : t('accounts.accountCreationFailed'),
        description: errorMessage(error),
      })
    },
  })

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='max-h-[90vh] sm:max-w-4xl'>
        <DialogHeader>
          <DialogTitle>
            {isEdit
              ? t('accounts.viewDialogEditTitle', 'Edit view account')
              : t('accounts.viewDialogTitle', 'Create a view account')}
          </DialogTitle>
          <DialogDescription>
            {t(
              'accounts.viewDialogDesc',
              'A read-only view over other accounts. Each source is narrowed by its own filter, ANDed with whatever the viewer searches for. Grant a user data:read on this account in the access dialog to give them exactly that scope.'
            )}
          </DialogDescription>
        </DialogHeader>

        <div className='space-y-4'>
          <div className='grid gap-4 sm:grid-cols-2'>
            <div className='space-y-2'>
              <Label htmlFor='view-email'>
                {t('accounts.viewEmail', 'Account e-mail')}
              </Label>
              <Input
                id='view-email'
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder='zhang@firma.de'
              />
              <p className='text-xs text-muted-foreground'>
                {t(
                  'accounts.viewEmailHint',
                  'Shown in the account list. It does not need to be a real mailbox.'
                )}
              </p>
            </div>
            <div className='space-y-2'>
              <Label htmlFor='view-name'>
                {t('accounts.accountName', 'Account name')}
              </Label>
              <Input
                id='view-name'
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
              <div className='flex items-center justify-between pt-1'>
                <Label htmlFor='view-enabled'>
                  {t('accounts.enabled', 'Enabled')}
                </Label>
                <Switch
                  id='view-enabled'
                  checked={enabled}
                  onCheckedChange={setEnabled}
                />
              </div>
            </div>
          </div>

          <div className='space-y-2'>
            <div className='flex items-center justify-between'>
              <Label>{t('accounts.viewSources', 'Sources')}</Label>
              <div className='text-right'>
                <p className='text-xs text-muted-foreground'>
                  {t(
                    'accounts.viewSourceFilterHint',
                    'Leave the filters empty to expose the whole source account read-only.'
                  )}
                </p>
                <p className='text-xs text-muted-foreground'>
                  {t(
                    'accounts.viewSourceDisabledHint',
                    'Accounts you cannot manage are disabled.'
                  )}
                </p>
              </div>
            </div>

            {/* The list grows with every source; without its own scroll area
                a handful of them pushes the footer (and the save button) off
                the dialog. Radix' viewport is `height: 100%` of the root, so
                the root needs a definite height — `max-h` alone would leave
                the viewport unconstrained and nothing would scroll. */}
            <ScrollArea className={cn('pr-3', sources.length > 2 && 'h-[40vh]')}>
              <div className='space-y-3'>
                {sources.length === 0 && (
                  <div className='rounded-md border border-dashed p-4 text-center text-xs text-muted-foreground'>
                    {t(
                      'accounts.viewNoSources',
                      'Add at least one source to save this view.'
                    )}
                  </div>
                )}
                {sources.map((source, index) => {
                  // A source the operator does not manage stays visible and
                  // keeps its filters, but cannot be changed or removed: the
                  // backend allows it to remain unchanged on save.
                  const rowManageable =
                    source.account_id === '' ||
                    require_any_permission(
                      ['system:root', 'account:manage', 'account:manage:all'],
                      Number(source.account_id)
                    )
                  // A source the picker no longer offers is either deleted or
                  // invisible to this operator — either way the select renders
                  // empty and the row looks broken. Saying so, next to an
                  // enabled remove button, is the difference between "clear
                  // this row" and "why is this blank?".
                  const sourceAvailable =
                    source.account_id === '' ||
                    candidates.some(
                      (candidate) =>
                        String(candidate.account.id) === source.account_id
                    )
                  return (
                  <div
                    key={index}
                    className={cn(
                      'grid gap-2 rounded-md border p-3 sm:grid-cols-[1fr_1.2fr_1.2fr_auto]',
                      !rowManageable && 'opacity-60'
                    )}
                  >
                    <div className='space-y-1'>
                      <Label className='text-xs'>
                        {t('accounts.viewSourceAccount', 'Source account')}
                      </Label>
                      <Select
                        value={source.account_id}
                        disabled={!rowManageable}
                        onValueChange={(v) =>
                          updateSource(index, { account_id: v })
                        }
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {candidates.map(({ account, manageable }) => (
                            <SelectItem
                              key={account.id}
                              value={String(account.id)}
                              disabled={!manageable}
                            >
                              {account.account_name || account.email}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <div className='space-y-1'>
                      <Label className='text-xs'>
                        {t(
                          'accounts.viewSourceParticipant',
                          'Participant address (optional)'
                        )}
                      </Label>
                      <AddressMultiSelect
                        value={source.any_participant}
                        disabled={!rowManageable}
                        onChange={(next) =>
                          updateSource(index, { any_participant: next })
                        }
                        placeholder={t(
                          'address_select.searchPlaceholder',
                          'Search contacts or type an address'
                        )}
                      />
                    </div>
                    <div className='space-y-1'>
                      <Label className='text-xs'>
                        {t(
                          'accounts.viewSourceRecipient',
                          'Recipient address (optional)'
                        )}
                      </Label>
                      <AddressMultiSelect
                        value={source.any_recipient}
                        disabled={!rowManageable}
                        onChange={(next) =>
                          updateSource(index, { any_recipient: next })
                        }
                        placeholder={t(
                          'address_select.searchPlaceholder',
                          'Search contacts or type an address'
                        )}
                      />
                    </div>
                    <div className='flex items-end'>
                      {/* Removing the last source is allowed: a disabled
                          button with no explanation reads as "broken". Save
                          stays disabled until a source exists again, and the
                          empty state below says why.

                          Removal is also allowed for a source the operator
                          does not manage — the backend accepts it, because
                          dropping a source only narrows the view. Without that
                          a view whose source account was deleted would keep a
                          dangling row that no non-admin could ever clear: the
                          account no longer exists, so `rowManageable` is false
                          and the select renders empty. */}
                      <Button
                        type='button'
                        variant='ghost'
                        size='icon'
                        onClick={() =>
                          setSources((prev) =>
                            prev.filter((_, i) => i !== index)
                          )
                        }
                      >
                        <Trash2 className='h-4 w-4' />
                      </Button>
                    </div>
                    {!sourceAvailable && (
                      <p className='text-[11px] text-destructive sm:col-span-4'>
                        {t('accounts.viewSourceMissing')}
                      </p>
                    )}
                    {!rowManageable && sourceAvailable && (
                      <p className='text-[11px] text-muted-foreground sm:col-span-4'>
                        {t(
                          'accounts.viewSourceReadOnly',
                          'You do not manage this account; it is shown read-only.'
                        )}
                      </p>
                    )}
                  </div>
                  )
                })}
              </div>
            </ScrollArea>

            <Button
              type='button'
              variant='outline'
              size='sm'
              onClick={() => setSources((prev) => [...prev, emptySource()])}
            >
              <Plus className='mr-1 h-4 w-4' />
              {t('accounts.viewAddSource', 'Add source')}
            </Button>
          </div>
        </div>

        <DialogFooter>
          <Button
            variant='outline'
            onClick={() => onOpenChange(false)}
            disabled={save.isPending}
          >
            {t('common.cancel')}
          </Button>
          <Button onClick={() => save.mutate()} disabled={!canSave}>
            {save.isPending && <Loader2 className='mr-2 h-4 w-4 animate-spin' />}
            {t('accounts.viewSave', 'Save view')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
