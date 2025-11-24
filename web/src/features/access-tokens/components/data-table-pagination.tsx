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


import {
  ChevronLeftIcon,
  ChevronRightIcon,
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
  showSelected: boolean,
  showPageSizeSelector: boolean
}

export function DataTablePagination<TData>({
  table,
  showSelected = true,
  showPageSizeSelector = true
}: DataTablePaginationProps<TData>) {
  const { t } = useTranslation();
  return (
    <div className='flex items-center justify-between overflow-auto px-2'>
      {showSelected && <div className='hidden flex-1 text-sm text-muted-foreground sm:block'>
        {table.getFilteredRowModel().rows.length} {t("table.results")}
      </div>}
      {!showPageSizeSelector && <div className='hidden flex-1 text-sm text-muted-foreground sm:block'>
        10 {t("table.rowsPerPage")}.
      </div>}
      <div className='flex items-center sm:space-x-6 lg:space-x-8 ml-auto'>
        {showPageSizeSelector && <div className='flex items-center space-x-2'>
          <p className='hidden text-sm font-medium sm:block'>{t("table.rowsPerPage")}</p>
          <Select
            value={`${table.getState().pagination.pageSize}`}
            onValueChange={(value) => {
              table.setPageSize(Number(value))
            }}
          >
            <SelectTrigger className='h-8 w-[70px]'>
              <SelectValue placeholder={table.getState().pagination.pageSize} />
            </SelectTrigger>
            <SelectContent side='top'>
              {[10, 20, 30, 40, 50].map((pageSize) => (
                <SelectItem key={pageSize} value={`${pageSize}`}>
                  {pageSize}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>}
        <div className='flex w-[100px] items-center justify-center text-sm font-medium'>
          {t("table.page")} {table.getState().pagination.pageIndex + 1}{" "}
          {t("table.of")} {table.getPageCount()}
        </div>
        <div className='flex items-center space-x-2'>
          <Button
            variant='outline'
            className='h-8 w-8 p-0'
            onClick={() => table.previousPage()}
            disabled={!table.getCanPreviousPage()}
          >
            <span className='sr-only'>{t("table.prevPage")}</span>
            <ChevronLeftIcon className='h-4 w-4' />
          </Button>
          <Button
            variant='outline'
            className='h-8 w-8 p-0'
            onClick={() => table.nextPage()}
            disabled={!table.getCanNextPage()}
          >
            <span className='sr-only'>{t("table.nextPage")}</span>
            <ChevronRightIcon className='h-4 w-4' />
          </Button>
        </div>
      </div>
    </div>
  )
}
