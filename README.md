<p align="center">
    <img width="200" height="175" alt="havanese-bichon-sso Logo" src="https://github.com/user-attachments/assets/06dc3b67-7d55-4a93-a3de-8b90951c575b" />
</p>

<H1 align="center">HAVANESE-BICHON-SSO</H1>

<!-- FORK-NOTICE:START -->
> **This is [birdrock00/havanese-bichon-sso](https://github.com/birdrock00/havanese-bichon-sso)**, a fork of [rustmailer/bichon](https://github.com/rustmailer/bichon) with OpenID Connect single sign-on added.
> SSO support was originally proposed upstream in [rustmailer/bichon#328](https://github.com/rustmailer/bichon/pull/328) but closed without being merged; this fork carries it going forward.
> A GitHub Action syncs every other feature and fix from upstream on a biweekly schedule -- see [.github/workflows/sync-upstream.yaml](.github/workflows/sync-upstream.yaml).
<!-- FORK-NOTICE:END -->

<p align="center">
  <a href="https://github.com/birdrock00/havanese-bichon-sso/stargazers">
    <img src="https://img.shields.io/github/stars/birdrock00/havanese-bichon-sso?style=for-the-badge&color=gold&label=STARS" alt="GitHub Stars">
  </a>
  <a href="https://github.com/birdrock00/havanese-bichon-sso/pkgs/container/havanese-bichon-sso"><img src="https://img.shields.io/badge/ghcr.io-birdrock00%2Fhavanese--bichon--sso-2496ED?style=for-the-badge" alt="GHCR Image"></a>
  <a href="https://bichon.rustmailer.com">
    <img src="https://img.shields.io/badge/WEBSITE-bichon.rustmailer.com-blue?style=for-the-badge" alt="Website">
  </a>
  <a href="https://docs.google.com/forms/d/e/1FAIpQLScOlwsiUMfyQPBCLW2MLkygdRmAutEgvXDYPzzvEGPz0HFPXQ/viewform">
    <img src="https://img.shields.io/badge/Roadmap-2026_Survey-blue?style=for-the-badge&logo=googleforms" alt="User Survey">
  </a>
</p>

<p align="center">
  <a href="https://github.com/birdrock00/havanese-bichon-sso/releases">
    <img src="https://img.shields.io/github/v/release/birdrock00/havanese-bichon-sso" alt="Release">
  </a>
  <a href="https://github.com/birdrock00/havanese-bichon-sso/pkgs/container/havanese-bichon-sso"><img src="https://img.shields.io/badge/docker-ghcr.io-2496ED?style=for-the-badge" alt="Docker"></a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-AGPLv3-blue.svg" alt="License">
  </a>
  <a href="https://discord.gg/Bq4M2cDmF4">
    <img src="https://img.shields.io/badge/Discord-Join%20Server-7289DA?logo=discord&logoColor=white" alt="Discord">
  </a>
  <a href="https://x.com/rustmailer">
    <img src="https://img.shields.io/twitter/follow/rustmailer?style=social" alt="Follow on X">
  </a>
</p>

<p align="center">A self-hosted email archiving server built in Rust. Download emails from IMAP accounts, builds a full-text search index, and serves a REST API with an embedded WebUI. Purpose-built for long-term preservation, unified cross-account search, and programmatic access to archived email.</p>

<p align="center">
  <a href="https://www.youtube.com/watch?v=fMlayXo3Bo0">
    <img src="https://img.youtube.com/vi/fMlayXo3Bo0/maxresdefault.jpg" alt="Watch the demo"/>
  </a>
  <br/>
  <em>▶ Click to watch the demo</em>
</p>

> [!NOTE]
> havanese-bichon-sso is an **archiver**, not an email client. It does not send, compose, forward, or reply to emails. Its optional SMTP server is for **receiving** emails only.

<p align="center">
  <a href="https://bichon.rustmailer.com"><strong>Website</strong></a> ·
  <a href="https://bichon.rustmailer.com/docs"><strong>Documentation</strong></a> ·
  <a href="https://bichon.rustmailer.com/docs/quick-start"><strong>Quick Start</strong></a> ·
  <a href="https://bichon.rustmailer.com/docs/api-reference"><strong>API Reference</strong></a>
</p>

## Features

- **IMAP archiving** — multiple accounts with password or OAuth 2.0 (XOAUTH2) login, incremental UID sync, gap-fill repair, UIDVALIDITY rebuilds, per-account schedules, archive scoping and SOCKS5 routing.
- **Full-text search** — search subject, body, senders, recipients and attachment metadata; filter by date, size, folder and tags; reconstruct threads; browse an aggregated contact list.
- **Attachment search** — attachment metadata in the Community edition; full attachment **content** indexing on Pro.
- **Embedded storage** — Tantivy for search, bichon-blob for content-addressed blobs and memdb for metadata. Zstd compression, BLAKE3 deduplication, zero external dependencies.
- **Import & export** — EML, MBOX (including the Gmail variant), Thunderbird profiles and Outlook PST; MBOX export; WebUI upload; bulk restore back to the source IMAP server.
- **SMTP receiver** — receive mail at delivery time into local accounts, including mail deleted from the source before the first sync. Tenant-wide journaling is Pro/Enterprise.
- **Message lifecycle** — tags, remote-content blocking, message repair, source-mailbox cleanup, retention and background batch deletion.
- **Backup & restore** — incremental, content-addressed backups to S3-compatible storage or a local directory, with per-restore-point manifests and retention.
- **Access control** — RBAC with custom roles, account-level isolation, MFA/TOTP and scoped API tokens.
- **Compliance** — audit trail, legal hold, retention locks, GDPR erasure and dual control (Pro/Enterprise).
- **Interfaces** — WebUI in 19 languages, OpenAPI 3.0 at `/api-docs`, plus `bichon-cli` and `bichon-admin`.
- **Deployment** — a single Rust binary or Docker image; no external database, search engine or message queue.

> **Community vs Pro/Enterprise:** attachment content search, SSO, journaling, the built-in IMAP server, analytics and the compliance workflows are commercial features. See [Editions](https://bichon.rustmailer.com/docs/editions) for the full comparison.

## Quick Start

```bash
# Pull the image
docker pull ghcr.io/birdrock00/havanese-bichon-sso:latest

mkdir -p ./bichon-data

docker run -d \
  --name bichon \
  -p 15630:15630 \
  -v $(pwd)/bichon-data:/data \
  --user 1000:1000 \
  -e BICHON_ROOT_DIR=/data \
  -e BICHON_ENCRYPT_PASSWORD=your-secure-password-here \
  ghcr.io/birdrock00/havanese-bichon-sso:latest
```

Open **[http://localhost:15630](http://localhost:15630)**.

> [!IMPORTANT]
> Default login: username `admin`, password `admin@bichon`. **Change this immediately** via Settings → Profile.

Docker Compose, binary installation, source builds and the full setup guide are in **[Quick Start](https://bichon.rustmailer.com/docs/quick-start)**.

## Documentation

Full documentation lives at **[bichon.rustmailer.com/docs](https://bichon.rustmailer.com/docs)**.

| Topic | Guides |
|---|---|
| Install & configure | [Quick Start](https://bichon.rustmailer.com/docs/quick-start) · [Configuration](https://bichon.rustmailer.com/docs/configuration) |
| Accounts & sync | [Account Setup](https://bichon.rustmailer.com/docs/account-setup) · [Accounts](https://bichon.rustmailer.com/docs/accounts) · [Archive Filters](https://bichon.rustmailer.com/docs/archive-filters) · [Source Cleanup](https://bichon.rustmailer.com/docs/source-cleanup) |
| Search & messages | [Search](https://bichon.rustmailer.com/docs/search) · [Batch Deletion](https://bichon.rustmailer.com/docs/batch-delete) · [Message Repair](https://bichon.rustmailer.com/docs/message-repair) |
| Users & security | [Authentication](https://bichon.rustmailer.com/docs/authentication) · [Users & Access](https://bichon.rustmailer.com/docs/users-access) |
| Import, export & CLI | [Import & Export](https://bichon.rustmailer.com/docs/import-export) · [CLI Tools](https://bichon.rustmailer.com/docs/cli-tools) · [SMTP Receiver](https://bichon.rustmailer.com/docs/smtp-receiver) |
| Operations | [Storage & Backup](https://bichon.rustmailer.com/docs/storage-backup) · [Monitoring](https://bichon.rustmailer.com/docs/monitoring) · [Migration](https://bichon.rustmailer.com/docs/migration) · [FAQ](https://bichon.rustmailer.com/docs/faq) |
| Architecture & API | [Architecture](https://bichon.rustmailer.com/docs/architecture) · [Content Deduplication](https://bichon.rustmailer.com/docs/content-deduplication) · [API Reference](https://bichon.rustmailer.com/docs/api-reference) |
| Editions & licensing | [Editions](https://bichon.rustmailer.com/docs/editions) · [Licensing](https://bichon.rustmailer.com/docs/licensing) |
| Pro & Enterprise | [Analytics](https://bichon.rustmailer.com/docs/pro/analytics) · [SSO](https://bichon.rustmailer.com/docs/pro/sso) · [Journaling](https://bichon.rustmailer.com/docs/pro/journaling) · [MCP](https://bichon.rustmailer.com/docs/pro/mcp) · [Legal Hold](https://bichon.rustmailer.com/docs/enterprise/legal-hold) · [Dual Control](https://bichon.rustmailer.com/docs/enterprise/dual-control) |
| Contributing | [Contributing](https://bichon.rustmailer.com/docs/contributing) |
## Fork-specific documentation

These features exist only in this fork, so they are not covered by
[bichon.rustmailer.com/docs](https://bichon.rustmailer.com/docs).

## OpenID Connect (OIDC) Single Sign-On

havanese-bichon-sso can delegate WebUI authentication to any OIDC provider (Authentik,
Keycloak, PocketID, Authelia, Zitadel, Dex, …) using the Authorization
Code flow with PKCE.

| Variable | Default | Description |
|----------|---------|-------------|
| `BICHON_OIDC_ENABLED` | `false` | Master switch for OIDC single sign-on |
| `BICHON_OIDC_ISSUER_URL` | — | Issuer URL. havanese-bichon-sso appends `/.well-known/openid-configuration` for discovery. Example: `https://auth.example.com/application/o/bichon/` |
| `BICHON_OIDC_CLIENT_ID` | — | OAuth2 client ID registered with the IdP |
| `BICHON_OIDC_CLIENT_SECRET` | — | OAuth2 client secret registered with the IdP |
| `BICHON_OIDC_REDIRECT_URI` | — | Redirect URI registered with the IdP. Must resolve to `<public-url>/api/auth/oidc/callback` |
| `BICHON_OIDC_DEFAULT_ROLE_ID` | `100200000000000` (Member) | Global role ID assigned to auto-provisioned OIDC users |
| `BICHON_OIDC_AUTO_REDIRECT` | `false` | When true, `/sign-in` immediately redirects to the IdP. Local login stays reachable via `/sign-in?local=1` |

**User resolution.** On each login havanese-bichon-sso looks up the user by
`(sso_provider, sso_id)` first, then by `email`, and finally auto-provisions
a new user with `BICHON_OIDC_DEFAULT_ROLE_ID`. The `sub` claim from the IdP
is stored on the user and used for subsequent logins.

**Signature verification.** The ID token is verified with HS256 using the
client secret. Only signed tokens are accepted — other algorithms are
rejected until JWKS-based asymmetric verification is added. Discovery,
issuer, audience, expiration (with 60 s skew), and nonce are validated.

**Token handoff.** After a successful callback the SPA receives a one-shot
handoff id in the URL and POSTs it to `/api/auth/oidc/handoff` to obtain the
WebUI access token in the response body. The access token itself is never
placed in the URL, so it does not leak into browser history, `Referer`
headers, or server access logs.

> [!IMPORTANT]
> Set `BICHON_OIDC_REDIRECT_URI` to the exact value you registered with the
> IdP (including scheme, host, port, and path). The IdP rejects mismatched
> callbacks. Behind a reverse proxy this must be the externally-reachable
> URL, not `http://localhost:15630`.


## Tech Stack

Rust (Tokio, Poem, Tantivy, bichon-blob, memdb) on the backend; React, TypeScript and Vite in the WebUI. See [Architecture](https://bichon.rustmailer.com/docs/architecture) for the design.

## Roadmap

Planned work is tracked in [GitHub Issues](https://github.com/rustmailer/bichon/issues). Priorities come from the annual [user survey](https://docs.google.com/forms/d/e/1FAIpQLScOlwsiUMfyQPBCLW2MLkygdRmAutEgvXDYPzzvEGPz0HFPXQ/viewform).

## Contributing

Contributions of all kinds are welcome. Read the [Contributing guide](https://bichon.rustmailer.com/docs/contributing) first; by submitting a Pull Request you agree to the terms of the [Contributor License Agreement](CLA.md).

- Open an issue before starting a new feature or a large fix.
- Keep each PR to one logical change, and include tests for new or fixed behavior.
- Join the [Discord](https://discord.gg/Bq4M2cDmF4) for questions and design discussions.

## License

havanese-bichon-sso is licensed under the [GNU Affero General Public License v3.0](LICENSE).
Copyright &copy; 2025–2026 [rustmailer.com](https://rustmailer.com)

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=birdrock00/havanese-bichon-sso&type=Date)](https://star-history.com/#birdrock00/havanese-bichon-sso&Date)

## Contributors

Thanks to everyone who has contributed to Bichon.

[![Contributors](https://contrib.rocks/image?repo=birdrock00/havanese-bichon-sso&max=100)](https://github.com/birdrock00/havanese-bichon-sso/graphs/contributors)
