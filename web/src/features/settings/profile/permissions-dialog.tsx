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

import * as React from 'react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { getPermissions, User } from '@/api/users/api'
import { CheckCircle2, XCircle } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { useTranslation } from 'react-i18next'

interface Props {
  currentRow?: User
  open: boolean
  onOpenChange: (open: boolean) => void
  mode: 'global' | 'account'
  accountId?: number
}


function getGlobalCategories(t: (key: string) => string) {
  return [
    {
      title: t('permission.category.system_identity'),
      keys: [
        'system:access',
        'system:root',
        'user:manage',
        'user:view',
        'token:manage',
        'account:create',
      ],
    },
    {
      title: t('permission.category.global_data'),
      keys: [
        'account:manage:all',
        'data:read:all',
        'data:manage:all',
        'data:raw:download:all',
        'data:delete:all',
        'data:export:batch:all',
      ],
    },
  ]
}

function getAccountCategories(t: (key: string) => string) {
  return [
    {
      title: t('permission.category.account'),
      keys: [
        'account:manage',
        'account:read_details',
        'data:read',
        'data:manage',
        'data:raw:download',
        'data:delete',
        'data:export:batch',
        'data:import:batch',
      ],
    },
  ]
}

export function PermissionsDialog({
  currentRow,
  open,
  onOpenChange,
  mode,
  accountId,
}: Props) {
  const { t } = useTranslation()


  const ownedPermissions = React.useMemo<string[]>(() => {
    if (!currentRow) return []

    if (mode === 'global') {
      return currentRow.global_permissions ?? []
    }

    if (mode === 'account' && accountId != null) {
      return currentRow.account_permissions?.[accountId] ?? []
    }

    return []
  }, [currentRow, mode, accountId])


  const permissions = React.useMemo(() => {
    const list = getPermissions(t)
    return new Map(list.map((p) => [p.value, p]))
  }, [t])

  const categories =
    mode === 'global'
      ? getGlobalCategories(t)
      : getAccountCategories(t)
      
  const title =
    mode === 'global'
      ? t('permission.dialog.global_title')
      : t('permission.dialog.account_title')

  const description =
    mode === 'global'
      ? t('permission.dialog.global_description')
      : t('permission.dialog.account_description')

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl w-[90vw] overflow-hidden flex flex-col max-h-[90vh]">
        <DialogHeader className="pb-4 border-b">
          <div className="flex items-center gap-3">
            <DialogTitle>{title}</DialogTitle>
            <Badge variant="outline" className="text-[10px]">
              {mode === 'global'
                ? t('permission.scope.global')
                : t('permission.scope.account', { id: accountId })}
            </Badge>
          </div>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto py-4">
          <div className="grid gap-6 px-1">
            {categories.map((cat) => (
              <div key={cat.title} className="flex flex-col">
                <h3 className="text-[11px] font-bold text-slate-500 border-l-4 border-blue-500 pl-2 mb-4 uppercase tracking-widest">
                  {cat.title}
                </h3>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                  {cat.keys.map((key) => {
                    const item = permissions.get(key)
                    if (!item) return null

                    const hasPermission =
                      ownedPermissions.includes(item.value)

                    return (
                      <div
                        key={item.value}
                        className={cn(
                          'flex items-center gap-2.5 p-2 rounded-md transition-all text-xs border',
                          hasPermission
                            ? 'bg-green-50/40 border-green-100 text-green-800 shadow-sm'
                            : 'bg-slate-50/30 border-transparent text-slate-400 opacity-60',
                        )}
                      >
                        {hasPermission ? (
                          <CheckCircle2 className="w-3.5 h-3.5 text-green-600 shrink-0" />
                        ) : (
                          <XCircle className="w-3.5 h-3.5 text-slate-300 shrink-0" />
                        )}

                        <div className="flex flex-col min-w-0 flex-1">
                          <span
                            className={cn(
                              'font-semibold text-sm leading-none truncate',
                              hasPermission
                                ? 'text-slate-900'
                                : 'text-slate-500',
                            )}
                          >
                            {item.label}
                          </span>
                          <span className="text-xs opacity-70 font-mono mt-1 truncate">
                            {item.value}
                          </span>
                        </div>
                      </div>
                    )
                  })}
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="flex justify-end pt-4 border-t mt-auto">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onOpenChange(false)}
          >
            {t('common.close')}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}