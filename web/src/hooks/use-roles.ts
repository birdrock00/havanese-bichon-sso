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


import { list_roles, RoleType, UserRole } from '@/api/users/api'
import { useQuery } from '@tanstack/react-query'

export function useRoles() {
    const query = useQuery<UserRole[]>({
        queryKey: ['role-list'],
        queryFn: list_roles,
        staleTime: 5 * 60 * 1000,
    })

    const roles = query.data ?? []

    const globalRoles = roles.filter(r => r.role_type === 'Global')
    const accountRoles = roles.filter(r => r.role_type === 'Account')

    const hasPermission = (
        roleIds: number[],
        permission: string,
        type?: RoleType
    ) => {
        if (!roleIds.length) return false

        const pool =
            type === 'Global'
                ? globalRoles
                : type === 'Account'
                    ? accountRoles
                    : roles

        return pool
            .filter(role => roleIds.includes(role.id))
            .some(role => role.permissions.includes(permission))
    }


    const global = {
        roles: globalRoles,
        hasPermission: (roleIds: number[], permission: string) =>
            hasPermission(roleIds, permission, 'Global'),
        isAdmin: (roleIds: number[]) =>
            hasPermission(roleIds, 'system:root', 'Global'),
    }

    const account = {
        roles: accountRoles,
        hasPermission: (roleIds: number[], permission: string) =>
            hasPermission(roleIds, permission, 'Account'),
    }

    return {
        ...query,
        roles,
        globalRoles,
        accountRoles,
        global,
        account,
        hasPermission,
    }
}
