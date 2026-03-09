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


import React from "react";
import { GitHubLogoIcon } from "@radix-ui/react-icons";

interface GithubLinkButtonProps {
  /** GitHub repository or profile URL */
  href?: string;
  /** Icon size (default: 20) */
  size?: number;
  /** Optional tooltip title */
  title?: string;
}

export const GithubLinkButton: React.FC<GithubLinkButtonProps> = ({
  href = "https://github.com/rustmailer/bichon",
  size = 20,
  title = "View on GitHub",
}) => {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      title={title}
      className="inline-flex items-center justify-center rounded-full p-2 text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
    >
      <GitHubLogoIcon className="w-5 h-5" style={{ width: size, height: size }} />
    </a>
  );
};
