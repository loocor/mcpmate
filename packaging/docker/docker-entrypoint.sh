#!/usr/bin/env sh
set -eu

if [ "$#" -gt 0 ]; then
  exec mcpmate "$@"
fi

api_port="${MCPMATE_API_PORT:-8080}"
mcp_port="${MCPMATE_MCP_PORT:-8000}"
dashboard_port="${MCPMATE_DASHBOARD_PORT:-3000}"
log_level="${MCPMATE_LOG:-info}"
transport="${MCPMATE_TRANSPORT:-uni}"
internal_api_port="${MCPMATE_INTERNAL_API_PORT:-18080}"
internal_mcp_port="${MCPMATE_INTERNAL_MCP_PORT:-18000}"

cat >/tmp/nginx.conf <<EOF
worker_processes 1;
pid /tmp/nginx/nginx.pid;

events {
  worker_connections 1024;
}

http {
  include /etc/nginx/mime.types;
  default_type application/octet-stream;
  access_log /dev/stdout;
  error_log /dev/stderr warn;
  sendfile on;

  proxy_http_version 1.1;
  proxy_set_header Host \$host;
  proxy_set_header X-Real-IP \$remote_addr;
  proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
  proxy_set_header X-Forwarded-Proto \$scheme;

  server {
    listen ${dashboard_port};
    root /opt/mcpmate/board;
    index index.html;

    location /api/ {
      proxy_pass http://127.0.0.1:${internal_api_port};
    }

    location /ws {
      proxy_pass http://127.0.0.1:${internal_api_port};
      proxy_set_header Upgrade \$http_upgrade;
      proxy_set_header Connection "upgrade";
    }

    location / {
      try_files \$uri \$uri/ /index.html;
    }
  }

  server {
    listen ${api_port};

    location / {
      proxy_pass http://127.0.0.1:${internal_api_port};
    }
  }

  server {
    listen ${mcp_port};

    location / {
      proxy_pass http://127.0.0.1:${internal_mcp_port};
    }
  }
}
EOF

nginx -c /tmp/nginx.conf

set -- \
  --api-port "${internal_api_port}" \
  --mcp-port "${internal_mcp_port}" \
  --log-level "${log_level}" \
  --transport "${transport}"

exec mcpmate "$@"
