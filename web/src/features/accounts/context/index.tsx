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


import { AccountModel } from '@/api/account/api';
import React from 'react'

export type AccountDialogType =
  | 'add-imap'
  | 'add-nosync'
  | 'edit-imap'
  | 'edit-nosync'
  | 'delete'
  | 'detail'
  | 'oauth2'
  | 'running-state'
  | 'sync-folders'
  | 'access-assign';

interface AccountContextType {
  open: AccountDialogType | null
  setOpen: (str: AccountDialogType | null) => void
  currentRow: AccountModel | null
  setCurrentRow: React.Dispatch<React.SetStateAction<AccountModel | null>>
}

const AccountContext = React.createContext<AccountContextType | null>(null)

interface Props {
  children: React.ReactNode
  value: AccountContextType
}

export default function AccountProvider({ children, value }: Props) {
  return <AccountContext.Provider value={value}>{children}</AccountContext.Provider>
}

export const useAccountContext = () => {
  const accountContext = React.useContext(AccountContext)

  if (!accountContext) {
    throw new Error(
      'useAccountContext has to be used within <AccountContext.Provider>'
    )
  }

  return accountContext
}
