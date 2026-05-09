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

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { getPermissions, UserRole } from '@/api/users/api'
import { CheckCircle, XCircle } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { useTranslation } from 'react-i18next'

interface Props {
  currentRow?: UserRole
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function PermissionsDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const ownedPermissions = currentRow?.permissions
    ? Array.from(currentRow.permissions)
    : [];

  const roleType = currentRow?.role_type;

  const allCategories = {
    global: [
      {
        titleKey: "roles.categories.identity",
        keys: ["system:access", "system:root", "user:manage", "user:view", "token:manage", "account:create"]
      },
      {
        titleKey: "roles.categories.global_data",
        keys: ["account:manage:all", "data:read:all", "data:manage:all", "data:raw:download:all", "data:delete:all", "data:export:batch:all"]
      }
    ],
    account: [
      {
        titleKey: "roles.categories.account_resource",
        keys: ["account:manage", "account:read_details", "data:read", "data:manage", "data:raw:download", "data:delete", "data:export:batch", "data:import:batch", "data:smtp:ingest"]
      }
    ]
  };

  const categories = roleType === 'Global'
    ? allCategories.global
    : roleType === 'Account'
      ? allCategories.account
      : [];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-7xl w-[40vw] overflow-hidden flex flex-col max-h-[90vh]">
        <DialogHeader className="pb-4 border-b">
          <div className="flex items-center gap-3">
            <DialogTitle>
              {t('roles.details.title', { name: currentRow?.name || t('roles.details.unknown') })}
            </DialogTitle>
            <Badge variant="outline" className="capitalize text-[10px]">
              {t('roles.details.role_badge', { type: t(`roles.types.${roleType}`) })}
            </Badge>
          </div>
          <DialogDescription>
            {roleType === 'Global'
              ? t('roles.details.desc_global')
              : t('roles.details.desc_account')}
          </DialogDescription>
        </DialogHeader>
        <div className="flex-1 overflow-y-auto py-4">
          <div className="flex flex-col gap-6 px-1">
            {categories.map((cat) => (
              <div key={cat.titleKey} className="flex flex-col">
                <h3 className="text-[11px] font-bold text-muted-foreground border-l-2 border-primary pl-2 mb-3 uppercase tracking-widest">
                  {t(cat.titleKey)}
                </h3>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-1.5">
                  {cat.keys.map((key) => {
                    const item = getPermissions(t).find(p => p.value === key);
                    if (!item) return null;

                    const hasPermission = ownedPermissions.includes(item.value);

                    return (
                      <div
                        key={item.value}
                        className={cn(
                          "flex items-center gap-2.5 p-2.5 rounded-md border text-xs transition-all",
                          hasPermission
                            ? "bg-primary/5 border-primary/20 text-foreground"
                            : "bg-muted/30 border-border text-muted-foreground opacity-50"
                        )}
                      >
                        {hasPermission ? (
                          <CheckCircle className="w-4 h-4 text-primary shrink-0" />
                        ) : (
                          <XCircle className="w-4 h-4 text-muted-foreground/30 shrink-0" />
                        )}

                        <div className="flex flex-col min-w-0 flex-1">
                          <span className="font-medium text-xs leading-none truncate">
                            {item.label}
                          </span>
                          <span className="text-[10px] text-muted-foreground font-mono mt-1 truncate">
                            {item.value}
                          </span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        </div>
        <div className="flex justify-end pt-4 border-t mt-auto">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onOpenChange(false)}
          >
            {t('roles.details.close_btn')}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}