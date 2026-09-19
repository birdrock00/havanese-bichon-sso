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


import axiosInstance from "@/api/axiosInstance";

export type SavedSearchKind = "Email" | "Attachment";

export interface SavedSearchModel {
  id: string;
  user_id: number;
  name: string;
  kind: SavedSearchKind;
  filter: Record<string, any>;
  created_at: number;
  updated_at: number;
}

export interface SavedSearchCreateRequest {
  name: string;
  kind: SavedSearchKind;
  filter: Record<string, any>;
}

export const list_saved_searches = async (kind?: SavedSearchKind) => {
  const response = await axiosInstance.get<SavedSearchModel[]>("api/v1/saved-searches", {
    params: kind ? { kind } : undefined,
  });
  return response.data;
};

export const create_saved_search = async (payload: SavedSearchCreateRequest) => {
  const response = await axiosInstance.post<SavedSearchModel>("api/v1/saved-searches", payload);
  return response.data;
};

export const rename_saved_search = async (id: string, name: string) => {
  const response = await axiosInstance.patch<SavedSearchModel>(`api/v1/saved-searches/${id}`, { name });
  return response.data;
};

export const delete_saved_search = async (id: string) => {
  await axiosInstance.delete(`api/v1/saved-searches/${id}`);
};