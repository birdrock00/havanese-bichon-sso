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


import React from 'react';
import AceEditor from 'react-ace';
import { cn } from '@/lib/utils';
import 'ace-builds/src-noconflict/mode-handlebars';
import 'ace-builds/src-noconflict/mode-json';
import 'ace-builds/src-noconflict/mode-python';
import 'ace-builds/src-noconflict/mode-markdown';
import 'ace-builds/src-noconflict/theme-kuroir';
import 'ace-builds/src-noconflict/theme-monokai';
import 'ace-builds/src-noconflict/ext-language_tools';

interface ReactAceEditorProps {
    value?: string;
    onChange?: (value: string) => void;
    placeholder?: string;
    className?: string;
    readOnly?: boolean;
    theme?: 'kuroir' | 'monokai';
    mode?: 'handlebars' | 'json' | 'markdown' | 'python';
}

const ReactAceEditor: React.FC<ReactAceEditorProps> = ({
    value,
    onChange,
    placeholder,
    className,
    readOnly = false,
    theme = 'github',
    mode = 'handlebars'
}) => {
    return (
        <div className={cn('w-full h-[100px]', className)}>
            <AceEditor
                mode={mode}
                theme={theme}
                readOnly={readOnly}
                value={value || ''}
                onChange={onChange}
                placeholder={placeholder}
                fontSize={14}
                showPrintMargin={false}
                showGutter={true}
                highlightActiveLine={true}
                width="100%"
                height="100%"
                setOptions={{
                    useWorker: false,
                    enableBasicAutocompletion: true,
                    enableMobileMenu: true,
                    enableLiveAutocompletion: false,
                    enableSnippets: false,
                    showLineNumbers: true,
                    tabSize: 2,
                }}
            />
        </div>
    );
};

export default ReactAceEditor;