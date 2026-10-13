#!/usr/bin/env node
/**
 * Sync the official website with GitHub releases.
 *
 * What it does:
 *   1. Fetches the release list from the GitHub API.
 *   2. Writes site/data/releases.json (a snapshot the site can read without CORS pain).
 *   3. Re-renders the download cards in site/index.html and site/en/index.html,
 *      plus every <span data-site-version></span> version label.
 *
 * Usage:
 *   node scripts/site-releases.mjs              # online, uses the GitHub API
 *   node scripts/site-releases.mjs --offline    # reuse the existing releases.json
 *
 * Optional env vars:
 *   GITHUB_TOKEN / GH_TOKEN   raises the API rate limit (and lets it see private repos)
 *   SITE_REPO                 override the repository, default EthanHannn/time-remind
 */

import { readFile, writeFile, mkdir, cp } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = resolve(HERE, '..')
const SITE = resolve(ROOT, 'site')
const DATA_DIR = resolve(SITE, 'data')
const DATA_FILE = resolve(DATA_DIR, 'releases.json')

const REPO = process.env.SITE_REPO || 'EthanHannn/time-remind'
const REPO_URL = `https://github.com/${REPO}`
const OFFLINE = process.argv.includes('--offline')
const TOKEN = process.env.GITHUB_TOKEN || process.env.GH_TOKEN || ''

/** How many releases to keep in the JSON snapshot. */
const KEEP = 12
/** How many releases to list in the website history section. */
const HISTORY = 6

const PLATFORM_LABEL = {
  windows: 'Windows',
  macos: 'macOS',
  linux: 'Linux',
}

const HINT = {
  windows: 'Windows 10 / 11 · 双击安装，首次运行可能提示未知发布者',
  macos: 'macOS 社区预览 · 未签名，Gatekeeper 可能拦截',
  linux: 'Linux 社区预览 · 建议先看平台说明',
}

const HINT_EN = {
  windows: 'Windows 10 / 11 · run the installer; SmartScreen may warn once',
  macos: 'macOS community preview · unsigned, Gatekeeper may block it',
  linux: 'Linux community preview · read the platform notes first',
}

function platformOf(name) {
  const n = name.toLowerCase()
  if (n.includes('darwin') || n.includes('macos')) return 'macos'
  if (n.includes('windows') || n.includes('win32')) return 'windows'
  if (n.endsWith('.exe') || n.endsWith('.msi')) return 'windows'
  if (n.endsWith('.dmg') || n.endsWith('.app.zip')) return 'macos'
  if (n.endsWith('.deb') || n.endsWith('.appimage') || n.endsWith('.rpm')) return 'linux'
  if (n.endsWith('.tar.gz') || n.endsWith('.zip')) return 'linux'
  return null
}

/** Prefer the mainstream x86_64 build, then ARM, then anything. */
function pickByArch(candidates) {
  return candidates.find(a => /(amd64|x86_64|x64)/i.test(a.name))
    || candidates.find(a => /(aarch64|arm64)/i.test(a.name))
    || candidates[0]
}

function humanSize(bytes) {
  if (!bytes) return ''
  const mb = bytes / 1024 / 1024
  return mb >= 1 ? `${mb.toFixed(1)} MB` : `${(bytes / 1024).toFixed(0)} KB`
}

function formatDate(iso) {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return ''
  return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, '0')}-${String(d.getUTCDate()).padStart(2, '0')}`
}

function esc(s) {
  return String(s).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', '\'': '&#39;' }[c]))
}

/** Root-relative prefix so the same markup works at site root and under site/en/. */
function prefixFor(htmlPath) {
  const parts = htmlPath.split(/[\\/]/)
  return parts[parts.length - 2] === 'en' ? '../' : ''
}

function pickAsset(assets, platform) {
  const pool = assets.filter(a => a.platform === platform)
  if (pool.length === 0) return null

  if (platform === 'windows') {
    return pickByArch(pool.filter(a => /_nsis\.exe$/i.test(a.name)))
      || pickByArch(pool.filter(a => /\.exe$/i.test(a.name)))
      || pickByArch(pool.filter(a => /\.msi$/i.test(a.name)))
  }
  if (platform === 'macos') {
    // The .app.tar.gz asset is the updater payload, not something users install.
    return pickByArch(pool.filter(a => /\.dmg$/i.test(a.name)))
      || pickByArch(pool.filter(a => /\.app\.zip$/i.test(a.name)))
      || pickByArch(pool.filter(a => /\.zip$/i.test(a.name)))
  }
  if (platform === 'linux') {
    return pickByArch(pool.filter(a => /\.deb$/i.test(a.name)))
      || pickByArch(pool.filter(a => /\.appimage$/i.test(a.name)))
      || pickByArch(pool.filter(a => /\.rpm$/i.test(a.name)))
  }
  return pickByArch(pool)
}

function renderDownloads(version, assets, dict) {
  const found = ['windows', 'macos', 'linux']
    .map(p => ({ platform: p, asset: pickAsset(assets, p) }))
    .filter(x => x.asset)

  const cards = found.map(({ platform, asset }) => {
    const size = humanSize(asset.size)
    return `          <a class="dl-card" href="${esc(asset.url)}" data-platform="${platform}">
            <span class="dl-card__icon">${ICONS[platform]}</span>
            <span class="dl-card__body">
              <span class="dl-card__platform">${PLATFORM_LABEL[platform]}</span>
              <span class="dl-card__file">${esc(asset.name)}</span>
              <span class="dl-card__meta">${size ? `${size} · ` : ''}${esc(dict.hint[platform])}</span>
            </span>
            <span class="dl-card__cta">${esc(dict.download)}</span>
          </a>`
  }).join('\n')

  return `<!-- site:downloads:start (generated by scripts/site-releases.mjs) -->
        <p class="dl-version">${esc(dict.current)} <strong>v${esc(version)}</strong>${esc(dict.platforms(found.length))}</p>
${cards}
        <p class="dl-all"><a href="${REPO_URL}/releases" target="_blank" rel="noopener">${esc(dict.all)} →</a></p>
        <!-- site:downloads:end -->`
}

function renderHistory(releases, dict) {
  const rows = releases.slice(0, HISTORY).map((r) => {
    const bits = []
    if (r.assets.some(a => a.platform === 'windows')) bits.push('Windows')
    if (r.assets.some(a => a.platform === 'macos')) bits.push(dict.macPreview)
    if (r.assets.some(a => a.platform === 'linux')) bits.push(dict.linuxPreview)
    return `            <li>
              <a href="${esc(r.url)}" target="_blank" rel="noopener">
                <span class="rel-tag">${esc(r.tag)}</span>
                <span class="rel-date">${esc(r.date)}</span>
                <span class="rel-platforms">${esc(bits.join(' · '))}</span>
              </a>
            </li>`
  }).join('\n')

  return `<!-- site:history:start (generated by scripts/site-releases.mjs) -->
          <ul class="rel-list">
${rows}
          </ul>
          <!-- site:history:end -->`
}

function replaceBlock(html, name, replacement) {
  const re = new RegExp(`<!-- site:${name}:start[\\s\\S]*?site:${name}:end -->`)
  if (!re.test(html)) {
    console.warn(`  ! block site:${name} not found, skipped`)
    return html
  }
  return html.replace(re, replacement)
}

async function loadOffline() {
  const raw = await readFile(DATA_FILE, 'utf8')
  return JSON.parse(raw)
}

async function fetchReleases() {
  const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'time-remind-site-sync' }
  if (TOKEN) headers.Authorization = `Bearer ${TOKEN}`
  const res = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=100`, { headers })
  if (!res.ok) throw new Error(`GitHub API ${res.status} ${res.statusText}`)
  return res.json()
}

async function main() {
  /*
   * The website ships its own copy of the screenshots so that `site/` is a
   * self-contained directory: a Docker build with `site` as context, or copying
   * `site/` to a VPS, both work without reaching outside the folder. The README
   * keeps using assets/screenshots, so this copy keeps them identical.
   */
  await mkdir(resolve(SITE, 'assets'), { recursive: true })
  await cp(resolve(ROOT, 'assets', 'screenshots'), resolve(SITE, 'assets', 'screenshots'), { recursive: true })
  console.log('synced assets/screenshots -> site/assets/screenshots')

  let snapshot
  let latest

  if (OFFLINE) {
    snapshot = await loadOffline()
    latest = snapshot.latest
    console.log(`offline: using ${DATA_FILE}`)
  }
  else {
    const releases = await fetchReleases()
    if (!Array.isArray(releases) || releases.length === 0) throw new Error('GitHub returned no releases')

    const mapped = releases.map(r => ({
      tag: r.tag_name,
      name: r.name || r.tag_name,
      version: String(r.tag_name).replace(/^v/, ''),
      date: formatDate(r.published_at || r.created_at),
      url: r.html_url,
      prerelease: Boolean(r.prerelease),
      assets: (r.assets || [])
        .map(a => ({ name: a.name, size: a.size, platform: platformOf(a.name), url: a.browser_download_url }))
        .filter(a => a.platform),
    }))

    latest = mapped.find(r => !r.prerelease) || mapped[0]
    snapshot = {
      repo: REPO,
      repoUrl: REPO_URL,
      syncedAt: new Date().toISOString(),
      latest,
      releases: mapped.slice(0, KEEP),
    }

    await mkdir(DATA_DIR, { recursive: true })
    await writeFile(DATA_FILE, `${JSON.stringify(snapshot, null, 2)}\n`, 'utf8')
    console.log(`wrote ${DATA_DIR}\\releases.json (${snapshot.releases.length} releases, latest ${latest.tag})`)
  }

  if (!latest) throw new Error('no latest release available')

  const DICT_ZH = {
    download: '下载',
    current: '当前版本',
    platforms: n => ` · ${n} 个平台可选`,
    all: '查看全部版本与校验文件',
    hint: HINT,
    macPreview: 'macOS 预览',
    linuxPreview: 'Linux 预览',
  }

  const DICT_EN = {
    download: 'Download',
    current: 'Current release',
    platforms: n => ` · ${n} platforms available`,
    all: 'See every release and checksums',
    hint: HINT_EN,
    macPreview: 'macOS preview',
    linuxPreview: 'Linux preview',
  }

  const targets = [
    { path: resolve(SITE, 'index.html'), dict: DICT_ZH },
    { path: resolve(SITE, 'en', 'index.html'), dict: DICT_EN },
  ]

  for (const target of targets) {
    let html
    try {
      html = await readFile(target.path, 'utf8')
    }
    catch {
      console.warn(`  ! ${target.path} not found, skipped`)
      continue
    }

    html = replaceBlock(html, 'downloads', renderDownloads(latest.version, latest.assets, target.dict))
    html = replaceBlock(html, 'history', renderHistory(snapshot.releases, target.dict))
    html = html.replace(/<span data-site-version>[^<]*<\/span>/g, `<span data-site-version>${esc(latest.version)}</span>`)

    await writeFile(target.path, html, 'utf8')
    console.log(`  updated ${target.path}`)
  }
}

const ICONS = {
  windows: '<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M3 5.5l7.4-1v7H3v-6zm0 13l7.4 1v-7H3v6zm8.6 1.2L21 21V12.5h-9.4v7.2zM11.6 4.3v7.2H21V3l-9.4 1.3z"/></svg>',
  macos: '<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M16.4 12.7c0-2.4 2-3.6 2.1-3.6-1.1-1.7-2.9-1.9-3.5-1.9-1.5-.2-2.8.9-3.6.9-.8 0-1.9-.9-3.1-.9-1.6 0-3.1.9-3.9 2.4-1.7 2.9-.4 7.2 1.2 9.5.8 1.2 1.8 2.5 3 2.4 1.2 0 1.7-.8 3.1-.8 1.4 0 1.9.8 3.1.8 1.3 0 2.1-1.2 2.9-2.4.9-1.3 1.3-2.6 1.3-2.7-.1 0-3-.9-3-3.7zM14 5.1c.6-.8 1.1-1.9 1-3-.9 0-2 .6-2.7 1.4-.6.7-1.1 1.8-1 2.9 1 .1 2-.5 2.7-1.3z"/></svg>',
  linux: '<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M12 2c-1.9 0-3.2 1.6-3.2 3.7 0 1.1.2 2 .2 3-.3.9-1.3 2-1.9 3.2-.7 1.4-.4 2.4-1 3.6-.4.8-1 1.4-1 2.1 0 .6.4 1 .9 1.3.6.4 1.2.3 1.7.9.4.5.6 1.2 1.5 1.7.7.4 1.8.5 2.7.5s1.9-.1 2.6-.5c.9-.5 1.1-1.2 1.5-1.7.5-.6 1.1-.5 1.7-.9.5-.3.9-.7.9-1.3 0-.7-.6-1.3-1-2.1-.6-1.2-.3-2.2-1-3.6-.6-1.2-1.6-2.3-1.9-3.2 0-1 .2-1.9.2-3C15.2 3.6 13.9 2 12 2zm0 1.6c1 0 1.6.9 1.6 2.1 0 .8-.1 1.5-.2 2.2h-2.8c-.1-.7-.2-1.4-.2-2.2 0-1.2.6-2.1 1.6-2.1zm-1.2 5.6h2.4c.3.5.6 1.1.7 1.6-.6.3-1.2.4-1.9.4s-1.3-.1-1.9-.4c.1-.5.4-1.1.7-1.6zm-2.1 4c.5 0 .9.5.9 1.1s-.4 1.1-.9 1.1-.9-.5-.9-1.1.4-1.1.9-1.1zm6.6 0c.5 0 .9.5.9 1.1s-.4 1.1-.9 1.1-.9-.5-.9-1.1.4-1.1.9-1.1z"/></svg>',
}

main().catch((err) => {
  console.error(`site-releases failed: ${err.message}`)
  process.exitCode = 1
})
