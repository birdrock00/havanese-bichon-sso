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
import { OAuth2Action } from './oauth2-action'
import { RunningStateCellAction } from './running-state-action'
import { EnableAction } from './enable-action'
import { useTranslation } from 'react-i18next'
import { AccountModel } from '@/api/account/api'

export function useColumns(): ColumnDef<AccountModel>[] {
  const { t } = useTranslation()

  return [
    {
      accessorKey: "id",
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('accounts.id')} />
      ),
      cell: ({ row }) => {
        return <LongText>{row.original.id}</LongText>
      },
      enableSorting: false,
      enableHiding: false,
    },
    {
      accessorKey: "email",
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('accounts.email')} className="justify-center" />
      ),
      cell: ({ row }) => {
        return <LongText>{row.original.email}</LongText>
      },
      enableHiding: false,
    },
    {
      accessorKey: "enabled",
      header: ({ column }) => (
        <DataTableColumnHeader className="justify-center" column={column} title={t('accounts.enabled')} />
      ),
      cell: EnableAction,
      meta: { className: 'w-18 text-center' },
      enableHiding: false,
    },
    {
      id: 'auth_type',
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('accounts.auth')} />
      ),
      cell: OAuth2Action,
      meta: { className: 'text-center' },
      enableHiding: false,
      enableSorting: false
    },
    {
      id: 'account_type',
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('accounts.type')} />
      ),
      cell: ({ row }) => {
        return <LongText>{row.original.account_type}</LongText>
      },
      meta: { className: 'w-18' },
      enableHiding: false,
      enableSorting: false
    },
    {
      accessorKey: "sync_interval_sec",
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('accounts.incSync')} className="justify-center" />
      ),
      cell: ({ row }) => {
        let account_type = row.original.account_type;
        if (account_type === "NoSync") {
          return <LongText className="text-center">n/a</LongText>
        }
        return <LongText className="text-center">{row.original.sync_interval_min} min</LongText>
      },
      //meta: { className: 'w-18 text-center' },
      enableHiding: false,
    },
    {
      id: 'running_state',
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('accounts.state')} />
      ),
      cell: RunningStateCellAction,
      meta: { className: 'text-center' },
      enableHiding: false,
    },
    {
      accessorKey: 'created_by',
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('accounts.owner')} className="justify-center" />
      ),
      cell: ({ row }) => {
        const { created_user_name, created_user_email } = row.original;
        return (
          <div className="flex flex-col items-center leading-[1.1]">
            <span className="text-[13px] font-medium text-foreground leading-none">
              {created_user_name}
            </span>
            <span className="text-[11px] text-muted-foreground font-mono leading-none">
              {created_user_email}
            </span>
          </div>
        );
      },
      meta: { className: 'w-60 text-center' },
      enableHiding: false,
    },
    {
      accessorKey: 'created_at',
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('accounts.createdAt')} />
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
        <DataTableColumnHeader column={column} title={t('accounts.updatedAt')} />
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
}
