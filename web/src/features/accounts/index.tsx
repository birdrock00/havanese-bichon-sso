//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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


import { useState } from 'react'
import useDialogState from '@/hooks/use-dialog-state'
import { Button } from '@/components/ui/button'
import { Main } from '@/components/layout/main'
import { AccountActionDialog } from './components/action-dialog'
import { useColumns } from './components/columns'
import { AccountDeleteDialog } from './components/delete-dialog'
import { AccountTable } from './components/table'
import AccountProvider, {
  type AccountDialogType,
} from './context'
import { MoreVertical, Plus } from 'lucide-react'
import Logo from '@/assets/logo.svg'
import { AccountModel } from './data/schema'
import { AccountDetailDrawer } from './components/account-detail'
import { list_accounts } from '@/api/account/api'
import { TableSkeleton } from '@/components/table-skeleton'
import { useQuery } from '@tanstack/react-query'
import { OAuth2TokensDialog } from './components/oauth2-tokens'
import { RunningStateDialog } from './components/running-state-dialog'
import { FixedHeader } from '@/components/layout/fixed-header'
import { SyncFoldersDialog } from './components/sync-folders'
import { NoSyncAccountDialog } from './components/nosync-dialog'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { useTranslation } from 'react-i18next'

export default function Accounts() {
  const { t } = useTranslation()
  const columns = useColumns()
  // Dialog states
  const [currentRow, setCurrentRow] = useState<AccountModel | null>(null)
  const [open, setOpen] = useDialogState<AccountDialogType>(null)

  const { data: accountList, isLoading } = useQuery({
    queryKey: ['account-list'],
    queryFn: list_accounts,
  })

  const hasAccounts = accountList != null && accountList.items.length > 0;

  return (
    <AccountProvider value={{ open, setOpen, currentRow, setCurrentRow }}>
      <FixedHeader />

      <Main>
        <div className="mx-auto w-full max-w-7xl px-4">
          {/* Header Section */}
          <div className='mb-2 flex items-center justify-between flex-wrap gap-x-4 gap-y-2'>
            <div>
              <h2 className='text-2xl font-bold tracking-tight'>{t('accounts.title')}</h2>
              <p className='text-muted-foreground'>
                {t('accounts.description')}
              </p>
            </div>
            <div className="flex gap-2">
              <div className="flex rounded-md shadow-sm">
                <Button
                  onClick={() => setOpen("add-imap")}
                  className="rounded-r-none border-r-0"
                >
                  <Plus className="h-4 w-4" />
                  {t('accounts.addImap')}
                </Button>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button
                      size="icon"
                      className="h-9 w-9 rounded-l-none border-l-0"
                    >
                      <MoreVertical className="h-4 w-4" />
                      <span className="sr-only">{t('accounts.moreAccountTypes')}</span>
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem onClick={() => setOpen("add-nosync")}>
                      <Plus className="h-4 w-4" />
                      {t('accounts.addNoSync')}
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            </div>
          </div>

          {/* Table / Empty State Section */}
          <div className='flex-1 overflow-auto py-1 flex-row lg:space-x-12 space-y-0'>
            {isLoading ? (
              <TableSkeleton columns={columns.length} rows={10} />
            ) : hasAccounts ? (
              <AccountTable data={accountList.items} columns={columns} />
            ) : (
              <div className="flex h-[450px] shrink-0 items-center justify-center rounded-md border border-dashed">
                <div className="mx-auto flex max-w-[420px] flex-col items-center justify-center text-center">
                  <img
                    src={Logo}
                    className="max-h-[100px] w-auto opacity-20 saturate-0 transition-all duration-300 hover:opacity-100 hover:saturate-100 object-contain"
                    alt="Bichon Logo"
                  />
                  <h3 className="mt-4 text-lg font-semibold">{t('accounts.noAccountConfigurations')}</h3>
                  <p className="mb-4 mt-2 text-sm text-muted-foreground">
                    {t('accounts.noAccountConfigurationsDesc')}
                  </p>
                  <div className="mt-4 flex flex-col items-center gap-3 sm:flex-row sm:flex-wrap sm:justify-center sm:gap-4">
                    <Button variant="default" className="w-64" onClick={() => setOpen("add-imap")}>
                      {t('accounts.addConfiguration')}
                    </Button>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </Main>


      <AccountActionDialog
        key='imap-account-add'
        open={open === 'add-imap'}
        onOpenChange={() => setOpen('add-imap')}
      />

      <NoSyncAccountDialog
        key='nosync-account-add'
        open={open === 'add-nosync'}
        onOpenChange={() => setOpen('add-nosync')}
      />

      {currentRow && (
        <>
          <AccountActionDialog
            key={`imap-account-edit-${currentRow.id}`}
            open={open === 'edit-imap'}
            onOpenChange={() => {
              setOpen('edit-imap')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />
          <NoSyncAccountDialog
            key={`nosync-account-edit-${currentRow.id}`}
            open={open === 'edit-nosync'}
            onOpenChange={() => {
              setOpen('edit-nosync')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />
          <RunningStateDialog
            key='running-state'
            open={open === 'running-state'}
            onOpenChange={() => {
              setOpen('running-state')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />
          <AccountDeleteDialog
            key={`account-delete-${currentRow.id}`}
            open={open === 'delete'}
            onOpenChange={() => {
              setOpen('delete')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />
          <SyncFoldersDialog
            key={`sync-folders-${currentRow.id}`}
            open={open === 'sync-folders'}
            onOpenChange={() => {
              setOpen('sync-folders')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />

          <AccountDetailDrawer
            open={open === 'detail'}
            onOpenChange={() => setOpen('detail')}
            currentRow={currentRow}
          />

          <OAuth2TokensDialog open={open === 'oauth2'}
            onOpenChange={() => setOpen('oauth2')}
            currentRow={currentRow}
          />
        </>
      )}
    </AccountProvider>
  )
}
