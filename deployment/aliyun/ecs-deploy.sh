#!/usr/bin/env bash
# Deploy the IndexLink backend on an Alibaba Cloud ECS host.
#
# This script never creates, prints, or uploads secrets. Create the ignored
# repository-root .env file from .env.example before invoking it.
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
compose_file="$repository_root/deployment/docker-compose.yml"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required. Install Docker Engine and Docker Compose first." >&2
  exit 1
fi

if [[ ! -f "$repository_root/.env" ]]; then
  echo "Missing $repository_root/.env. Copy .env.example and configure local secrets first." >&2
  exit 1
fi

cd "$repository_root"
docker compose --project-name indexlink -f "$compose_file" up --build --detach
docker compose --project-name indexlink -f "$compose_file" ps

for _ in {1..12}; do
  if curl --fail --silent http://127.0.0.1:8080/ready >/dev/null; then
    echo "IndexLink is ready on port 8080."
    exit 0
  fi
  sleep 2
done

echo "IndexLink did not become ready; inspect logs with: docker compose --project-name indexlink -f $compose_file logs --tail=100 server" >&2
exit 1
