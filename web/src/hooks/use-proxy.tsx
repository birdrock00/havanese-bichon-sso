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


import { list_proxy } from "@/api/system/api";
import { useQuery } from "@tanstack/react-query";

const useProxyList = () => {
    const { data: proxyList, ...rest } = useQuery({
        queryKey: ['proxy-list'],
        queryFn: list_proxy,
        staleTime: 10 * 60 * 1000
    });

    const proxyOptions = proxyList
        ? proxyList.map(proxy => ({
            label: proxy.url,
            value: `${proxy.id}`,
        }))
        : [];


    const getUrlById = (id: string | number) => {
        if (!proxyList) return null;
        const proxy = proxyList.find(a => `${a.id}` === `${id}`);
        return proxy?.url || null;
    };

    return {
        proxyOptions,
        proxyList,
        getUrlById,
        ...rest
    };
};

export default useProxyList;