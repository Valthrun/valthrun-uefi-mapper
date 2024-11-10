artifact="$1"
iso_path="$2"
payload_git_hash="$3"

iso_name=$(basename -- "$iso_path")
git_commit_shash=$(git rev-parse --short "$GITHUB_SHA")
artifact_track="release"

echo "Uploading $iso_path"
curl -H "Content-Type:multipart/form-data" \
    -X POST \
    -F "info={\"version\": \"$payload_git_hash ($git_commit_shash)\", \"versionHash\": \"$payload_git_hash\", \"updateLatest\": true }" \
    -F "payload=@$iso_path; filename=${artifact}_${payload_git_hash}.${iso_name##*.}" \
    "https://valth.run/api/artifacts/$artifact/release-iso?api-key=$ARTIFACT_API_KEY" || exit 1

echo ""