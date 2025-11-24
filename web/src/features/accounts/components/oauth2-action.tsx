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
import { useAccountContext } from '../context'
import { AccountModel } from '../data/schema'
import { useTranslation } from 'react-i18next'

interface DataTableRowActionsProps {
  row: Row<AccountModel>
}
export function OAuth2Action({ row }: DataTableRowActionsProps) {
  const { t } = useTranslation()
  const { setOpen, setCurrentRow } = useAccountContext()
  const mailer = row.original
  const account_type = mailer.account_type;

  if (account_type === "NoSync") {
    return <span className="text-xs text-muted-foreground">n/a</span>
  }

  const isOAuth2 = mailer.imap?.auth.auth_type === "OAuth2"

  if (isOAuth2) {
    return (
      <Button
        variant="ghost"
        size="sm"
        className="text-xs text-blue-500 hover:text-blue-700 underline"
        onClick={() => {
          setCurrentRow(mailer)
          setOpen("oauth2")
        }}
      >
        OAuth2
      </Button>
    )
  }

  return <Button variant={"ghost"} className="text-xs text-muted-foreground">{t('settings.password')}</Button>
}
