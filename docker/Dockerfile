# Use Debian bullseye-slim as the base image for newer OpenSSL
FROM ubuntu:24.04
ARG CRATE_VERSION
ARG TARGETARCH

# Set maintainer metadata
LABEL maintainer="rustmailer <rustmailer.git@gmail.com>"
LABEL version="${CRATE_VERSION}"
LABEL description="Dockerized Bichon service"

# Set working directory
WORKDIR /opt/bichon

# Copy compiled binary (ensure it's statically linked or compatible with bullseye)
COPY ${TARGETARCH}/bichon /opt/bichon/bichon
COPY ${TARGETARCH}/LICENSE /opt/bichon/


# cli tools
COPY ${TARGETARCH}/bichonctl /usr/local/bin/bichonctl
COPY ${TARGETARCH}/bichon-admin /usr/local/bin/bichon-admin

# Set permissions
RUN chmod +x /opt/bichon/bichon
RUN chmod +x /usr/local/bin/bichonctl
RUN chmod +x /usr/local/bin/bichon-admin


# Install ca-certificates to ensure HTTPS certificate verification works correctly
RUN apt update && apt install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
# Create data directory
RUN mkdir -p /data

# Expose default ports
EXPOSE 15630

# Set volume and working directory
WORKDIR /data

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -fs http://localhost:15630/api/status || exit 1

CMD ["/opt/bichon/bichon"]
