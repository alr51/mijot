#!/usr/bin/env bash
# Construit l'application Android de Mijot : bibliothèque Rust (cargo-ndk),
# puis APK (Gradle).
#
#   ./build.sh                 APK de mise au point pour téléphone (arm64-v8a)
#   ./build.sh --emulateur     pour l'émulateur d'un PC (x86_64)
#   ./build.sh --release       bibliothèque Rust optimisée
#   ./build.sh --installer     installe et lance sur l'appareil branché (adb)
#
# Prérequis : SDK Android (plateforme 36) et NDK, cargo-ndk
# (`cargo install cargo-ndk`), un JDK 17 ou plus récent.
set -euo pipefail

ici="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
abi=arm64-v8a
cible=aarch64-linux-android
profil=()
installer=false
for option in "$@"; do
    case "$option" in
        --emulateur) abi=x86_64; cible=x86_64-linux-android ;;
        --release) profil=(--release) ;;
        --installer) installer=true ;;
        *) echo "option inconnue : $option" >&2; exit 2 ;;
    esac
done

# Le SDK : celui d'ANDROID_HOME s'il contient un NDK, sinon celui d'Android Studio.
sdk="${ANDROID_HOME:-}"
if [[ ! -d "$sdk/ndk" ]]; then
    sdk="$HOME/Android/Sdk"
fi
if [[ ! -d "$sdk/ndk" ]]; then
    echo "NDK introuvable : installe-le (sdkmanager \"ndk;…\") ou définis ANDROID_HOME." >&2
    exit 1
fi
export ANDROID_HOME="$sdk"
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
    ANDROID_NDK_HOME="$(find "$sdk/ndk" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -n 1)"
    export ANDROID_NDK_HOME
fi

rustup target add "$cible" >/dev/null

# Une seule architecture dans l'APK : celle qui vient d'être compilée.
bibliotheques="$ici/gradle/app/src/main/jniLibs"
rm -rf "$bibliotheques"
(cd "$ici" && cargo ndk -t "$abi" -P 26 -o "$bibliotheques" build "${profil[@]}")
# Sans ses informations de débogage, la bibliothèque passe de plus d'un
# gigaoctet à quelques dizaines de mégaoctets ; les noms de fonctions restent.
"$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-strip --strip-debug \
    "$bibliotheques/$abi/libmijot_android.so"

(cd "$ici/gradle" && ./gradlew --quiet assembleDebug)
apk="$ici/gradle/app/build/outputs/apk/debug/app-debug.apk"
echo "APK : $apk ($(du -h "$apk" | cut -f1))"

if $installer; then
    adb install -r "$apk"
    adb shell am start -n app.mijot/dev.gpui.mobile.GpuiActivity
fi
