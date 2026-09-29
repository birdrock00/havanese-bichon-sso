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
import { RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'

interface SearchRefreshButtonProps {
  onRefresh: () => Promise<unknown> | unknown
  isRefreshing: boolean
}

// Compact icon button that re-runs the current query against the server.
// Shown in the search / attachment toolbar next to the other view controls;
// the icon spins while a fetch is in flight and the button is disabled to
// avoid double-fetching.
export function SearchRefreshButton({
  onRefresh,
  isRefreshing,
}: SearchRefreshButtonProps) {
  const { t } = useTranslation()
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant='outline'
          size='icon'
          className='h-6 w-6 rounded-none'
          onClick={() => onRefresh()}
          disabled={isRefreshing}
          aria-label={t('search.refresh')}
        >
          <RefreshCw
            className={cn('h-3.5 w-3.5', isRefreshing && 'animate-spin')}
          />
        </Button>
      </TooltipTrigger>
      <TooltipContent>{t('search.refresh')}</TooltipContent>
    </Tooltip>
  )
}
