#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RESOURCES_DIR="${ROOT_DIR}/resources"

mkdir -p "${RESOURCES_DIR}"

base_url="https://raw.githubusercontent.com/zanfranceschi/rinha-de-backend-2026/main/resources"

curl -fsSL "${base_url}/normalization.json" -o "${RESOURCES_DIR}/normalization.json"
curl -fsSL "${base_url}/mcc_risk.json" -o "${RESOURCES_DIR}/mcc_risk.json"
curl -fsSL "${base_url}/references.json.gz" -o "${RESOURCES_DIR}/references.json.gz"

echo "Resources downloaded to ${RESOURCES_DIR}"
