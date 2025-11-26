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
  folder_limit?: number,
  sync_folders: string[];
  sync_interval_min?: number;
  created_at: number;
  updated_at: number;
  use_proxy?: number
  use_dangerous: boolean
}