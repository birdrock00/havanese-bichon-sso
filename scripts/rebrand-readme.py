#!/usr/bin/env python3
"""Reapplies this fork's README branding on top of whatever upstream ships.

Run after every upstream merge (see .github/workflows/sync-upstream.yaml) so
the fork notice and self-referential links (stars/release/docker badges)
survive README changes pulled in from rustmailer/bichon. The one thing this
deliberately does NOT touch is the upstream attribution link inside the fork
notice itself -- that must keep pointing at the real rustmailer/bichon repo.
"""
import re
import sys

README_PATH = "README.md"
UPSTREAM = "rustmailer/bichon"
FORK = "birdrock00/havanese-bichon-sso"
OIDC_PR_URL = "https://github.com/rustmailer/bichon/pull/328"

FORK_NOTICE = f"""<!-- FORK-NOTICE:START -->
> **This is [{FORK}](https://github.com/{FORK})**, a fork of [{UPSTREAM}](https://github.com/{UPSTREAM}) with OpenID Connect single sign-on added.
> SSO support was originally proposed upstream in [{UPSTREAM}#328]({OIDC_PR_URL}) but closed without being merged; this fork carries it going forward.
> A GitHub Action syncs every other feature and fix from upstream on a biweekly schedule -- see [.github/workflows/sync-upstream.yaml](.github/workflows/sync-upstream.yaml).
<!-- FORK-NOTICE:END -->
"""

# Self-referential links (this repo's own stars/releases/actions/docker image)
# get repointed at the fork. Anything that's genuinely about the upstream
# project itself (e.g. the fork notice above) is inserted after this pass so
# it can't be caught by it.
SELF_REFERENTIAL_GITHUB = re.compile(
    r"github\.com/rustmailer/bichon(?!#328)(/(?:stargazers|releases|actions|issues)?)"
)

# shields.io badge *images* that read live stats off the rustmailer/bichon
# repo (stars, release version, etc) -- these should reflect this fork's own
# repo, not upstream's.
SHIELDS_GITHUB_STAT = re.compile(
    r"(img\.shields\.io/github/[a-zA-Z0-9_/-]+?)/rustmailer/bichon\b"
)

DOCKER_PULLS_BADGE = re.compile(
    r'<a href="https://hub\.docker\.com/r/rustmailer/bichon">\s*'
    r'<img src="https://img\.shields\.io/docker/pulls/rustmailer/bichon\?[^"]*" alt="Docker Pulls">\s*'
    r"</a>",
    re.DOTALL,
)
DOCKER_PULLS_REPLACEMENT = (
    f'<a href="https://github.com/{FORK}/pkgs/container/havanese-bichon-sso">'
    f'<img src="https://img.shields.io/badge/ghcr.io-{FORK.replace("/", "%2F").replace("-", "--")}-2496ED?style=for-the-badge" alt="GHCR Image">'
    f"</a>"
)

DOCKER_VERSION_BADGE = re.compile(
    r'<a href="https://hub\.docker\.com/r/rustmailer/bichon">\s*'
    r'<img src="https://img\.shields\.io/docker/v/rustmailer/bichon\?[^"]*" alt="Docker">\s*'
    r"</a>",
    re.DOTALL,
)
DOCKER_VERSION_REPLACEMENT = (
    f'<a href="https://github.com/{FORK}/pkgs/container/havanese-bichon-sso">'
    f'<img src="https://img.shields.io/badge/docker-ghcr.io-2496ED?style=for-the-badge" alt="Docker">'
    f"</a>"
)

GHCR_IMAGE = f"ghcr.io/{FORK}"

# docker pull/run/compose examples in the Quick Start section: point at this
# fork's GHCR image instead of upstream's Docker Hub image, so the copy-paste
# commands actually run this fork.
DOCKER_IMAGE_REF = re.compile(r"rustmailer/bichon:latest")

# Source clone instructions (Development / Contributing sections) should
# clone this fork, not upstream -- and the directory `cd` after cloning
# changes to match the fork's repo name.
GIT_CLONE_URL = re.compile(r"https://github\.com/rustmailer/bichon\.git")
CD_AFTER_CLONE = re.compile(r"^cd bichon$", re.MULTILINE)

# Product-name prose: real technical identifiers in this doc (BICHON_* env
# vars, bichon-cli/bichon-admin/bichon-blob/bichon-data/bichon-indices/
# bichon-storage, admin@bichon, --bichon-* flags, ./bichon binaries) are all
# either ALL_CAPS or fully-lowercase-hyphenated, so this exact-case whole-word
# match on "Bichon" only ever hits prose/title mentions of the product name,
# never those identifiers.
# Both patterns exclude matches already adjacent to the replacement text
# (HAVANESE-/-SSO, havanese-/-sso) so re-running this script -- which happens
# on every sync, whether or not upstream's README actually changed -- can
# never compound HAVANESE-BICHON-SSO into HAVANESE-HAVANESE-BICHON-SSO-SSO.
PRODUCT_NAME_TITLE = re.compile(r"(?<!HAVANESE-)\bBICHON\b(?!-SSO)")
# Also excludes "Bichon" when it's part of a kebab-case URL slug (e.g. the
# wiki link ".../wiki/Bichon-v2.x-Migration-Guide") rather than standalone
# prose.
PRODUCT_NAME_PROSE = re.compile(r"(?<![-/])(?<!havanese-)\bBichon\b(?!-sso)(?!-)")
NEW_NAME_TITLE = "HAVANESE-BICHON-SSO"
NEW_NAME_PROSE = "havanese-bichon-sso"


def strip_existing_notice(text: str) -> str:
    return re.sub(
        r"<!-- FORK-NOTICE:START -->.*?<!-- FORK-NOTICE:END -->\n*",
        "",
        text,
        flags=re.DOTALL,
    )


def main() -> int:
    with open(README_PATH, encoding="utf-8") as f:
        text = f.read()

    text = strip_existing_notice(text)
    text = DOCKER_PULLS_BADGE.sub(DOCKER_PULLS_REPLACEMENT, text)
    text = DOCKER_VERSION_BADGE.sub(DOCKER_VERSION_REPLACEMENT, text)
    text = SELF_REFERENTIAL_GITHUB.sub(lambda m: f"github.com/{FORK}{m.group(1)}", text)
    text = SHIELDS_GITHUB_STAT.sub(lambda m: f"{m.group(1)}/{FORK}", text)
    text = DOCKER_IMAGE_REF.sub(f"{GHCR_IMAGE}:latest", text)
    text = GIT_CLONE_URL.sub(f"https://github.com/{FORK}.git", text)
    text = CD_AFTER_CLONE.sub("cd havanese-bichon-sso", text)
    text = PRODUCT_NAME_TITLE.sub(NEW_NAME_TITLE, text)
    text = PRODUCT_NAME_PROSE.sub(NEW_NAME_PROSE, text)

    # Insert the notice right after the title heading, or at the top if the
    # heading isn't found (upstream restructured the README).
    heading = re.search(r"</H1>\n", text, re.IGNORECASE)
    if heading:
        insert_at = heading.end()
        # Collapse any blank lines upstream (or a prior run) left between the
        # heading and the next content, so the notice always sits on exactly
        # one blank line below the title.
        rest = text[insert_at:].lstrip("\n")
        text = text[:insert_at] + "\n" + FORK_NOTICE + "\n" + rest
    else:
        text = FORK_NOTICE + "\n" + text

    with open(README_PATH, "w", encoding="utf-8") as f:
        f.write(text)

    return 0


if __name__ == "__main__":
    sys.exit(main())
