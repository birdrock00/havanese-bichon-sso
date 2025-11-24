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


import { OAuth2Entity } from "@/features/oauth2/data/schema";
import axiosInstance from "../axiosInstance";

export const get_oauth2_list = async () => {
    const response = await axiosInstance.get<{ items: OAuth2Entity[] }>("/api/v1/oauth2-list");
    return response.data;
};

export const delete_oauth2 = async (id: number) => {
    const response = await axiosInstance.delete(`/api/v1/oauth2/${id}`);
    return response.data;
};

export const create_oauth2 = async (data: Record<string, any>) => {
    const response = await axiosInstance.post<OAuth2Entity>("/api/v1/oauth2", data);
    return response.data;
};

export const update_oauth2 = async (id: number, data: Record<string, any>) => {
    const response = await axiosInstance.post<OAuth2Entity>(`/api/v1/oauth2/${id}`, data);
    return response.data;
};


export const get_authorize_url = async (data: Record<string, any>) => {
    const response = await axiosInstance.post('/api/v1/oauth2-authorize-url', data);
    return response.data;
};


export interface OAuth2Tokens {
    access_token: string;
    account_id: string;
    created_at: number;
    oauth2_id: number;
    refresh_token: string;
    updated_at: number;
}

export const get_oauth2_tokens = async (accountId: number) => {
    const response = await axiosInstance.get<OAuth2Tokens>(`/api/v1/oauth2-tokens/${accountId}`);
    return response.data;
};