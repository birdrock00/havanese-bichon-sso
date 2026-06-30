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

import { useFormContext } from "react-hook-form";
import { useTranslation } from "react-i18next";
import {
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
  FormControl,
  FormDescription,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { AccountFormValues } from "./schema";

interface TabGeneralProps {
  isEdit?: boolean;
}

export function TabGeneral({ isEdit }: TabGeneralProps) {
  const { t } = useTranslation();
  const { control } = useFormContext<AccountFormValues>();

  return (
    <div className="space-y-6">
      <FormField
        control={control}
        name="email"
        render={({ field }) => (
          <FormItem>
            <FormLabel>{t('accounts.email')}<span className="text-red-500 align-super text-xs">*</span></FormLabel>
            <FormControl>
              <Input {...field} disabled={isEdit} placeholder={t('accounts.emailPlaceholder')} />
            </FormControl>
            {isEdit && (
              <FormDescription>{t('accounts.emailCannotBeModified')}</FormDescription>
            )}
            <FormMessage />
          </FormItem>
        )}
      />

      <FormField
        control={control}
        name="account_name"
        render={({ field }) => (
          <FormItem>
            <FormLabel>{t('accounts.name')}</FormLabel>
            <FormControl>
              <Input {...field} value={field.value ?? ''} placeholder={t('accounts.nameDescription')} />
            </FormControl>
            <FormMessage />
          </FormItem>
        )}
      />

      <FormField
        control={control}
        name="enabled"
        render={({ field }) => (
          <FormItem className="flex flex-row items-start space-x-3 space-y-0 rounded-md border p-4">
            <FormControl>
              <Checkbox checked={field.value} onCheckedChange={field.onChange} />
            </FormControl>
            <div className="space-y-1 leading-none">
              <FormLabel>{t('accounts.enabled')}</FormLabel>
              <FormDescription>{t('accounts.enabledDescription')}</FormDescription>
            </div>
          </FormItem>
        )}
      />
    </div>
  );
}
