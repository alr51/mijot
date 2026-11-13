//! Mijot : menus de la semaine à budget donné et liste de courses.
//!
//! Les données sont locales, dans le dossier de données de l'utilisateur : le
//! catalogue (ingrédients, prix, recettes) dans une base SQLite, remplie au
//! premier lancement et modifiable depuis l'application ; l'état (budget, menu,
//! cases cochées) dans un fichier JSON.
//!
//! Le réseau ne sert qu'à une chose, sur demande : suggérer un prix d'après
//! Open Prices (voir `prix_ouverts.rs`).

// Pas de fenêtre console sous Windows (sauf en debug).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod catalogue;
mod donnees;
mod etat;
mod plan;
mod prix_ouverts;
mod ui;

use gpui_kit::assets::AllAssets;
use gpui_kit::component::Theme;
use gpui_kit::*;

use catalogue::Catalogue;

/// Ouvre la base du catalogue. Si elle est illisible, l'application démarre quand
/// même sur le catalogue de départ, en mémoire, avec un message pour l'utilisateur.
fn ouvrir_catalogue() -> (Catalogue, Option<String>) {
    let ouverture = match etat::dossier() {
        Some(dossier) => std::fs::create_dir_all(&dossier)
            .map_err(|e| e.to_string())
            .and_then(|()| Catalogue::ouvrir(&dossier.join("mijot.db")).map_err(|e| e.to_string())),
        None => Err("dossier de données introuvable".to_owned()),
    };
    match ouverture {
        Ok(catalogue) => (catalogue, None),
        Err(e) => {
            eprintln!("mijot : catalogue illisible : {e}");
            let secours = Catalogue::en_memoire().expect("catalogue de départ en mémoire");
            let alerte = format!(
                "Catalogue illisible ({e}). Le catalogue de départ est chargé à la place : les modifications de prix et de recettes ne seront pas conservées."
            );
            (secours, Some(alerte))
        }
    }
}

fn main() {
    let (catalogue, alerte) = ouvrir_catalogue();
    let etat = etat::Etat::charger(&catalogue);

    // `AllAssets` : catalogue Lucide complet (toque, panier…).
    gpui_kit::application()
        .with_assets(AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            Theme::sync_system_appearance(None, cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1180.), px(800.)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(640.), px(480.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Mijot".into()),
                    ..Default::default()
                }),
                app_id: Some("mijot".into()),
                ..Default::default()
            };
            let ouverte = gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| ui::Mijot::new(catalogue, etat, alerte, window, cx))
            });
            if let Err(e) = ouverte {
                eprintln!("mijot : fenêtre impossible à ouvrir : {e}");
                cx.quit();
            }
            cx.activate(true);
        });
}
