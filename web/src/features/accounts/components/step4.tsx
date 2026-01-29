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


import { useFormContext } from "react-hook-form";
import { Account } from "./action-dialog";
import { Accordion, AccordionItem, AccordionTrigger, AccordionContent } from "@/components/ui/accordion";
import { useTranslation } from "react-i18next";

export default function Step4() {
    const { t } = useTranslation();
    const { getValues } = useFormContext<Account>();
    const summaryData = getValues();


    const sinceText = (() => {
        if (summaryData.date_since?.fixed) {
            return summaryData.date_since.fixed;
        }

        if (summaryData.date_since?.relative?.value) {
            return `${t('accounts.sinceRelativeValue', {
                value: summaryData.date_since!.relative!.value,
                unit: t(`accounts.${summaryData.date_since!.relative!.unit!.toLowerCase()}`)
            })}`;
        }

        return t('accounts.syncAll');
    })();

    const hasSince = !!summaryData.date_since;
    const hasBefore = !!summaryData.date_before?.value;

    return (
        <div className="rounded-xl">
            <Accordion type="multiple" defaultValue={['email', 'name', 'imap', 'date_since', 'folder_limit', 'sync_interval', 'sync_scope', 'sync_batch_size']}>
                <AccordionItem key="email" value="email">
                    <AccordionTrigger className="font-medium capitalize text-gray-600">{t('accounts.email')}:</AccordionTrigger>
                    <AccordionContent>{summaryData.email}</AccordionContent>
                </AccordionItem>

                <AccordionItem key="name" value="name">
                    <AccordionTrigger className="font-medium capitalize text-gray-600">{t('accounts.name')}:</AccordionTrigger>
                    <AccordionContent>{summaryData.name ?? t('accounts.notAvailable')}</AccordionContent>
                </AccordionItem>

                <AccordionItem key="imap" value="imap">
                    <AccordionTrigger className="font-medium capitalize text-gray-600">{t('accounts.imap')}:</AccordionTrigger>
                    <AccordionContent>
                        <div className="overflow-x-auto">
                            <table className="min-w-full divide-y">
                                <tbody className="divide-y">
                                    <tr>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm font-medium text-gray-600">{t('accounts.host')}:</td>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm">{summaryData.imap.host}</td>
                                    </tr>
                                    <tr>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm font-medium text-gray-600">{t('accounts.port')}:</td>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm">{summaryData.imap.port}</td>
                                    </tr>
                                    <tr>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm font-medium text-gray-600">{t('accounts.encryption')}:</td>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm">{summaryData.imap.encryption}</td>
                                    </tr>
                                    <tr>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm font-medium text-gray-600">{t('accounts.useDangerous')}:</td>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm">{`${summaryData.use_dangerous}`}</td>
                                    </tr>
                                    <tr>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm font-medium text-gray-600">{t('accounts.authType')}:</td>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm">{summaryData.imap.auth.auth_type}</td>
                                    </tr>
                                    {summaryData.imap.auth.auth_type === 'Password' && (
                                        <tr>
                                            <td className="px-6 py-2 whitespace-nowrap text-sm font-medium text-gray-600">{t('accounts.password')}:</td>
                                            <td className="px-6 py-2 whitespace-nowrap text-sm break-words">{summaryData.imap.auth.password}</td>
                                        </tr>
                                    )}
                                    <tr>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm font-medium text-gray-600">{t('accounts.useProxyField')}:</td>
                                        <td className="px-6 py-2 whitespace-nowrap text-sm">{summaryData.imap.use_proxy ? t('common.yes') : t('common.no')}</td>
                                    </tr>
                                </tbody>
                            </table>
                        </div>
                    </AccordionContent>
                </AccordionItem>

                <AccordionItem key="sync_scope" value="sync_scope">
                    <AccordionTrigger className="font-medium capitalize text-gray-600">
                        {t('accounts.syncScope')}:
                    </AccordionTrigger>

                    <AccordionContent className="space-y-3">
                        {hasSince && (
                            <div className="flex flex-col">
                                <span className="text-xs text-muted-foreground">
                                    {t('accounts.sinceFixed')}:
                                </span>
                                <span className="text-sm">{sinceText}</span>
                            </div>
                        )}

                        {hasBefore && (
                            <div className="flex flex-col border-t pt-2">
                                <span className="text-xs text-muted-foreground">
                                    {t('accounts.beforeRelative')}:
                                </span>
                                <span className="text-sm">
                                    {t('accounts.beforeRelativeValue', {
                                        value: summaryData.date_before!.value,
                                        unit: t(`accounts.${summaryData.date_before!.unit!.toLowerCase()}`)
                                    })}
                                </span>
                            </div>
                        )}

                        {!hasSince && !hasBefore && (
                            <span className="text-sm text-muted-foreground">
                                {t('accounts.syncAll')}
                            </span>
                        )}
                    </AccordionContent>
                </AccordionItem>


                <AccordionItem key="folder_limit" value="folder_limit">
                    <AccordionTrigger className="font-medium capitalize text-gray-600">{t('accounts.folderLimit')}:</AccordionTrigger>
                    <AccordionContent>{summaryData.folder_limit ?? t('accounts.notAvailable')}</AccordionContent>
                </AccordionItem>

                <AccordionItem key="sync_interval" value="sync_interval">
                    <AccordionTrigger className="font-medium capitalize text-gray-600">{t('accounts.incrementalSync')}:</AccordionTrigger>
                    <AccordionContent>{summaryData.sync_interval_min} {t('accounts.minutes')}</AccordionContent>
                </AccordionItem>

                <AccordionItem key="sync_batch_size" value="sync_batch_size">
                    <AccordionTrigger className="font-medium capitalize text-gray-600">{t('accounts.syncBatchSize')}:</AccordionTrigger>
                    <AccordionContent>{summaryData.sync_batch_size}</AccordionContent>
                </AccordionItem>
            </Accordion>
        </div>
    );
}
