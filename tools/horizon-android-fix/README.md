# Android import fix: "Cross-device link"

The launcher's Mods page answered **Import failed: Cross-device link (os error 18)** for
every `.hmod` on the Thor. The file was never the problem; Android put the import
scratch file on another filesystem than the profile folder, and the install step
moved the file with `rename()`, which the kernel refuses across filesystems
(`EXDEV`). It then aborted, so the mod never appeared.

`documents.patch` is the fix against `kaito65452/horizon-android`, branch
`android` at `4681e5f` (the Horizon Android client); `git apply --check`
passes against a fresh clone of it. It replaces both `rename()` calls (a new
import, and the Replace/Keep both choice after a conflict) with `place()`:

- copy into `.<name>.importing` **inside the destination folder** (same
  filesystem by construction), `fsync`, then rename within that one directory;
- check free space first, so a full folder says so instead of leaving a partial
  file;
- delete the scratch file on any failure.

## Applying it

```sh
git clone -b android https://github.com/kaito65452/horizon-android
cd horizon-android
git apply tools/horizon-android-fix/documents.patch
```

The patch also adds `libc` to the Android-only dependencies of
`crates/horizon-client` for the `statvfs` free-space check.

## Checks run here

- `place()` was extracted from `documents.rs` as written and built on the host
  with the Android `statvfs` code enabled (type-checked against the real `libc`
  crate), with five tests: a file lands with its bytes and no scratch remains,
  an existing target is replaced only after a successful copy, a missing source
  leaves neither target nor scratch, a copy that fails after opening the scratch
  file removes it and leaves no target, and free space is reported for a real
  directory. All five passed.
- The whole `horizon-client` crate was **not** compiled: its dependency set is
  far larger than this workspace can build.

## Meanwhile, on a stock launcher

The same error is avoided by getting the file into the profile folder without
the import path: copy `aynthor_secondscreen.hmod` into the game folder's
`horizon-mods/` with the built-in Files app (Horizon Android appears in its
navigation drawer), then add `mod.aynthor_secondscreen:true` to
`horizon-mods.txt` beside `options.txt`.
