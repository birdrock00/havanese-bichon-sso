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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.


import { useState } from 'react'
import useDialogState from '@/hooks/use-dialog-state'
import { Button } from '@/components/ui/button'
import { UserActionDialog } from './components/action-dialog'
import { getColumns } from './components/columns'
import { UserDeleteDialog } from './components/delete-dialog'
import { UsersTable } from './components/table'
import UserProvider, {
  type UserDialogType,
} from './context'
import { Plus } from 'lucide-react'
import { TableSkeleton } from '@/components/table-skeleton'
import Logo from '@/assets/logo.svg'
import { list_users, User } from '@/api/users/api'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { useRoles } from '@/hooks/use-roles'
import { UserApiTokensDialog } from './components/api-tokens-dialog'

export default function Users() {
  const { t } = useTranslation()
  const [currentRow, setCurrentRow] = useState<User | null>(null)
  const [open, setOpen] = useDialogState<UserDialogType>(null)
  const { global } = useRoles()

  const { data: users, isLoading } = useQuery({
    queryKey: ['user-list'],
    queryFn: list_users,
  })
  const columns = getColumns(t, global.roles!)

  return (
    <div className="w-full max-w-6xl ml-0 px-4">
      <UserProvider value={{ open, setOpen, currentRow, setCurrentRow }}>
        <div className="w-full">
          <div className="mb-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2">
            <Button
              className="mt-2 sm:mt-0 space-x-1"
              onClick={() => setOpen('add')}
            >
              <span>{t('users.buttons.add')}</span> <Plus size={18} />
            </Button>
          </div>

          <div className="w-full">
            {isLoading ? (
              <TableSkeleton columns={columns.length} rows={10} />
            ) : users?.length ? (
              <div className="overflow-x-auto">
                <UsersTable data={users} columns={columns} />
              </div>
            ) : (
              <div className="flex min-h-[300px] items-center justify-center rounded-md border border-dashed p-4">
                <div className="mx-auto flex w-full max-w-lg flex-col items-center justify-center text-center">
                  <img
                    src={Logo}
                    className="max-h-[100px] w-auto opacity-20 saturate-0 transition-all duration-300 hover:opacity-100 hover:saturate-100 object-contain"
                    alt="Bichon Logo"
                  />
                  <h3 className="mt-4 text-lg font-semibold">{t('users.empty.title')}</h3>
                  <p className="mt-2 mb-4 text-sm text-muted-foreground">
                    {t('users.empty.description')}
                  </p>
                  <Button onClick={() => setOpen('add')}>{t('users.buttons.add_new')}</Button>
                </div>
              </div>
            )}
          </div>

          <UserActionDialog
            key="user-add"
            open={open === 'add'}
            onOpenChange={() => setOpen(null)}
          />

          {currentRow && (
            <>
              <UserActionDialog
                key={`user-edit-${currentRow.id}`}
                currentRow={currentRow}
                open={open === 'edit'}
                onOpenChange={() => {
                  setOpen(null)
                  setTimeout(() => setCurrentRow(null), 500)
                }}
              />

              <UserApiTokensDialog
                key={`api-tokens-${currentRow.id}`}
                currentRow={currentRow}
                open={open === 'api-tokens'}
                onOpenChange={() => {
                  setOpen(null)
                  setTimeout(() => setCurrentRow(null), 500)
                }}
              />

              <UserDeleteDialog
                key={`user-delete-${currentRow.id}`}
                currentRow={currentRow}
                open={open === 'delete'}
                onOpenChange={() => {
                  setOpen(null)
                  setTimeout(() => setCurrentRow(null), 500)
                }}
              />
            </>
          )}
        </div>
      </UserProvider>
    </div>
  )
}