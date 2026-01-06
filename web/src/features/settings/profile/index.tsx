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

import { useCurrentUser } from '@/hooks/use-current-user'
import { UserProfileForm } from './profile-form'
import { Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'

export function Profile() {
  const { t } = useTranslation()
  const { data: user, isLoading, error } = useCurrentUser()


  if (isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        <Loader2 className="h-6 w-6 animate-spin" />
      </div>
    )
  }


  if (error || !user) {
    return (
      <div className="p-6 text-red-600">
        {t('settings.profile.loadError')}
      </div>
    )
  }

  return (
    <div className="w-full max-w-7xl ml-0 px-4">
      <UserProfileForm user={user!} />
    </div>
  )
}
