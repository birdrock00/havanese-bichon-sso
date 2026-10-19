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


type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline'

/**
 * Shared by the report view and the history table so a status never renders two
 * ways. Takes `t` as a plain function rather than a `TFunction`, matching
 * `failureTypeLabel` in the integrity feature.
 *
 * Lives outside the component file so that module only exports components.
 */
export function repairStatusLabel(
    t: (key: string, defaultValue: string) => string,
    status: string
): string {
    const labels: Record<string, string> = {
        complete: t('repair_message.statusComplete', 'Nothing was missing'),
        repaired: t('repair_message.statusRepaired', 'Repaired'),
        partial: t('repair_message.statusPartial', 'Partially repaired'),
        failed: t('repair_message.statusFailed', 'Failed'),
    }
    // An unrecognised status falls through to the raw string rather than
    // rendering nothing.
    return labels[status] ?? status
}

export function repairStatusVariant(status: string): BadgeVariant {
    const variants: Record<string, BadgeVariant> = {
        complete: 'secondary',
        repaired: 'default',
        partial: 'outline',
        failed: 'destructive',
    }
    return variants[status] ?? 'outline'
}
