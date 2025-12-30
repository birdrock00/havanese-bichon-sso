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
  IconSettings
} from '@tabler/icons-react'
import { IdCard, Inbox, Mailbox, Search, Users2 } from 'lucide-react'
import { type SidebarData } from '../types'
import { useTranslation } from 'react-i18next'
import { useCurrentUser } from '@/hooks/use-current-user'

export function useSidebarData(): SidebarData {
  const { t } = useTranslation()

  const { require_any_permission } = useCurrentUser()

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
            visible: require_any_permission(['system:root', 'account:create']),
          }
        ]
      },
      {
        title: t('navigation.users'),
        items: [
          {
            title: t('navigation.users'),
            url: '/users',
            icon: Users2,
            visible: require_any_permission(['system:root', 'user:manage']),
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
