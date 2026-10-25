#!/bin/sh
set -e

PUBLIC_PORT=${PORT:-10000}
echo "=========================================================="
echo " Starting Orcrustrator Arena Production Container on :$PUBLIC_PORT"
echo "=========================================================="

# Export internal service URLs for gateways and auditor
export REGISTRY_URL="http://127.0.0.1:8081"
export LAB_URL="http://127.0.0.1:8082"
export PHARMACY_URL="http://127.0.0.1:8083"
export TERMINOLOGY_URL="http://127.0.0.1:8084"
export BILLING_URL="http://127.0.0.1:8085"
export NOTIFICATIONS_URL="http://127.0.0.1:8086"

# Substitute $PUBLIC_PORT into nginx configuration
PORT=$PUBLIC_PORT envsubst '$PORT' < /etc/nginx/conf.d/default.conf.template > /etc/nginx/conf.d/default.conf
# Remove default site if present on Debian
rm -f /etc/nginx/sites-enabled/default || true

# Start all internal microservices with explicit non-conflicting internal ports
echo ">>> Booting internal microservice mesh..."
PORT=8081 /app/bin/registry &
PORT=8082 /app/bin/lab &
PORT=8083 /app/bin/pharmacy &
PORT=8084 /app/bin/terminology &
PORT=8085 /app/bin/billing &
PORT=8086 /app/bin/notifications &

echo ">>> Booting gateways, recorder, and auditor..."
PORT=8001 /app/bin/naive &
PORT=8002 /app/bin/hand-rolled &
PORT=8003 /app/bin/orcrustrator-gw &
PORT=8090 /app/bin/auditor &
PORT=8095 /app/bin/recorder &

# Give services 2 seconds to bind internal sockets
sleep 2

echo ">>> Arena mesh healthy. Starting public reverse-proxy on :$PUBLIC_PORT..."
exec nginx -g "daemon off;"
