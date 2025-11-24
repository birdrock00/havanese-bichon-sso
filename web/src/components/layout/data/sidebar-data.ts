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


import {
  IconHelp,
  IconLayoutDashboard,
  IconLockAccess,
  IconSettings
} from '@tabler/icons-react'
import { IdCard, Inbox, Mailbox, Search } from 'lucide-react'
import { type SidebarData } from '../types'
import { useTranslation } from 'react-i18next'

export function useSidebarData(): SidebarData {
  const { t } = useTranslation()
  
  return {
    navGroups: [
      {
        title: t('navigation.general'),
        items: [
          {
            title: t('navigation.dashboard'),
            url: '/',
            icon: IconLayoutDashboard,
          }
        ],
      },
      {
        title: t('navigation.accounts'),
        items: [
          {
            title: t('navigation.accounts'),
            url: '/accounts',
            icon: Inbox,
          },
          {
            title: t('navigation.mailbox'),
            url: '/mailboxes',
            icon: Mailbox,
          },
          {
            title: t('common.search'),
            url: '/search',
            icon: Search,
          }
        ],
      },
      {
        title: t('navigation.auth'),
        items: [
          {
            title: t('navigation.oauth2'),
            url: '/oauth2',
            icon: IdCard,
          },
          {
            title: t('navigation.accessTokens'),
            url: '/access-tokens',
            icon: IconLockAccess,
          }
        ]
      },
      {
        title: t('navigation.other'),
        items: [
          {
            title: t('navigation.settings'),
            url: '/settings',
            icon: IconSettings,
          },
          {
            title: t('navigation.apiDocs'),
            url: '/api-docs',
            icon: IconHelp,
          },
        ],
      },
    ],
  }
}
