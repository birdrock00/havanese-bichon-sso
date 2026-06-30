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

import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { MatchType, PatternEntry } from "@/lib/pattern-utils";
import { isValidRegex } from "@/lib/pattern-utils";

interface PatternInputProps {
  entry: PatternEntry;
  onChange: (id: string, entry: Partial<PatternEntry>) => void;
  onRemove: (id: string) => void;
}

export function PatternInput({ entry, onChange, onRemove }: PatternInputProps) {
  const { t } = useTranslation();

  const matchTypeLabels: Record<MatchType, string> = {
    contains: t("accounts.filters.matchType.contains"),
    starts_with: t("accounts.filters.matchType.startsWith"),
    ends_with: t("accounts.filters.matchType.endsWith"),
    is_exactly: t("accounts.filters.matchType.isExactly"),
    regex: t("accounts.filters.matchType.regex"),
  };

  const regexValid = entry.matchType === 'regex' ? isValidRegex(entry.value) : true;
  const showRegexHint = entry.matchType === 'regex' && entry.value && !regexValid;

  return (
    <div className="flex items-center gap-2 group">
      <Select
        value={entry.matchType}
        onValueChange={(v) => onChange(entry.id, { matchType: v as MatchType })}
      >
        <SelectTrigger className="w-[130px] h-9 text-sm">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {Object.entries(matchTypeLabels).map(([value, label]) => (
            <SelectItem key={value} value={value}>{label}</SelectItem>
          ))}
        </SelectContent>
      </Select>
      <div className="flex-1 relative">
        <Input
          className="h-9 text-sm"
          value={entry.value}
          onChange={(e) => onChange(entry.id, { value: e.target.value })}
        />
        {showRegexHint && (
          <span className="text-[10px] text-destructive absolute -bottom-4 left-0">
            Invalid regex
          </span>
        )}
      </div>
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8 shrink-0 opacity-50 group-hover:opacity-100 transition-opacity"
        onClick={() => onRemove(entry.id)}
      >
        <X className="h-3.5 w-3.5" />
      </Button>
    </div>
  );
}
