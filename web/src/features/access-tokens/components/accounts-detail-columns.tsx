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


import { ColumnDef } from '@tanstack/react-table'
import LongText from '@/components/long-text'
import { AccountInfo } from '../data/schema'
import { DataTableColumnHeader } from './data-table-column-header'

export const getColumns = (t: (key: string) => string): ColumnDef<AccountInfo>[] => [
  {
    accessorKey: 'id',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('accessTokens.accountId')} />
    ),
    cell: ({ row }) => (
      <LongText className='max-w-80'>{row.original.id}</LongText>
    ),
    meta: { className: 'w-80' },
    enableHiding: false,
    enableSorting: false
  },
  {
    accessorKey: 'email',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('accounts.email')} />
    ),
    cell: ({ row }) => (
      <LongText className='max-w-80'>{row.getValue('email')}</LongText>
    ),
    meta: { className: 'w-80' },
    enableHiding: true,
    enableSorting: false
  },
]
