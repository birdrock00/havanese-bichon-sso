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


import { Outlet } from '@tanstack/react-router'
import { Separator } from '@/components/ui/separator'
import { Main } from '@/components/layout/main'
import SidebarNav from './components/sidebar-nav'
import { ShieldEllipsis, Waypoints } from 'lucide-react'
import { FixedHeader } from '@/components/layout/fixed-header'
import { useTranslation } from 'react-i18next'

export default function Settings() {
  const { t } = useTranslation()

  const sidebarNavItems = [
    {
      title: t('settings.root', 'Root'),
      icon: <ShieldEllipsis size={18} />,
      href: '/settings/root',
    },
    {
      title: t('settings.proxy', 'Proxy'),
      icon: <Waypoints size={18} />,
      href: '/settings/proxy',
    }
  ]

  return (
    <>
      <FixedHeader />

      <Main fixed>
        <h1 className='text-2xl font-bold tracking-tight md:text-3xl'>
          {t('settings.title', 'Settings')}
        </h1>
        <Separator className='my-4 lg:my-6' />
        <div className='flex flex-1 flex-col space-y-2 md:space-y-2 overflow-hidden lg:flex-row lg:space-x-12 lg:space-y-0'>
          <aside className='top-0 lg:sticky lg:w-1/5'>
            <SidebarNav items={sidebarNavItems} />
          </aside>
          <div className='flex w-full p-1 pr-4 overflow-y-hidden'>
            <Outlet />
          </div>
        </div>
      </Main>
    </>
  )
}


