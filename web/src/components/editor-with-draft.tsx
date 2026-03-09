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


import { useEffect, useState } from 'react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { AlertTriangle } from 'lucide-react';
import AceEditor from './ace-editor';

interface CodeEditorWithDraftProps {
    value: string | undefined;
    onChange: (value: string) => void;
    localStorageKey: string;
    mode?: 'handlebars' | 'json' | 'markdown' | 'python'; 
    theme?: 'kuroir' | 'monokai';
    placeholder?: string;
    className?: string;
}

export function CodeEditorWithDraft({
    value,
    onChange,
    localStorageKey,
    mode,
    theme,
    placeholder,
    className,
}: CodeEditorWithDraftProps) {
    const [hasDraft, setHasDraft] = useState(false);

    useEffect(() => {
        const saved = localStorage.getItem(localStorageKey);
        if (saved && !value) {
            setHasDraft(true);
        }
    }, []);

    useEffect(() => {
        const save = () => {
            if (value) {
                localStorage.setItem(localStorageKey, value);
            }
        };
        const handle = setInterval(save, 2000);
        return () => {
            clearInterval(handle);
        }
    }, [value]);

    const handleRestore = () => {
        const saved = localStorage.getItem(localStorageKey);
        if (saved) {
            onChange(saved);
            setHasDraft(false);
        }
    };

    const handleDiscard = () => {
        localStorage.removeItem(localStorageKey);
        setHasDraft(false);
    };

    return (
        <div>
            {hasDraft && (
                <Alert variant="destructive" className="mt-2 flex items-start gap-2">
                    <AlertTriangle className="w-5 h-5 text-red-600 mt-1" />
                    <div>
                        <AlertTitle>Unsaved draft detected</AlertTitle>
                        <AlertDescription>
                            A draft was found in your browser. Would you like to restore it?
                            <div className="mt-2 flex gap-2">
                                <Button size="sm" onClick={handleRestore}>
                                    Restore
                                </Button>
                                <Button size="sm" variant="outline" onClick={handleDiscard}>
                                    Discard
                                </Button>
                            </div>
                        </AlertDescription>
                    </div>
                </Alert>
            )}
            <AceEditor
                placeholder={placeholder}
                value={value}
                onChange={onChange}
                className={className}
                mode={mode}
                theme={theme}
            />
        </div>
    );
}
