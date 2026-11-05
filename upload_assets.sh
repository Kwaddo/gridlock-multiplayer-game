#!/usr/bin/env bash
# Uploads client/assets (kept out of git) to the gridlock-assets bucket on
# Dilmune, then starts the Deploy workflow so the change goes live. The
# workflow builds the browser version from the bucket's copy, and the registry
# serves the models from it (see registry/dcs/README.md).
#
#   AWS_ACCESS_KEY_ID=<key> AWS_SECRET_ACCESS_KEY=<secret> ./upload_assets.sh
#
# Files deleted locally stay in the bucket; remove them there by hand
# (aws s3 rm ... --endpoint-url https://api.dilmune.com/s3).
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENDPOINT="https://api.dilmune.com/s3"
export AWS_REGION="${AWS_REGION:-auto}"
# Newer AWS CLIs send large files as aws-chunked uploads with checksums, and
# Dilmune's S3 would store the chunk framing inside the file. Plain uploads only.
export AWS_REQUEST_CHECKSUM_CALCULATION=when_required
export AWS_RESPONSE_CHECKSUM_VALIDATION=when_required

if [ -z "$AWS_ACCESS_KEY_ID" ] || [ -z "$AWS_SECRET_ACCESS_KEY" ]; then
    echo "error: set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY to a Dilmune S3 key" >&2
    exit 1
fi

echo "==> Uploading client/assets to s3://gridlock-assets/..."
aws s3 sync "$SCRIPT_DIR/client/assets/" s3://gridlock-assets/ \
    --endpoint-url "$ENDPOINT" --exclude '*.DS_Store' --no-progress

echo "==> Starting the Deploy workflow..."
if command -v gh >/dev/null 2>&1; then
    if gh workflow run deploy.yml --ref main --repo Kwaddo/gridlock-multiplayer-game; then
        echo "    Follow it with: gh run watch --repo Kwaddo/gridlock-multiplayer-game"
    else
        echo "    Could not start it (the upload itself is done): run Deploy from the Actions tab."
    fi
else
    echo "    gh is not installed: run the Deploy workflow from the repository's Actions tab."
fi
