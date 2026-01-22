#!/bin/bash

# Entrypoint script for Bichon Docker container
# Handles PUID/PGID environment variables for proper user permissions

set -e

# Function to create user and switch to it
switch_user() {
    local puid="$1"
    local pgid="$2"

    # Create group if it doesn't exist
    if ! getent group bichon >/dev/null 2>&1; then
        groupadd -g "$pgid" bichon
    fi

    # Create user if it doesn't exist
    if ! getent passwd bichon >/dev/null 2>&1; then
        useradd -u "$puid" -g "$pgid" -s /bin/bash -d /data bichon
    fi

    # Change ownership of directories that the process needs to write to
    chown -R "$puid:$pgid" /data
    chown -R "$puid:$pgid" /opt/bichon
    # Handle mounted storage directories if they exist
    [ -d /envelope ] && chown -R "$puid:$pgid" /envelope
    [ -d /eml ] && chown -R "$puid:$pgid" /eml

    # Switch to the user and execute the command
    exec runuser -u bichon -- "$@"
}

# Check if PUID and PGID are set
if [ -n "$PUID" ] && [ -n "$PGID" ]; then
    echo "Switching to user with PUID=$PUID, PGID=$PGID"
    switch_user "$PUID" "$PGID" "$@"
else
    echo "No PUID/PGID specified, running as root"
    exec "$@"
fi