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


import { Mail } from "./components/mail"
import { Main } from "@/components/layout/main"
import { FixedHeader } from "@/components/layout/fixed-header"

export default function Mailboxes() {
    const layout = localStorage.getItem("react-resizable-panels:layout:mail")
    const collapsed = localStorage.getItem("react-resizable-panels:collapsed")

    const defaultLayout = layout ? JSON.parse(layout) : undefined
    const defaultCollapsed = collapsed ? JSON.parse(collapsed) : undefined

    const lastSelectedAccountId = localStorage.getItem('mailbox:selectedAccountId') ?? undefined

    return (
        <>
            <FixedHeader />
            <Main>
                <Mail
                    defaultLayout={defaultLayout}
                    defaultCollapsed={defaultCollapsed}
                    lastSelectedAccountId={lastSelectedAccountId ? parseInt(lastSelectedAccountId) : undefined}
                    navCollapsedSize={2}
                />
            </Main>
        </>
    )
}