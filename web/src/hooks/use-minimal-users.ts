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


import { list_minimal_users, MinimalUser } from '@/api/users/api'
import { useQuery } from '@tanstack/react-query'

export function useMinimalUsers() {
    const query = useQuery<MinimalUser[]>({
        queryKey: ['minimal-user-list'],
        queryFn: list_minimal_users,
        staleTime: 5 * 60 * 1000,
    })

    const users = query.data ?? []
    const userMap = users.reduce((map, user) => {
        map[user.id] = user
        return map
    }, {} as Record<number, MinimalUser>)


    const getUsername = (id: number) => userMap[id]?.username ?? ''
    const getEmail = (id: number) => userMap[id]?.email ?? ''
    const getUser = (id: number) => userMap[id] ?? null
    const hasUser = (id: number) => !!userMap[id]

    return {
        ...query,
        users,
        userMap,
        getUsername,
        getEmail,
        getUser,
        hasUser,
    }
}