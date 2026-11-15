//! Lanceur Android de Mijot : crée la plateforme gpui-mobile, indique à
//! l'application son dossier de données et les marges des barres système, puis
//! lui passe la main.
//!
//! `android-activity` appelle `android_main` sur un fil natif dédié, une fois la
//! bibliothèque chargée par `GpuiActivity` (voir `gradle/`).

#![cfg(target_os = "android")]

use std::sync::Mutex;
use std::time::{Duration, Instant};

use ::jni::objects::JObject;
use ::jni::{jni_sig, jni_str};
use android_activity::AndroidApp;
use gpui_mobile::android::jni;
use mijot::gpui_kit::assets::AllAssets;
use mijot::gpui_kit::{App, Application, WindowOptions};
use mijot::hote::{Hote, Marges};

/// Hauteurs, en pixels physiques, de ce que le système dessine par-dessus la
/// fenêtre : barre d'état en haut ; barre de navigation ou clavier en bas.
///
/// L'activité est affichée bord à bord (c'est la règle depuis Android 15) : le
/// rectangle de contenu que connaît gpui-mobile couvre tout l'écran, il faut
/// donc demander ces marges à la vue racine.
fn marges_systeme() -> Result<(i32, i32), String> {
    jni::with_env(|env| {
        let activite = jni::activity(env)?;
        let mut objet = |de: &JObject, methode, signature| {
            env.call_method(de, methode, signature, &[])
                .and_then(|valeur| valeur.l())
                .map_err(|e| {
                    env.exception_clear();
                    e.to_string()
                })
        };
        let fenetre = objet(
            &activite,
            jni_str!("getWindow"),
            jni_sig!("()Landroid/view/Window;"),
        )?;
        let vue = objet(
            &fenetre,
            jni_str!("getDecorView"),
            jni_sig!("()Landroid/view/View;"),
        )?;
        let marges = objet(
            &vue,
            jni_str!("getRootWindowInsets"),
            jni_sig!("()Landroid/view/WindowInsets;"),
        )?;
        if marges.is_null() {
            // La vue n'est pas encore attachée à sa fenêtre.
            return Ok((0, 0));
        }
        let mut hauteur = |methode| {
            env.call_method(&marges, methode, jni_sig!("()I"), &[])
                .and_then(|valeur| valeur.i())
                .map_err(|e| {
                    env.exception_clear();
                    e.to_string()
                })
        };
        Ok((
            hauteur(jni_str!("getSystemWindowInsetTop"))?,
            hauteur(jni_str!("getSystemWindowInsetBottom"))?,
        ))
    })
}

/// Marges du système en pixels logiques. L'application les demande à chaque
/// rendu : elles ne sont relues qu'au plus quatre fois par seconde.
fn marges() -> Marges {
    const VALIDITE: Duration = Duration::from_millis(250);
    static DERNIERES: Mutex<Option<(Instant, Marges)>> = Mutex::new(None);

    let mut dernieres = DERNIERES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((date, marges)) = *dernieres
        && date.elapsed() < VALIDITE
    {
        return marges;
    }
    let echelle = jni::platform()
        .and_then(|plateforme| plateforme.primary_window())
        .map_or(1., |fenetre| fenetre.scale_factor());
    let marges = match marges_systeme() {
        Ok((haut, bas)) => Marges {
            haut: haut as f32 / echelle,
            bas: bas as f32 / echelle,
        },
        Err(e) => {
            log::warn!("marges du système illisibles : {e}");
            Marges::default()
        }
    };
    *dernieres = Some((Instant::now(), marges));
    marges
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("mijot"),
    );
    // Sans lui, une panique ferme l'application sans un mot dans logcat.
    jni::install_panic_hook();

    mijot::hote::installer(Hote {
        dossier: app.internal_data_path(),
        marges: Some(marges),
    });

    jni::init_platform(&app);
    let Some(plateforme) = jni::shared_platform() else {
        log::error!("plateforme Android indisponible");
        return;
    };

    // `run` bloque sur la boucle d'événements Android ; la fermeture n'est
    // appelée qu'une fois la surface native disponible.
    Application::with_platform(plateforme.into_rc())
        .with_assets(AllAssets)
        .run(|cx: &mut App| {
            // Plein écran : pas de dimensions à proposer.
            mijot::demarrer(
                WindowOptions {
                    window_bounds: None,
                    ..Default::default()
                },
                cx,
            );
        });

    // `run` ne rend la main qu'à la destruction de l'activité. Si Android la
    // recrée ensuite (changement de taille de police, de thème système…), il
    // rappelle `android_main` sur un nouveau fil, alors que la plateforme de
    // gpui-mobile, globale au processus, reste liée à l'ancien : GPUI refuse de
    // démarrer. On quitte donc ici, et l'activité renaît dans un processus neuf.
    // Rien n'est perdu : l'état est enregistré à chaque modification.
    //
    // `_exit` plutôt que `std::process::exit` : pas de destructeurs globaux,
    // que les fils de rendu encore actifs feraient planter.
    unsafe { libc::_exit(0) }
}
