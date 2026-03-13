#!/usr/bin/env bash
set -euo pipefail

usage() {
	cat <<'USAGE'
Usage: generate-open-source-notices.sh [options]

Options:
  --output <path>         Override the output JSON file (default: board/public/open-source-notices.json)
  --board-dir <path>      Override the dashboard workspace directory (default: <repo>/board)
  --backend-dir <path>    Override the backend workspace directory (default: <repo>/backend)
  --tauri-dir <path>      Override the Desktop Tauri src directory (default: <repo>/desktop/tauri/src-tauri)
  -h, --help              Show this help message
USAGE
}

log() {
	echo "[generate-open-source-notices] $*" >&2
}

require_cmd() {
	if ! command -v "$1" >/dev/null 2>&1; then
		log "error: required command '$1' is not available in PATH"
		exit 1
	fi
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

BOARD_DIR="${PROJECT_ROOT}/board"
BACKEND_DIR="${PROJECT_ROOT}/backend"
TAURI_DIR="${PROJECT_ROOT}/desktop/tauri/src-tauri"
OUTPUT_FILE="${BOARD_DIR}/public/open-source-notices.json"

while [[ $# -gt 0 ]]; do
	case "$1" in
		--output)
			OUTPUT_FILE="$2"
			shift 2
			;;
		--board-dir)
			BOARD_DIR="$2"
			shift 2
			;;
		--backend-dir)
			BACKEND_DIR="$2"
			shift 2
			;;
		--tauri-dir)
			TAURI_DIR="$2"
			shift 2
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			log "error: unknown option '$1'"
			usage
			exit 1
			;;
	esac
done

require_cmd jq
require_cmd cargo
require_cmd node

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

collect_rust_section() {
	local manifest_path="$1"
	local section_id="$2"
	local section_label="$3"
	local output_path="$4"

	if [[ ! -f "$manifest_path" ]]; then
		return 1
	fi

	log "collecting Rust dependencies for ${section_label} (${manifest_path})"
	if ! cargo metadata --format-version 1 --manifest-path "$manifest_path" >"${TMP_DIR}/metadata.json"; then
		log "warning: failed to run cargo metadata for ${manifest_path}, skipping"
		return 1
	fi

	jq \
		--arg id "$section_id" \
		--arg label "$section_label" '
		. as $meta
		| $meta.resolve.root as $root
		| ($meta.resolve.nodes[] | select(.id == $root) | [ .deps[]
				| select(
					((.dep_kinds // []) | any(.kind == null)) or ((.dep_kinds // []) | length == 0)
				)
				| .pkg
			]) as $direct_ids
		| {
			id: $id,
			label: $label,
			packages: (
				[
					$meta.packages[]
					| select(.id as $pkg_id | $direct_ids | index($pkg_id))
					| {
						name,
						version,
						license: (.license // "UNKNOWN"),
						repository: (.repository // ""),
						homepage: (.homepage // ""),
						description: (.description // ""),
						author: (.authors // []) | join(", "),
						licenseFile: (.license_file // "")
					}
				]
				| sort_by(.name)
			)
		}
	' "${TMP_DIR}/metadata.json" >"$output_path"

	return 0
}

collect_node_section() {
	local project_dir="$1"
	local section_id="$2"
	local section_label="$3"
	local output_path="$4"

	if [[ ! -f "${project_dir}/package.json" ]]; then
		return 1
	fi

	if [[ ! -d "${project_dir}/node_modules" ]]; then
		log "warning: ${project_dir}/node_modules missing. Run the workspace install step before generating notices."
		return 1
	fi

	log "collecting Node dependencies for ${section_label} (${project_dir})"
	local node_script="${TMP_DIR}/collect-node-${section_id}.cjs"
	cat >"$node_script" <<'NODE'
const fs = require("fs");
const path = require("path");

const projectDir = process.env.PROJECT_DIR;
const sectionId = process.env.SECTION_ID;
const sectionLabel = process.env.SECTION_LABEL;
const outputPath = process.env.OUTPUT_JSON;

const manifestPath = path.join(projectDir, "package.json");
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));

const directDeps = {
	...(manifest.dependencies || {}),
	...(manifest.optionalDependencies || {}),
};

const packages = Object.keys(directDeps)
	.sort((a, b) => a.localeCompare(b))
	.map((name) => {
		const requested = directDeps[name];
		const pkgDir = path.join(projectDir, "node_modules", name);
		const pkgManifestPath = path.join(pkgDir, "package.json");

		let version = requested;
		let license = "UNKNOWN";
		let repository = "";
		let homepage = "";
		let description = "";
		let author = "";

		if (fs.existsSync(pkgManifestPath)) {
			try {
				const pkg = JSON.parse(fs.readFileSync(pkgManifestPath, "utf8"));
				version = pkg.version || version;
				license = pkg.license || license;
				if (typeof pkg.repository === "string") {
					repository = pkg.repository;
				} else if (pkg.repository && typeof pkg.repository.url === "string") {
					repository = pkg.repository.url;
				}
				homepage = pkg.homepage || "";
				description = pkg.description || "";
				if (typeof pkg.author === "string") {
					author = pkg.author;
				} else if (pkg.author && pkg.author.name) {
					author = pkg.author.name;
					if (pkg.author.email) {
						author += ` <${pkg.author.email}>`;
					}
				}
			} catch (error) {
				// ignore parse errors; fall back to requested metadata
			}
		}

		return {
			name,
			version,
			license,
			repository,
			homepage,
			description,
			author,
			licenseFile: "",
		};
	});

const result = {
	id: sectionId,
	label: sectionLabel,
	packages,
};

fs.writeFileSync(outputPath, JSON.stringify(result));
NODE

	if ! PROJECT_DIR="$project_dir" \
		SECTION_ID="$section_id" \
		SECTION_LABEL="$section_label" \
		OUTPUT_JSON="$output_path" \
		node "$node_script"; then
		log "warning: failed to evaluate package metadata in ${project_dir}"
		return 1
	fi

	return 0
}

declare -a SECTION_FILES=()

	if collect_rust_section "$BACKEND_DIR/Cargo.toml" "backend" "Backend (Rust workspace)" "${TMP_DIR}/backend.json"; then
		SECTION_FILES+=("${TMP_DIR}/backend.json")
	fi

	if collect_rust_section "$TAURI_DIR/Cargo.toml" "tauri" "Desktop Shell (Tauri)" "${TMP_DIR}/tauri.json"; then
		SECTION_FILES+=("${TMP_DIR}/tauri.json")
	fi

	if collect_node_section "$BOARD_DIR" "board" "Dashboard (Web)" "${TMP_DIR}/board.json"; then
		SECTION_FILES+=("${TMP_DIR}/board.json")
	fi

if [[ ${#SECTION_FILES[@]} -eq 0 ]]; then
	log "warning: no sections were generated; skipping output"
	exit 0
fi

mkdir -p "$(dirname "$OUTPUT_FILE")"

log "writing ${OUTPUT_FILE}"
jq -s \
	--arg generatedAt "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
	'{
		generatedAt: $generatedAt,
		sections: (map(select(.packages != null)) | map(.))
	}' \
	"${SECTION_FILES[@]}" >"${TMP_DIR}/open-source-notices.json"

mv "${TMP_DIR}/open-source-notices.json" "$OUTPUT_FILE"
log "completed"
