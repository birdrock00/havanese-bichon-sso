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


import { EmailEnvelope, PaginatedResponse } from "@/api";
import axiosInstance from "@/api/axiosInstance";
import { saveAs } from 'file-saver';

export const get_thread_messages = async (accountId: number, thread_id: string, page: number, page_size: number) => {
    const params = new URLSearchParams({
        thread_id: String(thread_id),
        page: String(page),
        page_size: String(page_size),
    });

    const response = await axiosInstance.get<PaginatedResponse<EmailEnvelope>>(
        `api/v1/get-thread-messages/${accountId}?${params.toString()}`
    );
    return response.data;
}

export const download_attachment = async (accountId: number, id: string, attachmentFileName: string) => {
    const response = await axiosInstance.get(`api/v1/download-attachment/${accountId}/${id}?name=${attachmentFileName}`, { responseType: 'blob' });
    const blob = new Blob([response.data]);
    saveAs(blob, attachmentFileName);
};

export const download_nested_attachment = async (accountId: number, id: string, attachmentFileName: string, nestedAttachmentFileName: string) => {
    const response = await axiosInstance.get(`api/v1/download-nested-attachment/${accountId}/${id}?name=${attachmentFileName}&nested_name=${nestedAttachmentFileName}`, { responseType: 'blob' });
    const blob = new Blob([response.data]);
    saveAs(blob, nestedAttachmentFileName);
};
export interface AttachmentInfo {
    /** MIME content type of the attachment (e.g., `image/png`, `application/pdf`). */
    file_type: string;
    /** Content-ID, used for inline attachments (referenced in HTML by `cid:` URLs). */
    content_id?: string;
    /** Whether the attachment is marked as inline (true) or a regular file (false). */
    inline: boolean;
    /** Original filename of the attachment, if provided. */
    filename: string;
    /** Size of the attachment in bytes. */
    size: number;
}
export interface MessageContentResponse {
    text?: string;
    html?: string;
    attachments?: AttachmentInfo[]
}

export interface NestedMessageContentResponse {
    text?: string;
    html?: string;
    attachments?: AttachmentInfo[];
    envelope: EmailEnvelope;
}

export const getContent = (messageContent: MessageContentResponse): string | null => {
    if (messageContent.html) {
        return messageContent.html;
    } else if (messageContent.text) {
        return messageContent.text;
    }
    return null;
};

export const load_message = async (accountId: number, id: string) => {
    const response = await axiosInstance.get<MessageContentResponse>(`api/v1/message-content/${accountId}/${id}`);
    return response.data;
};

export const load_nested_message = async (accountId: number, id: string, attachmentFileName: string) => {
    const response = await axiosInstance.get<NestedMessageContentResponse>(`api/v1/nested-message-content/${accountId}/${id}?name=${attachmentFileName}`);
    return response.data;
};

export const delete_messages = async (payload: Record<number, string[]>) => {
    const response = await axiosInstance.post("api/v1/delete-messages", payload);
    return response.data;
};

export const download_message = async (accountId: number, id: string) => {
    const response = await axiosInstance.get(`api/v1/download-message/${accountId}/${id}`, { responseType: 'blob' });
    const blob = new Blob([response.data]);
    saveAs(blob, `${id}.eml`);
};

export const restore_message = async (accountId: number, envelopeIds: string[]) => {
    const response = await axiosInstance.post(`api/v1/restore-messages/${accountId}`, {
        envelope_ids: envelopeIds,
    });
    return response.data;
};