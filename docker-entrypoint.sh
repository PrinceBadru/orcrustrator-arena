#!/bin/sh
set -e

PORT=${PORT:-10000}
echo "=========================================================="
echo " Starting Orcrustrator Arena Production Container on :$PORT"
echo "=========================================================="

# Substitute $PORT into nginx configuration
envsubst '$PORT' < /etc/nginx/conf.d/default.conf.template > /etc/nginx/conf.d/default.conf
# Remove default site if present on Debian
rm -f /etc/nginx/sites-enabled/default || true

# Start all internal microservices and gateways in the background
echo ">>> Booting internal microservice mesh..."
/app/bin/registry &
/app/bin/lab &
/app/bin/pharmacy &
/app/bin/terminology &
/app/bin/billing &
/app/bin/notifications &

echo ">>> Booting gateways, recorder, and auditor..."
/app/bin/naive &
/app/bin/hand-rolled &
/app/bin/orcrustrator-gw &
/app/bin/auditor &
/app/bin/recorder &

# Give services 1 second to bind sockets
sleep 1

echo ">>> Arena mesh healthy. Starting public gateway/UI reverse-proxy on :$PORT..."
exec nginx -g "daemon off;"
