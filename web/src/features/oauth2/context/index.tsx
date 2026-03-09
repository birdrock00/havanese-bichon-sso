//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
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


import React from 'react'
import { OAuth2Entity } from '../data/schema'

export type OAuth2DialogType = 'add' | 'edit' | 'delete' | 'authorize'

interface OAuth2ContextType {
  open: OAuth2DialogType | null
  setOpen: (str: OAuth2DialogType | null) => void
  currentRow: OAuth2Entity | null
  setCurrentRow: React.Dispatch<React.SetStateAction<OAuth2Entity | null>>
}

const OAuth2Context = React.createContext<OAuth2ContextType | null>(null)

interface Props {
  children: React.ReactNode
  value: OAuth2ContextType
}

export default function OAuth2Provider({ children, value }: Props) {
  return <OAuth2Context.Provider value={value}>{children}</OAuth2Context.Provider>
}

export const useOAuth2Context = () => {
  const oauth2Context = React.useContext(OAuth2Context)

  if (!oauth2Context) {
    throw new Error(
      'useOAuth2Context has to be used within <OAuth2Context.Provider>'
    )
  }

  return oauth2Context
}
