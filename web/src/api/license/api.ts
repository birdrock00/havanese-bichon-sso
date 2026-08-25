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

import axiosInstance from '@/api/axiosInstance'

export interface LicenseStatusResponse {
  status: string
  email?: string | null
  edition?: string | null
  updates_until?: string | null
  features?: string[] | null
  days_remaining?: number | null
  build_date?: string | null
  valid_until?: string | null
  machine_id: string
  account_limit?: number | null
  accounts_used: number
}

export interface UploadLicenseResponse {
  success: boolean
  email: string
  edition: string
  updates_until: string
}

export async function get_license_status(): Promise<LicenseStatusResponse> {
  const { data } = await axiosInstance.get<LicenseStatusResponse>('api/v1/license/status')
  return data
}

export async function upload_license(license: string): Promise<UploadLicenseResponse> {
  const { data } = await axiosInstance.post<UploadLicenseResponse>('api/v1/license/upload', {
    license,
  })
  return data
}
