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

use bichon_core::common::auth::ClientContext;
use poem::Request;
use poem_mcpserver::{streamable_http, McpServer};

mod tools;
use tools::BichonMcpTools;

/// Create a Poem endpoint that handles MCP Streamable HTTP requests.
///
/// The endpoint requires authentication via `ApiGuard` middleware (applied at
/// the route level in `rest/mod.rs`). The guard stores a `ClientContext` in
/// request extensions, which the server factory reads to configure per-session
/// tool authorization.
pub fn mcp_endpoint() -> impl poem::IntoEndpoint {
    streamable_http::endpoint(|req: &Request| {
        let ctx = req
            .extensions()
            .get::<ClientContext>()
            .expect("ApiGuard middleware must provide ClientContext in request extensions")
            .clone();
        McpServer::new()
            .with_server_info("bichon-mcp", env!("CARGO_PKG_VERSION"))
            .tools(BichonMcpTools::new(ctx))
    })
}
