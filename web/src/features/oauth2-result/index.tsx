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

import { Card, CardHeader, CardContent } from "@/components/ui/card";
import { FixedHeader } from "@/components/layout/fixed-header";
import { Main } from "@/components/layout/main";
import { useLocation } from "@tanstack/react-router";
import { AlertCircle, CheckCircle2, Info } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTranslation } from "react-i18next";

export default function OAuth2Result() {
    const { t } = useTranslation();
    const { search } = useLocation();
    const params = new URLSearchParams(search);
    const error = params.get("error");
    const message = params.get("message");
    const success = params.get("success");

    return (
        <>
            <FixedHeader />
            <Main className="flex min-h-screen flex-col items-center justify-center p-4">
                <div className="w-full max-w-5xl">
                    {error ? (
                        <Card className="shadow-lg">
                            <CardHeader>
                                <h2 className="text-2xl font-semibold text-center">
                                    {t('oauth2.authFailed')}
                                </h2>
                            </CardHeader>
                            <CardContent className="space-y-4">
                                <div className="rounded-lg p-4">
                                    <div className="flex items-start">
                                        <AlertCircle className="h-12 w-12 flex-shrink-0 mt-0.5 text-red-500" />
                                        <div className="ml-3 flex-1">
                                            <h3 className="text-sm font-medium">{t('oauth2.error')}</h3>
                                            <div className="mt-2 text-sm bg-gray-100 dark:bg-gray-800 rounded p-4">
                                                <code className="whitespace-pre-wrap text-sm font-mono break-all rounded p-2">
                                                    {message?.replace(/\\n/g, "\n").replace(/\\"/g, "") || t('oauth2.unknownError')}
                                                </code>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                                <div className="flex justify-center gap-4 pt-2">
                                    <Button variant="outline" asChild>
                                        <a href="/oauth2">{t('oauth2.tryAgain')}</a>
                                    </Button>
                                    <Button variant="link" asChild>
                                        <a href="/">{t('oauth2.goHome')}</a>
                                    </Button>
                                </div>
                            </CardContent>
                        </Card>
                    ) : success ? (
                        <Card className="shadow-lg">
                            <CardHeader>
                                <h2 className="text-2xl font-semibold text-center">
                                    {t('oauth2.authSuccess')}
                                </h2>
                            </CardHeader>
                            <CardContent className="space-y-4">
                                <div className="rounded-lg p-4">
                                    <div className="flex items-start">
                                        <CheckCircle2 className="h-12 w-12 flex-shrink-0 mt-0.5 text-green-500" />
                                        <div className="ml-3 flex-1">
                                            <h2 className="text-sm font-medium">{t('oauth2.success')}</h2>
                                            <div className="mt-2 text-sm">
                                                {t('oauth2.successMessage')}
                                            </div>
                                        </div>
                                    </div>
                                </div>
                                <div className="flex justify-center pt-2">
                                    <Button asChild>
                                        <a href="/accounts">{t('oauth2.goToAccounts')}</a>
                                    </Button>
                                </div>
                            </CardContent>
                        </Card>
                    ) : (
                        <Card className="shadow-lg">
                            <CardHeader>
                                <h2 className="text-2xl font-semibold text-center">
                                    {t('oauth2.authStatus')}
                                </h2>
                            </CardHeader>
                            <CardContent className="space-y-4">
                                <div className="rounded-lg p-4">
                                    <div className="flex items-start">
                                        <Info className="h-5 w-5 flex-shrink-0 mt-0.5" />
                                        <div className="ml-3 flex-1">
                                            <h3 className="text-sm font-medium">{t('oauth2.information')}</h3>
                                            <div className="mt-2 text-sm">
                                                {t('oauth2.noStatus')}
                                            </div>
                                        </div>
                                    </div>
                                </div>
                                <div className="flex justify-center pt-2">
                                    <Button variant="outline" asChild>
                                        <a href="/oauth2">{t('oauth2.backToLogin')}</a>
                                    </Button>
                                </div>
                            </CardContent>
                        </Card>
                    )}
                </div>
            </Main>
        </>
    );
}
