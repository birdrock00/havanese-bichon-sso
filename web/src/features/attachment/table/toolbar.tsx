import { type Table } from '@tanstack/react-table'
import { DataTableViewOptions } from './view-options'
import { TagFilterPopover } from '../tag-filter-popover'
import { TimePopover } from '../time-popover'
import { SenderFilterPopover } from '../sender-popover'
import { TextSearchInput } from '../text-search-input'
import { MoreFiltersPopover } from '../more-filters-popover'
import { FilterResetButton } from '../filter-reset'
import { MailboxPopover } from '../mailbox-popover'
import { AccountPopover } from '../account-popover'
import { MetadataFilter } from '../attachment-metadata-filter'
import { FileType, Laptop, Tag } from 'lucide-react'

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
            <SenderFilterPopover />
            <TagFilterPopover />

            <MetadataFilter
              type="extension"
              icon={<FileType className="h-3.5 w-3.5" />}
            />
            <MetadataFilter
              type="category"
              icon={<Tag className="h-3.5 w-3.5" />}
            />
            <MetadataFilter
              type="content_type"
              icon={<Laptop className="h-3.5 w-3.5" />}
            />

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