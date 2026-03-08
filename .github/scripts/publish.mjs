#!/usr/bin/env node

/**
 * Obsidian Publish Sync Script
 *
 * Syncs local vault files to Obsidian Publish by:
 * 1. Fetching the list of currently published files from the API
 * 2. Comparing local file hashes with remote hashes
 * 3. Uploading any files that have changed
 *
 * This only updates files that are ALREADY published -- it does not
 * add new files or remove existing ones. Publishing/unpublishing
 * should be managed through the Obsidian app.
 *
 * Required environment variables:
 *   OBSIDIAN_PUBLISH_TOKEN - API token for Obsidian Publish
 *   VAULT_ROOT             - path to the Obsidian vault directory
 *
 * The site ID is read from .obsidian/publish.json in the vault root.
 */

import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const VAULT_ROOT = process.env.VAULT_ROOT || ".";
const PUBLISH_CONFIG_PATH = resolve(
  VAULT_ROOT,
  ".obsidian",
  "publish.json"
);

async function loadPublishConfig() {
  const raw = await readFile(PUBLISH_CONFIG_PATH, "utf-8");
  const config = JSON.parse(raw);
  if (!config.siteId) {
    throw new Error("No siteId found in .obsidian/publish.json");
  }
  return config;
}

async function apiRequest(host, endpoint, options = {}) {
  const url = `https://${host}/api${endpoint}`;
  const res = await fetch(url, options);
  if (!res.ok) {
    const body = await res.text();
    throw new Error(
      `API ${endpoint} failed: ${res.status} ${res.statusText} -- ${body}`
    );
  }
  return res;
}

async function getPublishedFiles(host, siteId, token) {
  const res = await apiRequest(host, "/list", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id: siteId, token }),
  });
  const files = await res.json();
  const result = {};
  for (const file of files) {
    result[file.path] = file.hash;
  }
  return result;
}

async function uploadFile(host, siteId, token, filePath) {
  const fullPath = resolve(VAULT_ROOT, filePath);
  const buffer = await readFile(fullPath);
  const hash = createHash("sha256").update(buffer).digest("hex");

  const res = await apiRequest(host, "/upload", {
    method: "POST",
    headers: {
      "Content-Type": "application/octet-stream",
      "obs-hash": hash,
      "obs-id": siteId,
      "obs-path": filePath,
      "obs-token": token,
    },
    body: buffer,
  });
  return { hash, status: res.status };
}

function hashFile(buffer) {
  return createHash("sha256").update(buffer).digest("hex");
}

async function main() {
  const token = process.env.OBSIDIAN_PUBLISH_TOKEN;
  if (!token) {
    console.error(
      "Error: OBSIDIAN_PUBLISH_TOKEN environment variable is required"
    );
    process.exit(1);
  }

  // Load publish config
  const config = await loadPublishConfig();
  const { siteId, host } = config;
  const apiHost = host || "publish-01.obsidian.md";
  console.log(`Site ID: ${siteId}`);
  console.log(`API host: ${apiHost}`);
  console.log(`Vault root: ${resolve(VAULT_ROOT)}`);

  // Get currently published files
  console.log("\nFetching published files...");
  const published = await getPublishedFiles(apiHost, siteId, token);
  const publishedPaths = Object.keys(published);
  console.log(`Found ${publishedPaths.length} published file(s)`);

  // Compare hashes and upload changed files
  let updated = 0;
  let skipped = 0;
  let missing = 0;
  const errors = [];

  for (const filePath of publishedPaths) {
    const fullPath = resolve(VAULT_ROOT, filePath);
    let buffer;
    try {
      buffer = await readFile(fullPath);
    } catch {
      console.log(`  MISSING: ${filePath} (not found locally, skipping)`);
      missing++;
      continue;
    }

    const localHash = hashFile(buffer);
    const remoteHash = published[filePath];

    if (localHash === remoteHash) {
      skipped++;
      continue;
    }

    console.log(`  UPDATING: ${filePath}`);
    console.log(`    remote: ${remoteHash}`);
    console.log(`    local:  ${localHash}`);
    try {
      await uploadFile(apiHost, siteId, token, filePath);
      updated++;
      console.log(`    done`);
    } catch (err) {
      errors.push({ filePath, error: err.message });
      console.error(`    failed: ${err.message}`);
    }
  }

  // Summary
  console.log("\n--- Summary ---");
  console.log(`Updated:  ${updated}`);
  console.log(`Skipped:  ${skipped} (unchanged)`);
  console.log(`Missing:  ${missing} (not in repo)`);
  console.log(`Errors:   ${errors.length}`);

  if (errors.length > 0) {
    console.error("\nFailed uploads:");
    for (const { filePath, error } of errors) {
      console.error(`  ${filePath}: ${error}`);
    }
    process.exit(1);
  }
}

main().catch((err) => {
  console.error("Fatal error:", err.message);
  process.exit(1);
});
