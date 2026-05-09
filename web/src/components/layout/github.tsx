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


import React, { useEffect, useState } from "react";
import { GitHubLogoIcon } from "@radix-ui/react-icons";
import { Star } from "lucide-react";

interface GithubLinkButtonProps {
  href?: string;
  repo?: string;
  size?: number;
  title?: string;
}

export const GithubLinkButton: React.FC<GithubLinkButtonProps> = ({
  href = "https://github.com/rustmailer/bichon",
  repo = "rustmailer/bichon",
  size = 18,
  title = "View on GitHub",
}) => {
  const [stars, setStars] = useState<number | null>(null);

  useEffect(() => {
    fetch(`https://api.github.com/repos/${repo}`)
      .then(res => res.json())
      .then(data => setStars(data.stargazers_count))
      .catch(() => { });
  }, [repo]);

  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      title={title}
      className="inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 text-muted-foreground hover:text-foreground hover:bg-muted transition-colors text-xs font-medium"
    >
      <GitHubLogoIcon style={{ width: size, height: size }} />
      {stars !== null && (
        <>
          <Star className="h-3 w-3 fill-current" />
          <span>{stars >= 1000 ? `${(stars / 1000).toFixed(1)}k` : stars}</span>
        </>
      )}
    </a>
  );
};