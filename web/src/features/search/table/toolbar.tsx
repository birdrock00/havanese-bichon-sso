import { type Table } from '@tanstack/react-table'
import { DataTableViewOptions } from './view-options'
import { TagFilterPopover } from '../tag-filter-popover'
import { AccountMailboxFilter } from '../account-mailbox-filter'
import { TimePopover } from '../time-popover'
import { MailFilterPopover } from '../contact-popover'
import { TextSearchInput } from '../text-search-input'
import { MoreFiltersPopover } from '../more-filters-popover'
import { FilterResetButton } from '../filter-reset'

type DataTableToolbarProps<TData> = {
  table: Table<TData>
}

export function DataTableToolbar<TData>({
  table,
}: DataTableToolbarProps<TData>) {



  return (
    <div className="flex flex-col gap-1 px-1 py-1 lg:flex-row lg:items-center lg:gap-1">
      <div className="flex-1">
        <TextSearchInput />
      </div>
      <div className="flex flex-wrap items-center gap-1 lg:flex-nowrap lg:justify-end">
        <div className="flex flex-wrap items-center gap-1 lg:flex-nowrap">
          <TagFilterPopover />
          <AccountMailboxFilter />
          <MailFilterPopover />
          <TimePopover />
          <MoreFiltersPopover />
          <FilterResetButton />
        </div>
        <div className="flex-shrink-0 ml-auto lg:ml-0">
          <DataTableViewOptions table={table} />
        </div>
      </div>
    </div>
  )
}