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

import { ChevronLeft, ChevronRight } from 'lucide-react'

import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

interface Props {
  pageIndex: number
  pageCount: number
  pageSize: number
  hasNextPage: () => boolean
  setPageIndex: (pageIndex: number) => void
  setPageSize: (pageSize: number) => void
}

/**
 * Narrow pagination for the mailbox view's middle column.
 *
 * The shared `TablePagination` is built for full-width pages (results label,
 * rows-per-page label, page input, first/last buttons); in a ~500px pane it
 * overflows. This bar only shows prev / "current / total" / next plus a
 * compact page-size select.
 */
export function MessagePagination({
  pageIndex,
  pageCount,
  pageSize,
  hasNextPage,
  setPageIndex,
  setPageSize,
}: Props) {
  return (
    <div className="flex shrink-0 items-center justify-between gap-2 px-1 pt-1">
      <div className="flex items-center gap-1.5">
        <Button
          variant="outline"
          className="h-7 w-7 p-0"
          onClick={() => setPageIndex(Math.max(pageIndex - 1, 0))}
          disabled={pageIndex <= 0}
        >
          <ChevronLeft className="h-3.5 w-3.5" />
        </Button>
        <span className="min-w-[52px] text-center text-[11px] tabular-nums text-muted-foreground">
          {pageIndex + 1} / {pageCount}
        </span>
        <Button
          variant="outline"
          className="h-7 w-7 p-0"
          onClick={() => setPageIndex(pageIndex + 1)}
          disabled={!hasNextPage()}
        >
          <ChevronRight className="h-3.5 w-3.5" />
        </Button>
      </div>

      <Select
        value={String(pageSize)}
        onValueChange={(value) => setPageSize(Number(value))}
      >
        <SelectTrigger className="h-7 w-[64px] text-[11px]">
          <SelectValue placeholder={pageSize} />
        </SelectTrigger>
        <SelectContent side="top">
          {[20, 50, 100, 200].map((size) => (
            <SelectItem key={size} value={String(size)} className="text-xs">
              {size}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}
