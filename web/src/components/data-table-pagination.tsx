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
// Pagination for the tanstack-table pages.
//
// One implementation instead of a copy per feature. The copies had drifted only
// in their localStorage key; `storageKey` keeps that behaviour per page while
// the markup, typography and page-size options stay identical everywhere.
// Reading the stored size back is the table's job (it is part of the table's
// initial state); this component only writes it.
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  DoubleArrowLeftIcon,
  DoubleArrowRightIcon,
} from '@radix-ui/react-icons'
import { Table } from '@tanstack/react-table'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useTranslation } from 'react-i18next'

interface DataTablePaginationProps<TData> {
  table: Table<TData>
  showSelected?: boolean
  showPageSizeSelector?: boolean
  /** localStorage key under which the chosen page size is remembered. */
  storageKey?: string
}

export function DataTablePagination<TData>({
  table,
  showSelected = false,
  showPageSizeSelector = true,
  storageKey,
}: DataTablePaginationProps<TData>) {
  const { t } = useTranslation()

  return (
    <div className='flex items-center justify-between overflow-auto px-2'>
      {showSelected && (
        <div className='hidden flex-1 text-xs text-muted-foreground sm:block'>
          {t('table.pagination.selected', {
            selected: table.getFilteredSelectedRowModel().rows.length,
            total: table.getFilteredRowModel().rows.length,
          })}
        </div>
      )}
      {!showPageSizeSelector && (
        <div className='hidden flex-1 text-xs text-muted-foreground sm:block'>
          {t('table.pagination.fixed_page_size', { size: 10 })}
        </div>
      )}
      <div className='flex items-center sm:space-x-6 lg:space-x-8 ml-auto'>
        {showPageSizeSelector && (
          <div className='flex items-center space-x-2'>
            <p className='hidden text-xs font-medium sm:block'>
              {t('table.pagination.rows_per_page')}
            </p>
            <Select
              value={`${table.getState().pagination.pageSize}`}
              onValueChange={(value) => {
                if (storageKey) {
                  localStorage.setItem(storageKey, value)
                }
                table.setPageSize(Number(value))
              }}
            >
              <SelectTrigger className='h-8 w-[70px] text-xs'>
                <SelectValue
                  placeholder={table.getState().pagination.pageSize}
                />
              </SelectTrigger>
              <SelectContent side='top'>
                {[10, 20, 30, 40, 50].map((pageSize) => (
                  <SelectItem key={pageSize} value={`${pageSize}`} className='text-xs'>
                    {pageSize}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}
        <div className='flex w-[130px] items-center justify-center text-xs font-medium'>
          {t('table.pagination.page_info', {
            page: table.getState().pagination.pageIndex + 1,
            total: table.getPageCount(),
          })}
        </div>
        <div className='flex items-center space-x-2'>
          <Button
            variant='outline'
            className='hidden h-8 w-8 p-0 lg:flex'
            onClick={() => table.setPageIndex(0)}
            disabled={!table.getCanPreviousPage()}
          >
            <span className='sr-only'>{t('table.pagination.first')}</span>
            <DoubleArrowLeftIcon className='h-4 w-4' />
          </Button>
          <Button
            variant='outline'
            className='h-8 w-8 p-0'
            onClick={() => table.previousPage()}
            disabled={!table.getCanPreviousPage()}
          >
            <span className='sr-only'>{t('table.pagination.previous')}</span>
            <ChevronLeftIcon className='h-4 w-4' />
          </Button>
          <Button
            variant='outline'
            className='h-8 w-8 p-0'
            onClick={() => table.nextPage()}
            disabled={!table.getCanNextPage()}
          >
            <span className='sr-only'>{t('table.pagination.next')}</span>
            <ChevronRightIcon className='h-4 w-4' />
          </Button>
          <Button
            variant='outline'
            className='hidden h-8 w-8 p-0 lg:flex'
            onClick={() => table.setPageIndex(table.getPageCount() - 1)}
            disabled={!table.getCanNextPage()}
          >
            <span className='sr-only'>{t('table.pagination.last')}</span>
            <DoubleArrowRightIcon className='h-4 w-4' />
          </Button>
        </div>
      </div>
    </div>
  )
}
