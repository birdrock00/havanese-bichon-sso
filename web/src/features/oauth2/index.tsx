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
import { ActionDialog } from './components/action-dialog'
import { getColumns } from './components/columns'
import { TokenDeleteDialog } from './components/delete-dialog'
import { Oauth2Table } from './components/oauth2-table'
import OAuth2Provider, {
  type OAuth2DialogType,
} from './context'
import { Plus } from 'lucide-react'
import Logo from '@/assets/logo.svg'
import { OAuth2Entity } from './data/schema'
import { useQuery } from '@tanstack/react-query'
import { get_oauth2_list } from '@/api/oauth2/api'
import { TableSkeleton } from '@/components/table-skeleton'
import { AuthorizeDialog } from './components/authorize-dialog'
import { FixedHeader } from '@/components/layout/fixed-header'
import { useTranslation } from 'react-i18next'

export default function OAuth2() {
  const { t } = useTranslation()
  const [currentRow, setCurrentRow] = useState<OAuth2Entity | null>(null)
  const [open, setOpen] = useDialogState<OAuth2DialogType>(null)


  const { data: oauth2List, isLoading } = useQuery({
    queryKey: ['oauth2-list'],
    queryFn: get_oauth2_list,
  })

  const columns = getColumns(t)

  return (
    <OAuth2Provider value={{ open, setOpen, currentRow, setCurrentRow }}>
      {/* ===== Top Heading ===== */}
      <FixedHeader />

      <Main>
        <div className="mx-auto w-full max-w-6xl px-4">
          {/* Header */}
          <div className="mb-2 flex items-start flex-wrap gap-x-4 gap-y-2">
            <div className="flex-1 min-w-[300px]">
              <h2 className="text-2xl font-bold tracking-tight">{t('oauth2.title')}</h2>
              <p className="text-muted-foreground">
                {t('oauth2.description')}
              </p>
            </div>
            <div className="flex gap-2 ml-auto">
              <Button className="space-x-1" onClick={() => setOpen("add")}>
                <span>{t('common.add')}</span>
                <Plus size={18} />
              </Button>
            </div>
          </div>
          {/* Table / Empty State */}
          <div className="flex-1 overflow-auto py-1 flex-row lg:space-x-12 space-y-0">
            {isLoading ? (
              <TableSkeleton columns={columns.length} rows={10} />
            ) : oauth2List?.items.length ? (
              <Oauth2Table data={oauth2List.items} columns={columns} />
            ) : (
              <div className="flex h-[450px] shrink-0 items-center justify-center rounded-md border border-dashed">
                <div className="mx-auto flex max-w-[420px] flex-col items-center justify-center text-center">
                  <img
                    src={Logo}
                    className="max-h-[100px] w-auto opacity-20 saturate-0 transition-all duration-300 hover:opacity-100 hover:saturate-100 object-contain"
                    alt="Bichon Logo"
                  />
                  <h3 className="mt-4 text-lg font-semibold">{t('oauth2.noConfigurations')}</h3>
                  <p className="mb-4 mt-2 text-sm text-muted-foreground">
                    {t('oauth2.noConfigurationsDesc')}
                  </p>
                  <Button onClick={() => setOpen("add")}>{t('oauth2.addConfiguration')}</Button>
                </div>
              </div>
            )}
          </div>
        </div>
      </Main>
      <ActionDialog
        key='oauth2-add'
        open={open === 'add'}
        onOpenChange={() => setOpen('add')}
      />

      {currentRow && (
        <>
          <ActionDialog
            key={`oauth2-edit-${currentRow.id}`}
            open={open === 'edit'}
            onOpenChange={() => {
              setOpen('edit')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />

          <TokenDeleteDialog
            key={`oauth2-delete-${currentRow.id}`}
            open={open === 'delete'}
            onOpenChange={() => {
              setOpen('delete')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />

          <AuthorizeDialog
            key={`oauth2-authorize-${currentRow.id}`}
            open={open === 'authorize'}
            onOpenChange={() => {
              setOpen('authorize')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />
        </>
      )}
    </OAuth2Provider>
  )
}
