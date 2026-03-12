# mntpack 0.6.4 (2026-03-12)

## Fixed
- `sync` now attempts to use prebuilt binaries published in the cache/index release repo (for example `mntpack/mntpack-index`) before falling back to upstream releases or local builds.
- Added `.tar.xz`/`.txz` release extraction support so cached release archives can be used directly.
- Cache lookup failures are now warning-only and no longer abort sync when a cached binary cannot be read.

## Changed
- Binary cache repo resolution now falls back to `syncDispatch.repo` when `binaryCache.repo` is unset.
- Installer release workflow now uses this file as the GitHub release body.
