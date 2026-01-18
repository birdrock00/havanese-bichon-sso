import { DropdownMenuTrigger } from '@radix-ui/react-dropdown-menu'
import { MixerHorizontalIcon } from '@radix-ui/react-icons'
import { type Table } from '@tanstack/react-table'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu'
import React from 'react'
import { useTranslation } from 'react-i18next'

type DataTableViewOptionsProps<TData> = {
  table: Table<TData>
}

const defaultColumns = (t: (key: string) => string) => [
  { label: t('search.account'), value: "account_email" },
  { label: t('search.mailbox'), value: "mailbox_name" },
  { label: t('search.from'), value: "from" },
  { label: t('search.to'), value: "to" },
  { label: t('search.subject'), value: "subject" },
  { label: t('mail.attachments'), value: "attachments" },
  { label: t('search.size'), value: "size" },
  { label: t('search.date'), value: "date" },
]


export function DataTableViewOptions<TData>({
  table,
}: DataTableViewOptionsProps<TData>) {
  const { t } = useTranslation()


  const columnLabels = React.useMemo(() => {
    return Object.fromEntries(
      defaultColumns(t).map(col => [col.value, col.label])
    )
  }, [t]);


  const visibleColumnKeys = React.useMemo(() => {
    return new Set(defaultColumns(t).map(c => c.value))
  }, [t])

  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <Button
          variant='outline'
          size='sm'
          className='ms-auto hidden h-8 lg:flex rounded-none'
        >
          <MixerHorizontalIcon className='size-4' />
          View
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align='end' className='w-[150px]'>
        <DropdownMenuLabel className='text-xs'>Toggle columns</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {table
          .getAllColumns()
          .filter(column => visibleColumnKeys.has(column.id))
          .map((column) => {
            return (
              <DropdownMenuCheckboxItem
                key={column.id}
                className='capitalize text-xs'
                checked={column.getIsVisible()}
                onCheckedChange={(value) => column.toggleVisibility(!!value)}
              >
                {columnLabels[column.id] ?? column.id}
              </DropdownMenuCheckboxItem>
            )
          })}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
