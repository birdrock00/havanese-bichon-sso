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

export interface Release {
    tag_name: string;
    published_at: string;
    body: string;
    html_url: string;
}

export interface ReleaseNotification {
    latest: Release | null;  // `latest` can be null if the release information is not available
    is_newer: boolean;
    error_message: string | null;  // New field to store error message when the request fails
}

interface Notifications {
    release: ReleaseNotification;
}

export const get_notifications = async () => {
    const response = await axiosInstance.get<Notifications>(`/api/v1/notifications`);
    return response.data;
};

export interface DashboardStats {
    account_count: number;                 // Number of accounts
    email_count: number;                   // Total number of emails
    total_size_bytes: number;              // Total size of all emails (in bytes)
    storage_usage_bytes: number;           // Actual storage used (in bytes)
    index_usage_bytes: number;             // Index storage size (in bytes)
    recent_activity: TimeBucket[];        // Email activity over recent days
    top_senders: Group[];            // Top 10 senders
    top_accounts: Group[];            // Top 10 senders
    with_attachment_count: number;         // Emails with attachments
    without_attachment_count: number;      // Emails without attachments
    top_largest_emails: LargestEmail[];    // Top 10 largest emails
    system_version: string, //The semantic version string of the currently running backend service
    commit_hash: string //Git commit hash used to build this system version
}

export interface TimeBucket {
    timestamp_ms: number;   // Timestamp in milliseconds
    count: number;  // Number of emails in this time bucket
}

export interface Group {
    key: string;         // Sender email or name
    count: number;  // Number of emails from this sender
}

export interface LargestEmail {
    subject: string;        // Email subject
    size_bytes: number;     // Email size in bytes
}

export interface Proxy {
    id: number;
    url: string;
    created_at: number;
    updated_at: number;
}

export type ServerConfigurations = {
    bichon_log_level: string
    bichon_http_port: number
    bichon_bind_ip?: string | null
    bichon_public_url: string
    bichon_cors_origins?: string[] | null
    bichon_cors_max_age: number
    bichon_ansi_logs: boolean
    bichon_log_to_file: boolean
    bichon_json_logs: boolean
    bichon_max_server_log_files: number
    bichon_encrypt_password_set: boolean
    bichon_webui_token_expiration_hours: number
    bichon_root_dir: string
    bichon_metadata_cache_size?: number | null
    bichon_envelope_cache_size?: number | null
    bichon_enable_rest_https: boolean
    bichon_http_compression_enabled: boolean
    bichon_sync_concurrency?: number | null
}

export const get_dashboard_stats = async () => {
    const response = await axiosInstance.get<DashboardStats>(`/api/v1/dashboard-stats`);
    return response.data;
};

export const list_proxy = async () => {
    const response = await axiosInstance.get<Proxy[]>(`/api/v1/list-proxy`);
    return response.data;
};

export const delete_proxy = async (id: number) => {
    const response = await axiosInstance.delete(`/api/v1/proxy/${id}`);
    return response.data;
};

export const update_proxy = async (id: number, url: string) => {
    const response = await axiosInstance.post(`/api/v1/proxy/${id}`, url, {
        headers: {
            "Content-Type": "text/plain",
        },
    });
    return response.data;
};

export const add_proxy = async (url: string) => {
    const response = await axiosInstance.post(`/api/v1/proxy`, url, {
        headers: {
            "Content-Type": "text/plain",
        },
    });
    return response.data;
};


export const get_system_configurations = async () => {
    const response = await axiosInstance.get<ServerConfigurations>(`/api/v1/system-configurations`);
    return response.data;
};