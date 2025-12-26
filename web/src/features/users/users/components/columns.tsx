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
import { DataTableColumnHeader } from './data-table-column-header'
import { DataTableRowActions } from './data-table-row-actions'
import { format } from 'date-fns'
import { Badge } from '@/components/ui/badge'
import { User, UserRole } from '@/api/users/api'
import { LockIcon } from 'lucide-react'

export const getColumns = (t: (key: string) => string, roles: UserRole[]): ColumnDef<User>[] => [
  {
    accessorKey: 'id',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('users.columns.id')} className="justify-center" />
    ),
    cell: ({ row }) => (
      <LongText className='max-w-18 text-center'>{`${row.original.id}`}</LongText>
    ),
    enableHiding: false,
    meta: { className: 'w-18' },
    enableSorting: false
  },
  {
    accessorKey: "avatar",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('users.columns.avatar')} className="justify-center" />
    ),
    cell: ({ row }) => {
      if (row.original.avatar) {
        let avatarSrc = `data:image/png;base64,${row.original.avatar}`;
        return <div className="flex justify-center">
          <img
            src={avatarSrc}
            alt="Avatar"
            className="h-8 w-8 rounded-full object-cover"
          />
        </div>
      } else {
        return <LongText>{t('common.na')}</LongText>
      }
    },
    meta: { className: 'w-18 text-center' },
  },
  {
    accessorKey: "username",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('users.columns.username')} className="justify-center" />
    ),
    cell: ({ row }) => {
      return <LongText>{row.original.username}</LongText>
    },
    meta: { className: 'text-center' },
  },
  {
    accessorKey: "email",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('users.columns.email')} className="justify-center" />
    ),
    cell: ({ row }) => {
      return <LongText>{row.original.email}</LongText>
    },
    meta: { className: 'text-center' },
  },
  {
    id: 'roles',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('users.columns.roles')} className="justify-center" />
    ),
    cell: ({ row }) => {
      const user = row.original
      const userRoleIds = user.global_roles || []

      const mapped = userRoleIds
        .map((rid) => roles.find((r) => r.id === rid))
        .filter(Boolean) as UserRole[]

      if (mapped.length === 0) {
        return <span className="text-muted-foreground">-</span>
      }

      return (
        <div className="flex flex-wrap gap-1 justify-center">
          {mapped.map((role) => (
            <Badge key={role.id} variant="outline">
              {role.name}
              {user.id === 100000000000000 && <LockIcon className="ml-1 h-3 w-3 text-muted-foreground" />}
            </Badge>
          ))}
        </div>
      )
    },
    meta: { className: 'w-36 text-center' },
    enableHiding: false,
  },
  {
    accessorKey: 'created_at',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('users.columns.created_at')} className="justify-center" />
    ),
    cell: ({ row }) => {
      const created_at = row.original.created_at;
      const date = format(new Date(created_at), 'yyyy-MM-dd HH:mm:ss');
      return <LongText>{date}</LongText>;
    },
    meta: { className: 'text-center' },
    enableHiding: false,
  },
  {
    accessorKey: 'updated_at',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('users.columns.updated_at')} className="justify-center" />
    ),
    cell: ({ row }) => {
      const updated_at = row.original.updated_at;
      const date = format(new Date(updated_at), 'yyyy-MM-dd HH:mm:ss');
      return <LongText>{date}</LongText>;
    },
    meta: { className: 'text-center' },
    enableHiding: false,
  },
  {
    id: 'actions',
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title={t('users.columns.actions')} className="justify-center" />
    ),
    cell: DataTableRowActions,
  },
]
