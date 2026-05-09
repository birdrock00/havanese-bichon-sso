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

import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { useTranslation } from 'react-i18next'
import useMinimalAccountList from '@/hooks/use-minimal-account-list'
import { useCurrentUser } from '@/hooks/use-current-user'
import { PermissionsDialog } from './permissions-dialog'
import Logo from '@/assets/logo.svg'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import LongText from '@/components/long-text'


export function AccountAccessList() {
    const { t } = useTranslation()
    const { data: user } = useCurrentUser()
    const { getEmailById } = useMinimalAccountList()

    const [permissionsOpen, setPermissionsOpen] = useState(false)
    const [permissionsAccountId, setPermissionsAccountId] = useState<number | undefined>(undefined)

    if (!user) return null

    const accessibleAccountIds = user.account_access_map instanceof Map
        ? Array.from(user.account_access_map.keys())
        : Object.keys(user.account_access_map || {}).map(Number)

    const roleSummary = user.account_roles_summary || {}

    if (accessibleAccountIds.length === 0) {
        return (
            <div className="flex h-[450px] items-center justify-center rounded-md border border-dashed mt-4">
                <div className="mx-auto flex max-w-[420px] flex-col items-center justify-center text-center px-4">
                    <img
                        src={Logo}
                        className="max-h-[100px] w-auto opacity-20 saturate-0 object-contain"
                        alt="Bichon Logo"
                    />
                    <h3 className="mt-4 text-lg font-semibold">{t('settings.access.empty.title')}</h3>
                    <p className="mt-2 text-sm text-muted-foreground">
                        {t('settings.access.empty.description')}
                    </p>
                </div>
            </div>
        )
    }

    return (
        <>
            <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                {accessibleAccountIds.map((accountId) => {
                    const email = getEmailById(accountId)
                    const roleName = roleSummary[accountId]
                    if (!email) return null

                    return (
                        <Card key={accountId} className="group hover:bg-accent/40 transition-all h-fit min-w-0">
                            <CardHeader className="flex flex-row items-center gap-3 space-y-0 pb-3">
                                <div className="flex items-center justify-center w-10 h-10 rounded-full bg-primary/10 text-primary text-sm font-bold shrink-0">
                                    {email.charAt(0).toUpperCase()}
                                </div>
                                <div className="flex flex-col min-w-0 flex-1">
                                    <CardTitle className="text-sm font-semibold">
                                        <LongText className="max-w-[248px]">{email}</LongText>
                                    </CardTitle>
                                    <CardDescription className="text-[10px] font-mono">
                                        {t('settings.profile.account.id', { id: accountId })}
                                    </CardDescription>
                                </div>
                            </CardHeader>
                            <CardContent className="flex items-center justify-between pt-3 border-t">
                                {roleName ? (
                                    <Badge variant="secondary" className="text-[11px]">
                                        {roleName}
                                    </Badge>
                                ) : (
                                    <span />
                                )}
                                <Button
                                    type="button"
                                    variant="outline"
                                    size="sm"
                                    className="text-xs h-7"
                                    onClick={() => {
                                        setPermissionsAccountId(accountId)
                                        setPermissionsOpen(true)
                                    }}
                                >
                                    {t('settings.profile.button.permissions')}
                                </Button>
                            </CardContent>
                        </Card>
                    )
                })}
            </div>

            <PermissionsDialog
                currentRow={user}
                open={permissionsOpen}
                onOpenChange={setPermissionsOpen}
                mode="account"
                accountId={permissionsAccountId}
            />
        </>
    )
}