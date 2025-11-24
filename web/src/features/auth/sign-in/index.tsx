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


import Logo from '@/assets/logo.svg'
import { UserAuthForm } from './components/user-auth-form'
import { useTranslation } from 'react-i18next'

export default function SignIn() {
  const { t } = useTranslation()
  return (
    <div className='container relative flex h-svh flex-col items-center justify-center'>
      <div className='p-8 flex flex-col items-center'>
        <img
          src={Logo}
          className='mb-6'
          width={350}
          height={350}
          alt='Bichon Logo'
        />
        <h2 className='mb-4 text-lg font-medium text-muted-foreground'>
          {t('auth.welcome')}
        </h2>
        <div className='mx-auto flex w-full flex-col justify-center space-y-2 sm:w-[350px]'>
          <UserAuthForm />
        </div>
      </div>
    </div>
  )
}
