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
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useTranslation } from 'react-i18next'

interface PaginationProps {
  totalItems: number
  pageIndex: number,
  pageSize: number,
  hasNextPage: () => boolean,
  setPageIndex: (pageIndex: number) => void,
  setPageSize: (pageSize: number) => void,
}

export function EnvelopeListPagination({
  totalItems,
  pageIndex,
  pageSize,
  hasNextPage,
  setPageIndex,
  setPageSize,
}: PaginationProps) {
  const { t } = useTranslation()
  const pageCount = Math.ceil(totalItems / pageSize)

  const handlePageSizeChange = (value: string) => {
    const newPageSize = Number(value)
    setPageSize(newPageSize)
  }

  const goToPreviousPage = () => {
    const newPageIndex = Math.max(pageIndex - 1, 0)
    setPageIndex(newPageIndex)
  }

  const goToNextPage = () => {
    const newPageIndex = Math.min(pageIndex + 1, pageCount - 1)
    setPageIndex(newPageIndex)
  }

  return (
    <div className='flex items-center justify-between space-x-2 overflow-auto px-2'>
      <div className='hidden flex-1 text-sm text-muted-foreground sm:block'>
        {totalItems} {t("table.results")}
      </div>
      <div className='flex items-center sm:space-x-6 lg:space-x-4'>
        <div className='flex items-center space-x-2'>
          <p className='hidden text-sm font-medium sm:block'>{t("table.rowsPerPage")}</p>
          <Select
            value={`${pageSize}`}
            onValueChange={handlePageSizeChange}
          >
            <SelectTrigger className='h-8 w-[70px]'>
              <SelectValue placeholder={pageSize} />
            </SelectTrigger>
            <SelectContent side='top'>
              {[10, 20, 30, 40, 50].map((size) => (
                <SelectItem key={size} value={`${size}`}>
                  {size}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className='flex items-center justify-center text-sm font-medium'>
           {t("table.page")} {pageIndex + 1} {t("table.of")} {pageCount}
        </div>
        <div className='flex items-center space-x-2'>
          <Button
            variant='outline'
            className='h-8 w-8 p-0'
            onClick={goToPreviousPage}
            disabled={pageIndex === 0}
          >
            <span className='sr-only'>{t("table.prevPage")}</span>
            <ChevronLeftIcon className='h-4 w-4' />
          </Button>
          <Button
            variant='outline'
            className='h-8 w-8 p-0'
            onClick={goToNextPage}
            disabled={!hasNextPage()}
          >
            <span className='sr-only'>{t("table.nextPage")}</span>
            <ChevronRightIcon className='h-4 w-4' />
          </Button>
        </div>
      </div>
    </div>
  )
}