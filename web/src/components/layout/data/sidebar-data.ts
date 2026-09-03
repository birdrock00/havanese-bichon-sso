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


import {
  IconHelp,
  IconLayoutDashboard,
  IconSettings
} from '@tabler/icons-react'
import { BadgeCheck, BarChart3, IdCard, Inbox, Paperclip, Search, Upload, Users2, ScrollText } from 'lucide-react'
import { type SidebarData } from '../types'
import { useTranslation } from 'react-i18next'
import { useCurrentUser } from '@/hooks/use-current-user'
import { useEdition } from '@/hooks/use-edition'

export function useSidebarData(): SidebarData {
  const { t } = useTranslation()

  const { require_any_permission } = useCurrentUser()
  const { features } = useEdition()
  const auditEnabled = features.includes('audit_log')
  const licenseEnabled = features.includes('license')
  const analyticsEnabled = features.includes('analytics')

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
            title: t('common.search'),
            url: '/search',
            icon: Search,
          },
          {
            title: t('navigation.analytics', 'Analytics'),
            url: '/analytics',
            icon: BarChart3,
            visible:
              analyticsEnabled &&
              require_any_permission([
                'system:root',
                'user:manage',
                'data:read:all',
                // Account-scoped managers / readers can analyze the accounts
                // they can read - global "manage account all" is not required.
                'data:read',
                'account:manage',
              ]),
          },
          {
            title: t('import.title', 'Import'),
            url: '/import',
            icon: Upload,
            visible: require_any_permission(['data:import:batch']),
          },
          {
            title: t('navigation.attachment'),
            url: '/attachment',
            icon: Paperclip,
          }
        ],
      },
      {
        title: t('navigation.auth'),
        items: [
          {
            title: t('navigation.oauth2'),
            url: '/oauth2',
            icon: IdCard
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
          {
            title: t('navigation.license'),
            url: '/license',
            icon: BadgeCheck,
            visible: licenseEnabled && require_any_permission(['system:root', 'user:manage']),
          },
          {
            title: t('navigation.auditLog'),
            url: '/audit-log',
            icon: ScrollText,
            visible: auditEnabled && require_any_permission(['system:root', 'user:manage', 'data:read:all']),
          },
        ],
      },
    ],
  }
}
