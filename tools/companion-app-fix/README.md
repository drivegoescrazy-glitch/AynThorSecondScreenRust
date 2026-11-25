# Companion app 1.0.2: the port banner says what to do

The second screen reported **"Port 48291 is taken by something else — clear a
leftover 'adb reverse tcp:48291 tcp:48291'"**. The cause was right; the
instruction wasn't followable.

## What was wrong with the old text

- `adb` runs on the computer the Thor is plugged into. Android has no
  `adb reverse` to clear, so from the handheld there is nothing to type.
- `adb reverse` is per-connection and **dies with the cable**. Shown while
  nothing is plugged in, the advice sent the player looking for a command whose
  effect was already gone.
- It named one suspect for every case. Something else — an old mod build, a
  protocol mismatch, any program holding the port — got the same sentence.

## The fix (`PortAdvice.kt` + `PortAdviceTest.kt`)

The repository now keeps the hello line each connection sends (`mod` and
`protocol`) and turns it into a verdict and one sentence:

| Who answered | What the player is told |
|---|---|
| `aynthor_secondscreen`, protocol 1 | nothing: it's working |
| adbd | unplug the USB cable (or `adb reverse --remove` on that computer); nothing to do on the Thor |
| anything else that says hello | its name and protocol, and to install both halves from one release |
| nothing at all | another program has the port; close screen-mirroring/game apps and restart |

A foreign hello is reported on the first connection rather than after three
silent ones, since it already names itself. All wording is a pure function, so
the test suite pins who is blamed and that every message names its port.

## Building the 1.0.2 APK

The app is not a submodule of this repo. The release build was made from the
upstream companion source (`exojosh/Android_AynThor_MinecraftSecondScreen`,
`master` = `e7c7aac`, the same commit as the previous 1.0.1 APK's
`version-control-info`) with these two files added, the repository change
described above, and a `signingConfigs.release` that reads
`SECONDSCREEN_KEYSTORE`, `SECONDSCREEN_KEYSTORE_PASSWORD`, `SECONDSCREEN_KEY_ALIAS`
and `SECONDSCREEN_KEY_PASSWORD` from the environment (unsigned without them).

```
./gradlew :app:assembleRelease -x lintVitalAnalyzeRelease
```

Verified on that build: `apksigner verify` passes, `:app:testDebugUnitTest
--tests *PortAdviceTest*` passes 5/5, and the APK's `version-control-info`
records `e7c7aac` (this repo's edits are not in that upstream tree, so the
embedded revision is upstream's).

### Signing, and updating in place

1.0.1 was signed with the upstream author's key (`CN=exojosh, OU=Minecraft Second
Screen`, SHA-256 `8DDA0742…E858B3`). This build uses a fresh upload key
(`CN=AYN Thor Second Screen`, SHA-256 `5153B195…B9A624`). Android will not
replace an app signed with a different key, so **uninstall the old app first**;
its HUD settings are stored by the app and go with it.
