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




import { User } from '@/api/users/api'
import React from 'react'

export type UserDialogType = 'add' | 'edit' | 'delete' | 'api-tokens'

interface UserContextType {
  open: UserDialogType | null
  setOpen: (str: UserDialogType | null) => void
  currentRow: User | null
  setCurrentRow: React.Dispatch<React.SetStateAction<User | null>>
}

const UserContext = React.createContext<UserContextType | null>(null)

interface Props {
  children: React.ReactNode
  value: UserContextType
}

export default function UserProvider({ children, value }: Props) {
  return <UserContext.Provider value={value}>{children}</UserContext.Provider>
}

export const useUserContext = () => {
  const userContext = React.useContext(UserContext)

  if (!userContext) {
    throw new Error(
      'useUserContext has to be used within <UserContext.Provider>'
    )
  }

  return userContext
}
