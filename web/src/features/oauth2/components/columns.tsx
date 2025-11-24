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
import { Checkbox } from '@/components/ui/checkbox'
import LongText from '@/components/long-text'
import { OAuth2Entity } from '../data/schema'
import { DataTableColumnHeader } from './data-table-column-header'
import { DataTableRowActions } from './data-table-row-actions'
import { format } from 'date-fns'
import { EnableAction } from './enable-action'

export const getColumns = (t: (key: string) => string): ColumnDef<OAuth2Entity>[] => [
  {
    accessorKey: 'id',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('settings.id')} />
    ),
    cell: ({ row }) => {
      return <LongText className='max-w-[100px]'>{row.original.id}</LongText>
    },
    meta: { className: 'max-w-[100px]' },
    enableHiding: false,
    enableSorting: false,
  },
  {
    accessorKey: 'enabled',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('settings.enabled')} className='ml-4' />
    ),
    cell: EnableAction,
    meta: { className: 'w-8 text-center' },
  },
  {
    accessorKey: 'use_proxy',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('settings.useProxy')} className='ml-4' />
    ),
    cell: ({ row }) => {
      const enabled = row.original.use_proxy;
      if (enabled) {
        return <Checkbox className='max-w-8' checked disabled />
      } else {
        return <Checkbox className='max-w-8' disabled />
      }
    },
    meta: { className: 'w-8 text-center' },
  },
  {
    accessorKey: 'auth_url',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('settings.authUrl')} />
    ),
    cell: ({ row }) => {
      return <LongText className='max-w-[100px]'>{row.original.auth_url}</LongText>
    },
    meta: { className: 'max-w-[100px]' },
    enableHiding: false,
    enableSorting: false,
  },
  {
    accessorKey: 'token_url',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('settings.tokenUrl')} />
    ),
    cell: ({ row }) => {
      return <LongText className='max-w-[100px]'>{row.original.token_url}</LongText>
    },
    meta: { className: 'max-w-[100px]' },
    enableHiding: false,
    enableSorting: false,
  },
  {
    accessorKey: 'description',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('settings.description')} />
    ),
    cell: ({ row }) => (
      <LongText className='max-w-[200px]'>{row.original.description}</LongText>
    ),
    meta: { className: 'max-w-[200px]' },
    enableHiding: true,
    enableSorting: false
  },
  {
    accessorKey: 'created_at',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('settings.createdAt')} />
    ),
    cell: ({ row }) => {
      const created_at = row.original.created_at;
      const date = format(new Date(created_at), 'yyyy-MM-dd HH:mm:ss');
      return <LongText className='max-w-36'>{date}</LongText>;
    },
    meta: { className: 'w-36' },
    enableHiding: false,
  },
  {
    accessorKey: 'updated_at',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('settings.updatedAt')} />
    ),
    cell: ({ row }) => {
      const updated_at = row.original.updated_at;
      const date = format(new Date(updated_at), 'yyyy-MM-dd HH:mm:ss');
      return <LongText className='max-w-36'>{date}</LongText>;
    },
    meta: { className: 'w-36' },
    enableHiding: false,
  },
  {
    id: 'actions',
    cell: DataTableRowActions,
  },
]
