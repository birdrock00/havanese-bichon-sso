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


import { Row } from '@tanstack/react-table'
import { Button } from '@/components/ui/button'
import { UserRole } from '@/api/users/api';
import { useRoleContext } from '../context';
import { useTranslation } from 'react-i18next';

interface Props {
  row: Row<UserRole>
}

export function PermissionsCellAction({ row }: Props) {
  const { t } = useTranslation()
  const { setOpen, setCurrentRow } = useRoleContext()

  let permissions = row.original.permissions;
  if (Array.from(permissions || []).length === 0) {
    return <span className="text-xs text-muted-foreground">*</span>
  }

  return (
    <Button variant='ghost' className="h-auto p-1" onClick={() => {
      setCurrentRow(row.original)
      setOpen('permissions')
    }}>
      <span className="text-xs text-blue-500 cursor-pointer underline hover:text-blue-700">{t('roles.details.view_permissions')}</span>
    </Button>
  )
}
