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

export type MatchType = 'contains' | 'starts_with' | 'ends_with' | 'is_exactly' | 'regex';

export interface PatternEntry {
  id: string;
  matchType: MatchType;
  value: string;
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

export function simplePatternToRegex(matchType: MatchType, value: string): string {
  switch (matchType) {
    case 'contains':
      return `.*${escapeRegex(value)}.*`;
    case 'starts_with':
      return `^${escapeRegex(value)}.*`;
    case 'ends_with':
      return `.*${escapeRegex(value)}$`;
    case 'is_exactly':
      return `^${escapeRegex(value)}$`;
    case 'regex':
      return value;
  }
}

export function isValidRegex(pattern: string): boolean {
  try {
    new RegExp(pattern);
    return true;
  } catch {
    return false;
  }
}

let _idCounter = 0;
export function newPatternId(): string {
  return `pat_${++_idCounter}_${Date.now()}`;
}
