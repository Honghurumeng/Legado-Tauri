import { mkdir, copyFile, readdir, stat } from 'node:fs/promises'
import path from 'node:path'

const root = process.cwd()
const target = process.argv[2]
const outputRoot = path.join(root, '构建结果')

async function exists(filePath) {
  try {
    await stat(filePath)
    return true
  } catch {
    return false
  }
}

async function collectFiles(dir, predicate) {
  if (!(await exists(dir))) {
    return []
  }

  const entries = await readdir(dir, { withFileTypes: true })
  const files = []

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...(await collectFiles(fullPath, predicate)))
    } else if (predicate(fullPath)) {
      files.push(fullPath)
    }
  }

  return files
}

async function copyArtifacts(label, files) {
  if (files.length === 0) {
    throw new Error(`No ${label} build artifacts found`)
  }

  const destDir = path.join(outputRoot, label)
  await mkdir(destDir, { recursive: true })

  for (const file of files) {
    const dest = path.join(destDir, path.basename(file))
    await copyFile(file, dest)
    console.log(`copied ${path.relative(root, file)} -> ${path.relative(root, dest)}`)
  }
}

if (target === 'android') {
  const apkDir = path.join(root, 'src-tauri', 'gen', 'android', 'app', 'build', 'outputs', 'apk')
  const apks = await collectFiles(apkDir, (file) => file.endsWith('.apk'))
  await copyArtifacts('android', apks)
} else if (target === 'windows') {
  const bundleDir = path.join(root, 'src-tauri', 'target', 'x86_64-pc-windows-msvc', 'release', 'bundle')
  const installers = await collectFiles(bundleDir, (file) => ['.exe', '.msi'].includes(path.extname(file)))
  await copyArtifacts('windows', installers)
} else {
  throw new Error('Usage: node scripts/copy-build-result.mjs <android|windows>')
}
