#!/bin/bash
set -e

SERVICE_NAME="document-generation-service"
REGISTRY="us-docker.pkg.dev/mcxtest/qxproveit"
NAMESPACE="qxproveit"

cd "$(dirname "$0")"
echo "Deploying ${SERVICE_NAME}..."

echo "Building Docker image..."
docker buildx build --platform linux/amd64 \
  -t ${REGISTRY}/${SERVICE_NAME}:latest \
  --load \
  .

echo "Pushing to registry..."
docker push ${REGISTRY}/${SERVICE_NAME}:latest

echo "Deploying to GKE..."
kubectl apply -f kubernetes/ -n ${NAMESPACE}
kubectl rollout restart deployment/${SERVICE_NAME} -n ${NAMESPACE}
kubectl rollout status deployment/${SERVICE_NAME} -n ${NAMESPACE} --timeout=5m

echo "✓ ${SERVICE_NAME} deployed successfully!"
