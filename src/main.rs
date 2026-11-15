//! Lanceur de bureau de Mijot (Linux, macOS, Windows).

// Pas de fenêtre console sous Windows (sauf en debug).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use mijot::gpui_kit::assets::AllAssets;
use mijot::gpui_kit::{self, *};

fn main() {
    // `AllAssets` : catalogue Lucide complet (toque, panier…).
    gpui_kit::application()
        .with_assets(AllAssets)
        .run(move |cx| {
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1280.), px(840.)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                // Sous 720 px de large, l'interface passe au format téléphone.
                window_min_size: Some(size(px(360.), px(560.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Mijot".into()),
                    ..Default::default()
                }),
                app_id: Some("mijot".into()),
                ..Default::default()
            };
            mijot::demarrer(options, cx);
        });
}
