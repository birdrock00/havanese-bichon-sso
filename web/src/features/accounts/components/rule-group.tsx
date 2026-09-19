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
import { Checkbox } from "@/components/ui/checkbox";
import {
  FormControl,
  FormDescription,
  FormItem,
  FormLabel,
} from "@/components/ui/form";

interface RuleGroupProps {
  title: string;
  description: string;
  enabled: boolean;
  onToggle: (checked: boolean) => void;
  children?: React.ReactNode;
}

/**
 * A rule family group: master switch + (when enabled) the rule editor body.
 * Used for archive filtering and attachment extraction.
 */
export function RuleGroup({ title, description, enabled, onToggle, children }: RuleGroupProps) {
  return (
    <div className="space-y-4">
      <div className="rounded-md border p-5 space-y-2 bg-muted/30">
        <FormItem className="flex flex-row items-start space-x-3 space-y-0">
          <FormControl>
            <Checkbox checked={enabled} onCheckedChange={onToggle} />
          </FormControl>
          <div className="space-y-1 leading-none">
            <FormLabel>{title}</FormLabel>
            <FormDescription>{description}</FormDescription>
          </div>
        </FormItem>
      </div>
      {enabled && children}
    </div>
  );
}