import { type Table } from '@tanstack/react-table'
import { DataTableViewOptions } from './view-options'
import { TagFilterPopover } from '../tag-filter-popover'
import { TimePopover } from '../time-popover'
import { MailFilterPopover } from '../contact-popover'
import { TextSearchInput } from '../text-search-input'
import { MoreFiltersPopover } from '../more-filters-popover'
import { FilterResetButton } from '../filter-reset'
import { MailboxPopover } from '../mailbox-popover'
import { AccountPopover } from '../account-popover'

type DataTableToolbarProps<TData> = {
  table: Table<TData>
}

export function DataTableToolbar<TData>({
  table,
}: DataTableToolbarProps<TData>) {
  return (
    <div className="flex flex-col gap-1 p-1 bg-background">
      <div className="mb-4 flex items-center justify-center w-full">
        <div className="w-full max-w-3xl">
          <TextSearchInput />
        </div>
      </div>
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2 sm:gap-1">
        <div className="flex items-center gap-2 flex-wrap w-full sm:w-auto">
          <div className="flex items-center gap-1.5 flex-wrap">
            <AccountPopover />
            <MailboxPopover />
            <MailFilterPopover />
            <TagFilterPopover />
            <MoreFiltersPopover />
          </div>
          <FilterResetButton />
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <TimePopover />
          <DataTableViewOptions table={table} />
        </div>
      </div>
    </div>
  )
}