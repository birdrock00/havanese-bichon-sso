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


import axiosInstance from "@/api/axiosInstance";


export interface MailboxData {
    attributes: { attr: string; extension: string | null }[];
    delimiter: string | null;
    exists: number;
    id: number;
    name: string;
    uid_next: number | null;
    uid_validity: number | null;
    unseen: number | null;
}

export const list_mailboxes = async (accountId: number, remote: boolean) => {
    const response = await axiosInstance.get<MailboxData[]>(`/api/v1/list-mailboxes/${accountId}?remote=${remote}`);
    return response.data;
};


export const delete_mailbox = async (accountId: number, mailboxId: string) => {
    const response = await axiosInstance.delete(`/api/v1/delete-mailbox/${accountId}/${mailboxId}`);
    return response.data;
};