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


import { IconLogout } from '@tabler/icons-react'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { useTranslation } from 'react-i18next'

interface Props {
    open: boolean
    onOpenChange: (open: boolean) => void
    handleConfirm: () => void
}

export function LogoutConfirmDialog({ open, onOpenChange, handleConfirm }: Props) {
    const { t } = useTranslation()
    const handleLogout = () => {
        handleConfirm();
        onOpenChange(false);
    };

    return (
        <ConfirmDialog
            open={open}
            onOpenChange={onOpenChange}
            handleConfirm={handleLogout}
            className="max-w-md"
            title={
                <span className='text-destructive'>
                    <IconLogout
                        className='mr-1 inline-block stroke-destructive'
                        size={18}
                    />{' '}
                    {t('auth.logout')}
                </span>
            }
            desc={
                <p>
                    {t('auth.areYouSureYouWantToLogOut')}
                    <br />
                    {t('auth.youWillNeedToLogInAgain')}
                </p>
            }
            confirmText={t('auth.logout')}
            cancelBtnText={t('common.cancel')}
        />
    )
}