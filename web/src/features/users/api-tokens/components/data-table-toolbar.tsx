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


import { Table } from '@tanstack/react-table'
import { Input } from '@/components/ui/input'
import { useTranslation } from 'react-i18next'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
// import { useTranslation } from 'react-i18next'

interface DataTableToolbarProps<TData> {
  table: Table<TData>
}

export function DataTableToolbar<TData>({
  table,
}: DataTableToolbarProps<TData>) {
  const { t } = useTranslation()
  const metricsColumn = table.getColumn('metrics_only')
  const metricsFilter = (metricsColumn?.getFilterValue() as string) ?? 'all'
  return (
    <div className='flex items-center justify-between'>
      <div className='flex flex-1 flex-col-reverse items-start gap-y-2 sm:flex-row sm:items-center sm:space-x-2'>
        <Input
          placeholder={t('users.api_tokens.toolbar.search_placeholder')}
          value={(table.getState().globalFilter as string) ?? ''}
          onChange={(event) => {
            table.setGlobalFilter(event.target.value);
          }}
          className='h-8 w-80'
        />
        <Select
          value={metricsFilter}
          onValueChange={(value) =>
            metricsColumn?.setFilterValue(value === 'all' ? undefined : value)
          }
        >
          <SelectTrigger className='h-8 w-[180px]'>
            <SelectValue placeholder={t('apiTokens.filters.label')} />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value='all'>{t('apiTokens.filters.all')}</SelectItem>
            <SelectItem value='metrics'>
              {t('apiTokens.filters.metricsOnly')}
            </SelectItem>
            <SelectItem value='regular'>
              {t('apiTokens.filters.regular')}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  )
}
