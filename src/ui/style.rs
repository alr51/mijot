//! Identité visuelle : palette chaleureuse (crème et terracotta), pictogrammes
//! des catégories et des rayons, briques de mise en page communes aux écrans.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, Size, Theme, ThemeColor, v_flex};
use gpui_kit::*;

use crate::donnees::{Categorie, Rayon};

/// Couleur opaque : teinte en degrés, saturation et luminosité en pour cent.
fn couleur(teinte: f32, saturation: f32, luminosite: f32) -> Hsla {
    hsla(teinte / 360., saturation / 100., luminosite / 100., 1.)
}

/// Fond et couleur principale de la palette, selon le mode du thème.
fn reperes(sombre: bool) -> (Hsla, Hsla) {
    if sombre {
        (couleur(24., 10., 9.5), couleur(18., 85., 60.))
    } else {
        (couleur(40., 40., 97.5), couleur(16., 78., 50.))
    }
}

/// Vrai si le thème courant porte déjà la palette de l'application.
///
/// Le thème est rechargé, donc remis aux couleurs neutres, à chaque bascule
/// clair/sombre, qu'elle vienne du bouton ou du système : la fenêtre le
/// vérifie à chaque rendu et repose la palette au besoin.
pub(super) fn palette_en_place(cx: &App) -> bool {
    let theme = cx.theme();
    let (fond, primaire) = reperes(theme.is_dark());
    theme.background == fond && theme.primary == primaire
}

/// Remplace les couleurs neutres du thème par la palette de l'application.
/// Sans effet si elle est déjà en place.
pub fn appliquer_palette(cx: &mut App) {
    if palette_en_place(cx) {
        return;
    }
    let sombre = cx.theme().is_dark();
    let (fond, primaire) = reperes(sombre);
    // `Theme::update` garde en phase les couleurs, les jetons que lisent les
    // composants (boutons…) et rafraîchit les fenêtres.
    Theme::update(cx, |theme| {
        theme.radius = px(8.);
        theme.radius_lg = px(14.);
        peindre(&mut theme.colors, sombre, fond, primaire);
    });
}

fn peindre(c: &mut ThemeColor, sombre: bool, fond: Hsla, primaire: Hsla) {
    c.background = fond;
    c.primary = primaire;
    c.button_primary = primaire;
    c.ring = primaire;
    c.progress_bar = primaire;
    c.link = primaire;
    c.caret = primaire;
    c.selection = primaire.opacity(0.25);
    c.list_active = primaire.opacity(0.12);
    if sombre {
        c.foreground = couleur(35., 25., 92.);
        c.muted = couleur(24., 8., 16.);
        c.muted_foreground = couleur(30., 10., 64.);
        c.border = couleur(24., 8., 21.);
        c.input = couleur(24., 8., 27.);
        c.group_box = couleur(24., 9., 13.5);
        c.primary_hover = couleur(18., 85., 65.);
        c.primary_active = couleur(18., 85., 55.);
        c.primary_foreground = couleur(24., 30., 8.);
        c.accent = couleur(22., 22., 19.);
        c.secondary = couleur(24., 8., 18.);
        c.secondary_hover = couleur(24., 8., 21.);
        c.secondary_active = couleur(24., 8., 24.);
        c.sidebar = couleur(24., 11., 7.5);
    } else {
        c.foreground = couleur(25., 25., 14.);
        c.muted = couleur(38., 30., 93.);
        c.muted_foreground = couleur(28., 10., 40.);
        c.border = couleur(35., 25., 87.);
        c.input = couleur(35., 22., 80.);
        c.group_box = couleur(0., 0., 100.);
        c.primary_hover = couleur(16., 78., 45.);
        c.primary_active = couleur(16., 78., 41.);
        c.primary_foreground = couleur(0., 0., 100.);
        c.accent = couleur(26., 75., 93.);
        c.secondary = couleur(36., 35., 92.);
        c.secondary_hover = couleur(36., 35., 89.);
        c.secondary_active = couleur(36., 35., 86.);
        c.sidebar = couleur(38., 38., 95.);
    }
    c.accent_foreground = c.foreground;
    c.secondary_foreground = c.foreground;
    c.sidebar_foreground = c.foreground;
    c.sidebar_border = c.border;
    c.list_hover = c.accent;
    c.button_primary_hover = c.primary_hover;
    c.button_primary_active = c.primary_active;
    c.button_primary_foreground = c.primary_foreground;
}

/// Pictogramme et teinte (en degrés) d'une catégorie de recettes.
pub(super) fn visuel_categorie(categorie: Categorie) -> (IconName, f32) {
    match categorie {
        Categorie::Pates => (IconName::Wheat, 40.),
        Categorie::Riz => (IconName::Sprout, 95.),
        Categorie::Four => (IconName::Flame, 14.),
        Categorie::Mijote => (IconName::CookingPot, 330.),
        Categorie::Soupe => (IconName::Soup, 175.),
        Categorie::Poisson => (IconName::Fish, 210.),
        Categorie::Express => (IconName::Zap, 262.),
    }
}

/// Pictogramme et teinte (en degrés) d'un rayon.
pub(super) fn visuel_rayon(rayon: Rayon) -> (IconName, f32) {
    match rayon {
        Rayon::FruitsLegumes => (IconName::Apple, 110.),
        Rayon::ViandesPoissons => (IconName::Beef, 0.),
        Rayon::Frais => (IconName::Milk, 205.),
        Rayon::Epicerie => (IconName::Package, 35.),
        Rayon::Boulangerie => (IconName::Croissant, 48.),
    }
}

/// Fond translucide et couleur de trait d'une teinte, lisibles dans les deux thèmes.
fn tons(teinte: f32, cx: &App) -> (Hsla, Hsla) {
    let h = teinte / 360.;
    if cx.theme().is_dark() {
        (hsla(h, 0.55, 0.55, 0.20), hsla(h, 0.70, 0.72, 1.))
    } else {
        (hsla(h, 0.75, 0.50, 0.13), hsla(h, 0.70, 0.36, 1.))
    }
}

/// Carré arrondi teinté portant un pictogramme, de `cote` pixels de côté.
pub(super) fn pastille(icone: IconName, teinte: f32, cote: f32, cx: &App) -> Div {
    let (fond, trait_) = tons(teinte, cx);
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(px(cote))
        .rounded(px(cote * 0.28))
        .bg(fond)
        .child(
            Icon::new(icone)
                .with_size(Size::Size(px(cote * 0.5)))
                .text_color(trait_),
        )
}

/// Carte : fond clair, bordure fine, coins arrondis.
pub(super) fn carte(cx: &App) -> Div {
    let theme = cx.theme();
    v_flex()
        .p_4()
        .rounded(theme.radius_lg)
        .border_1()
        .border_color(theme.border)
        .bg(theme.group_box)
}

/// Titre de page et phrase d'accompagnement.
pub(super) fn entete(
    titre: impl Into<SharedString>,
    sous_titre: impl Into<SharedString>,
    cx: &App,
) -> Div {
    v_flex()
        .flex_1()
        .min_w_0()
        .gap_1()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::SEMIBOLD)
                .child(titre.into()),
        )
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(sous_titre.into()),
        )
}

/// Message centré pour une page ou une liste sans contenu.
pub(super) fn etat_vide(
    icone: IconName,
    titre: impl Into<SharedString>,
    texte: impl Into<SharedString>,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    v_flex()
        .items_center()
        .gap_2()
        .py_12()
        .child(
            Icon::new(icone)
                .with_size(Size::Size(px(40.)))
                .text_color(theme.muted_foreground.opacity(0.6)),
        )
        .child(
            div()
                .pt_2()
                .font_weight(FontWeight::SEMIBOLD)
                .child(titre.into()),
        )
        .child(
            div()
                .max_w(px(420.))
                .text_sm()
                .text_center()
                .text_color(theme.muted_foreground)
                .child(texte.into()),
        )
}
