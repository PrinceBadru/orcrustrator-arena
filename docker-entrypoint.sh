#!/bin/sh
set -e

PORT=${PORT:-3000}
echo "=========================================================="
echo " Starting Orcrustrator Arena Production Container on :$PORT"
echo "=========================================================="

# Substitute $PORT into nginx configuration
envsubst '$PORT' < /etc/nginx/conf.d/default.conf.template > /etc/nginx/conf.d/default.conf

# Start all internal microservices and gateways in the background (each consumes ~2MB RAM)
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
