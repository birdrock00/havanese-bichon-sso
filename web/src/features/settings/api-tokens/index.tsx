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

import { useCurrentUser } from '@/hooks/use-current-user'
import { Loader2, Plus } from 'lucide-react'
import { useQuery } from '@tanstack/react-query'
import { get_user_tokens } from '@/api/users/api'
import { Skeleton } from '@/components/ui/skeleton'
import Logo from '@/assets/logo.svg'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { TokenCardList } from './token-list'
import { TokensActionDialog } from './access-token-action'
import { useTranslation } from 'react-i18next'

export function APITokens() {
  const { t } = useTranslation()
  const { data: user, isLoading, error } = useCurrentUser()
  const [addOpen, setAddOpen] = useState(false)

  const { data: tokens = [], isLoading: tokensLoading } = useQuery({
    queryKey: ['user-tokens', user?.id!],
    queryFn: () => get_user_tokens(user?.id!),
    enabled: !!user?.id,
  })

  if (isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        <Loader2 className="h-6 w-6 animate-spin" />
      </div>
    )
  }

  if (error || !user) {
    return (
      <div className="p-6 text-red-600">
        {t('apiTokens.page.loadError')}
      </div>
    )
  }

  return (
    <div className="w-full max-w-3xl px-4 sm:px-6 lg:px-8">
      {tokensLoading ? (
        <div className="flex flex-col gap-4 mt-4">
          <Skeleton className="h-16 w-full rounded-lg" />
          <Skeleton className="h-16 w-full rounded-lg" />
          <Skeleton className="h-16 w-full rounded-lg" />
        </div>
      ) : tokens.length === 0 ? (
        <div className="flex h-[450px] items-center justify-center rounded-md border border-dashed mt-4">
          <div className="mx-auto flex max-w-[420px] flex-col items-center justify-center text-center px-4">
            <img
              src={Logo}
              className="max-h-[100px] w-auto opacity-20 saturate-0 transition-all duration-300 hover:opacity-100 hover:saturate-100 object-contain"
              alt="Bichon Logo"
            />
            <h3 className="mt-4 text-lg font-semibold">{t('apiTokens.page.emptyTitle')}</h3>
            <p className="mb-4 mt-2 text-sm text-muted-foreground">
              {t('apiTokens.page.emptyDescription')}
            </p>
            <Button onClick={() => setAddOpen(true)}>
              <span>{t('apiTokens.page.addBtn')}</span>
              <Plus size={18} className="ml-2" />
            </Button>
          </div>
        </div>
      ) : (
        <>
          <div className="flex justify-end mb-4">
            <Button onClick={() => setAddOpen(true)}>
              <span>{t('apiTokens.page.addBtn')}</span>
              <Plus size={18} className="ml-2" />
            </Button>
          </div>

          <ScrollArea className="h-[calc(100vh-16rem)] w-full pr-4 -mr-4 py-1">
            <TokenCardList tokens={tokens} userId={user.id} />
          </ScrollArea>
        </>
      )}

      <TokensActionDialog
        key="api-token-add"
        open={addOpen}
        userId={user.id}
        onOpenChange={setAddOpen}
      />
    </div>
  )
}