# Synthetic APK inputs

`AndroidManifest.xml` is project-authored test input. `scripts/apk_fixtures.py` links
it using Android Build Tools 36.0.0 and platform android-35, then signs with an
 ephemeral JDK 17 key held only in a temporary private directory. The APK contains
no application code, customer assets or credentials. It proves static APK intake,
not device installation/execution. Generated APKs/metadata live in ignored
`.private/test-apks`. Tests must fail, rather than skip, when this prerequisite is absent.
