# gpui-mobile (copie corrigée)

`gpui-mobile/` est une copie de <https://github.com/itsbalamurali/gpui-mobile>,
commit `5fd54f4` (23 octobre 2026), sous licence Apache-2.0 (au choix avec GPL-3.0
et AGPL-3.0, voir les fichiers `LICENSE-*`). `example/` et `screenshots/` n'ont pas
été repris ; les classes Java hôtes de l'exemple sont dans
`../gradle/app/src/main/java/dev/gpui/mobile/`, inchangées.

Pourquoi une copie : gpui-mobile épingle `gpui-pre =0.3.6`, alors que GPUI Kit
0.9.1, dont dépend Mijot, épingle `gpui-pre =0.3.12`. Les deux doivent partager la
même version de GPUI.

Modifications apportées :

- `Cargo.toml` : `gpui-pre` et `gpui-pre-wgpu` passés de `=0.3.6` à `=0.3.12` ;
  sections `[profile.*]` retirées (elles sont ignorées hors de la racine du
  workspace).
  `crate-type` réduit à `rlib` : en `cdylib`, la crate produisait à chaque
  compilation une seconde bibliothèque de plusieurs centaines de mégaoctets,
  que cargo-ndk copiait dans l'APK.
- `src/android/platform.rs` : `Platform::platform_info` et
  `Platform::on_displays_changed`, ajoutés au trait depuis la 0.3.6 ; le
  presse-papiers de GPUI lit et écrit celui d'Android (c'était un tampon interne
  à l'application).
- `src/android/display.rs` : `PlatformDisplay::refresh_interval`.
- `src/android/window.rs` : `PlatformWindow::on_display_changed`, nouveaux champs
  de `RequestFrameOptions`, `gpu_specs` qui renvoie désormais une `Option`.

Seule la cible Android a été adaptée et essayée ; le code iOS est tel qu'en amont.

Pour revenir à la version d'origine dès qu'elle suivra `gpui-pre` : remplacer la
dépendance `path` de `../Cargo.toml` par une dépendance `git` et supprimer ce
dossier.
