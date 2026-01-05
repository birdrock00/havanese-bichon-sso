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


import { MailboxData } from "@/api/mailbox/api";
import { TreeViewBaseItem } from '@mui/x-tree-view/models';


export type ExtendedTreeItemProps = {
    exists?: number;
    attributes?: { attr: string; extension: string | null }[],
    id: string;
    label: string;
};


export function buildTree(items: MailboxData[]): TreeViewBaseItem<ExtendedTreeItemProps>[] {
    const nodeByName = new Map<string, TreeViewBaseItem<ExtendedTreeItemProps>>();

    for (const mb of items) {
        if (!mb.name) continue;
        const delimiter = mb.delimiter ?? '/';
        const parts = mb.name.split(delimiter);
        let currentFullName = '';

        for (let i = 0; i < parts.length; i++) {
            const part = parts[i];
            currentFullName = currentFullName ? `${currentFullName}${delimiter}${part}` : part;

            if (!nodeByName.has(currentFullName)) {
                nodeByName.set(currentFullName, {
                    id: currentFullName,
                    label: part,
                    exists: mb.exists,
                    attributes: mb.attributes,
                    children: [],
                });
            }

            if (i === parts.length - 1) {
                const node = nodeByName.get(currentFullName)!;
                node.id = String(mb.id);
            }
        }
    }

    const roots: TreeViewBaseItem<ExtendedTreeItemProps>[] = [];
    for (const [fullName, node] of nodeByName.entries()) {
        const delim = mbDelimiterOrDefault(fullName, items);
        const lastDelimIndex = fullName.lastIndexOf(delim);

        if (lastDelimIndex === -1) {
            roots.push(node);
            continue;
        }

        const parentFullName = fullName.substring(0, lastDelimIndex);
        const parentNode = nodeByName.get(parentFullName);

        if (parentNode) {
            parentNode.children = parentNode.children ?? [];
            if (!parentNode.children.includes(node)) {
                parentNode.children.push(node);
            }
        } else {
            roots.push(node);
        }
    }

    const sortNodes = (nodes: TreeViewBaseItem[]) => {
        nodes.sort((a, b) => a.label.localeCompare(b.label, undefined, { numeric: true }));
        for (const n of nodes) {
            if (n.children && n.children.length) sortNodes(n.children);
        }
    };

    const uniqueRoots = Array.from(new Set(roots));
    sortNodes(uniqueRoots);
    return uniqueRoots;
}

function mbDelimiterOrDefault(fullName: string, items: MailboxData[]): string {
    const mb = items.find(it => it.name === fullName || fullName.startsWith(it.name + (it.delimiter ?? '/')));
    if (mb?.delimiter) return mb.delimiter;

    const withDelim = items.find(it => it.delimiter);
    if (withDelim?.delimiter) return withDelim.delimiter;

    return '.';
}