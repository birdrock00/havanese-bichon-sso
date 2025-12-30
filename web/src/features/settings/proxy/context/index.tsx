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


import React from 'react'
import { Proxy } from '@/api/system/api'

export type ProxyDialogType = 'add' | 'edit' | 'delete'

interface ProxyContextType {
  open: ProxyDialogType | null
  setOpen: (str: ProxyDialogType | null) => void
  currentRow: Proxy | null
  setCurrentRow: React.Dispatch<React.SetStateAction<Proxy | null>>
}

const ProxyContext = React.createContext<ProxyContextType | null>(null)

interface Props {
  children: React.ReactNode
  value: ProxyContextType
}

export default function ProxyProvider({ children, value }: Props) {
  return <ProxyContext.Provider value={value}>{children}</ProxyContext.Provider>
}

export const useProxyContext = () => {
  const proxyContext = React.useContext(ProxyContext)

  if (!proxyContext) {
    throw new Error(
      'useProxyContext has to be used within <ProxyContext.Provider>'
    )
  }

  return proxyContext
}
