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
import { PaginatedResponse } from "..";

export interface MinimalAccount {
    id: number;
    email: string;
}

export const minimal_account_list = async () => {
    const response = await axiosInstance.get<MinimalAccount[]>("/api/v1/minimal-account-list");
    return response.data;
};


export interface ErrorMessage {
    error: string;
    at: number; // milliseconds timestamp
}

export type ProgressMap = Record<string, MailboxBatchProgress>;

export interface AccountRunningState {
    account_id: number;
    last_incremental_sync_start: number;
    last_incremental_sync_end?: number,
    errors: ErrorMessage[];
    is_initial_sync_completed: boolean;
    progress?: ProgressMap;
    initial_sync_start_time?: number;
    initial_sync_end_time?: number;
}


export interface MailboxBatchProgress {
    total_batches: number;
    current_batch: number;
}



type Encryption = 'Ssl' | 'StartTls' | 'None';
type AuthType = 'Password' | 'OAuth2';
type Unit = 'Days' | 'Months' | 'Years';
type AccountType = 'IMAP' | 'NoSync';
// Interface definitions
interface AuthConfig {
    auth_type: AuthType;
    password?: string;
}

export interface ImapConfig {
    host: string;
    port: number; // integer, 0-65535
    encryption: Encryption;
    auth: AuthConfig;
    use_proxy?: number;
}

interface RelativeDate {
    unit: Unit;
    value: number; // integer, minimum 1
}

interface DateSelection {
    fixed?: string; // format: "YYYY-MM-DD"
    relative?: RelativeDate;
}

export interface AccountModel {
    id: number;
    account_type: AccountType;
    imap?: ImapConfig;
    enabled: boolean;
    name?: string,
    email: string;
    capabilities?: string[];
    date_since?: DateSelection;
    date_before?: RelativeDate;
    folder_limit?: number,
    sync_folders: string[];
    sync_interval_min?: number;
    sync_batch_size?: number;
    created_by: number;
    created_user_name: string;
    created_user_email: string;
    created_at: number;
    updated_at: number;
    use_proxy?: number
    use_dangerous: boolean
}

export const account_state = async (account_id: number) => {
    const response = await axiosInstance.get<AccountRunningState>(`/api/v1/account-state/${account_id}`);
    return response.data;
};

export const create_account = async (data: Record<string, any>) => {
    const response = await axiosInstance.post("/api/v1/account", data);
    return response.data;
};

export const list_accounts = async () => {
    const response = await axiosInstance.get<PaginatedResponse<AccountModel>>("/api/v1/accounts?desc=true");
    return response.data;
};

export const update_account = async (account_id: number, data: Record<string, any>) => {
    const response = await axiosInstance.post(`/api/v1/account/${account_id}`, data);
    return response.data;
};

export const remove_account = async (account_id: number) => {
    const response = await axiosInstance.delete(`/api/v1/account/${account_id}`);
    return response.data;
};

export interface AutoConfigResult {
    imap: ServerConfig;
    oauth2?: OAuth2Config;
}

export interface ServerConfig {
    host: string;
    port: number;
    encryption: 'None' | 'Ssl' | 'StartTls';
}

export interface OAuth2Config {
    issuer: string;
    scope: string;
    auth_url: string;
    token_url: string;
}

export const autoconfig = async (email: string) => {
    const response = await axiosInstance.get<AutoConfigResult>(`/api/v1/autoconfig/${email}`);
    return response.data;
};

export const access_assign = async (data: Record<string, any>) => {
    const response = await axiosInstance.post("/api/v1/accounts/access/assignments", data);
    return response.data;
};