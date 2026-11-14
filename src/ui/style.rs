//! Identité visuelle : palette sobre (gris neutres, un seul accent terracotta),
//! profondeur par ombres légères, pictogrammes des catégories et des rayons,
//! briques de mise en page communes aux écrans.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, Size, Theme, ThemeColor, h_flex, v_flex,
};
use gpui_kit::*;

use crate::donnees::{Categorie, Rayon};

/// Couleur opaque : teinte en degrés, saturation et luminosité en pour cent.
fn couleur(teinte: f32, saturation: f32, luminosite: f32) -> Hsla {
    hsla(teinte / 360., saturation / 100., luminosite / 100., 1.)
}

/// Fond et couleur principale de la palette, selon le mode du thème.
fn reperes(sombre: bool) -> (Hsla, Hsla) {
    if sombre {
        (couleur(230., 8., 10.), couleur(14., 92., 63.))
    } else {
        (couleur(0., 0., 100.), couleur(14., 84., 52.))
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
        theme.radius = px(10.);
        theme.radius_lg = px(16.);
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
        // Gris froids très sombres, trois paliers nets : fenêtre, feuille, carte.
        c.foreground = couleur(220., 14., 94.);
        c.muted = couleur(230., 8., 17.);
        c.muted_foreground = couleur(225., 8., 64.);
        c.border = couleur(230., 8., 20.);
        c.input = couleur(230., 8., 28.);
        c.group_box = couleur(230., 8., 13.5);
        c.primary_hover = couleur(14., 92., 68.);
        c.primary_active = couleur(14., 92., 58.);
        c.primary_foreground = couleur(230., 20., 8.);
        c.accent = couleur(230., 8., 19.);
        c.secondary = couleur(230., 8., 18.);
        c.secondary_hover = couleur(230., 8., 21.5);
        c.secondary_active = couleur(230., 8., 25.);
        c.sidebar = couleur(230., 10., 6.5);
        c.success = couleur(152., 60., 52.);
        c.danger = couleur(2., 85., 66.);
    } else {
        c.foreground = couleur(225., 15., 13.);
        c.muted = couleur(220., 14., 96.);
        c.muted_foreground = couleur(220., 8., 43.);
        c.border = couleur(220., 13., 91.);
        c.input = couleur(220., 12., 84.);
        c.group_box = couleur(0., 0., 100.);
        c.primary_hover = couleur(14., 84., 47.);
        c.primary_active = couleur(14., 84., 43.);
        c.primary_foreground = couleur(0., 0., 100.);
        c.accent = couleur(220., 14., 94.);
        c.secondary = couleur(220., 14., 95.);
        c.secondary_hover = couleur(220., 14., 92.);
        c.secondary_active = couleur(220., 14., 89.);
        c.sidebar = couleur(220., 16., 95.);
        c.success = couleur(152., 62., 36.);
        c.danger = couleur(2., 72., 50.);
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
        (hsla(h, 0.70, 0.60, 0.18), hsla(h, 0.80, 0.76, 1.))
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

/// Dégradé en diagonale d'une teinte : bandeau des cartes de repas.
pub(super) fn degrade(teinte: f32, cx: &App) -> Background {
    let (h, h2) = (teinte / 360., ((teinte + 16.) % 360.) / 360.);
    // En sombre : une lueur de la teinte qui s'éteint dans le fond de la carte,
    // plutôt qu'un aplat foncé. En clair : un pastel léger.
    let (debut, fin) = if cx.theme().is_dark() {
        (hsla(h, 0.80, 0.60, 0.30), hsla(h, 0.80, 0.60, 0.05))
    } else {
        (hsla(h, 0.80, 0.95, 1.), hsla(h2, 0.75, 0.90, 1.))
    };
    linear_gradient(
        135.,
        linear_color_stop(debut, 0.),
        linear_color_stop(fin, 1.),
    )
}

/// Couleur de trait d'une teinte, lisible sur son dégradé ou son fond teinté.
pub(super) fn trait_teinte(teinte: f32, cx: &App) -> Hsla {
    tons(teinte, cx).1
}

/// Ombre douce et diffuse, posée sous les surfaces flottantes.
pub(super) fn ombre(cx: &App) -> Vec<BoxShadow> {
    let force = if cx.theme().is_dark() { 0.35 } else { 0.06 };
    vec![
        BoxShadow::new(px(0.), px(1.), hsla(0., 0., 0., force)).blur_radius(px(2.)),
        BoxShadow::new(px(0.), px(6.), hsla(0., 0., 0., force * 0.7)).blur_radius(px(18.)),
    ]
}

/// Carte : fond clair, bordure à peine visible, grands coins arrondis.
pub(super) fn carte(cx: &App) -> Div {
    let theme = cx.theme();
    v_flex()
        .p_4()
        .rounded(theme.radius_lg)
        .border_1()
        .border_color(theme.border)
        .bg(theme.group_box)
}

/// Feuille : grande surface flottante posée sur le fond de la fenêtre. Elle
/// porte le contenu de la page ou un volet.
pub(super) fn feuille(cx: &App) -> Div {
    let theme = cx.theme();
    v_flex()
        .rounded(px(18.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .shadow(ombre(cx))
}

/// Bandeau sans bordure, sur fond discret : réglages, barres d'outils.
pub(super) fn bandeau(cx: &App) -> Div {
    let theme = cx.theme();
    h_flex()
        .px_4()
        .py_3()
        .rounded(theme.radius_lg)
        .bg(theme.muted)
}

/// Pilule d'état : un point de couleur et un libellé court.
pub(super) fn pilule(texte: impl Into<SharedString>, couleur: Hsla) -> Div {
    h_flex()
        .flex_shrink_0()
        .gap_1p5()
        .px_2p5()
        .py_0p5()
        .rounded_full()
        .bg(couleur.opacity(0.12))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(couleur)
        .child(div().size(px(6.)).rounded_full().bg(couleur))
        .child(texte.into())
}

/// Jauge épaisse aux bouts ronds, remplie à `part` pour cent.
pub(super) fn jauge(part: f32, couleur: Hsla, cx: &App) -> Div {
    div()
        .h(px(8.))
        .w_full()
        .rounded_full()
        .bg(cx.theme().muted)
        .child(
            div()
                .h_full()
                .w(relative(part.clamp(0., 100.) / 100.))
                .rounded_full()
                .bg(couleur),
        )
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
                .text_size(px(28.))
                .line_height(px(34.))
                .font_weight(FontWeight::BOLD)
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

/// Volet de droite : feuille défilante, à côté de celle du contenu.
pub(super) fn volet(id: &'static str, largeur: f32, cx: &App) -> Stateful<Div> {
    feuille(cx)
        .id(id)
        .w(px(largeur))
        .flex_shrink_0()
        .overflow_y_scroll()
        .p_5()
        .gap_4()
}

/// En-tête d'un volet : pastille, titre, précision, bouton de fermeture.
pub(super) fn entete_volet(
    pastille: Div,
    titre: impl Into<SharedString>,
    precision: impl Into<SharedString>,
    fermer: impl IntoElement,
    cx: &App,
) -> Div {
    div()
        .flex()
        .items_start()
        .gap_3()
        .child(pastille)
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(titre.into()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(precision.into()),
                ),
        )
        .child(fermer)
}

/// Petit titre au-dessus d'une section ou d'un champ.
pub(super) fn titre_section(texte: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(cx.theme().muted_foreground)
        .child(texte.into())
}

/// Chiffre clé : un libellé discret au-dessus d'une valeur.
pub(super) fn tuile(
    libelle: impl Into<SharedString>,
    valeur: impl Into<SharedString>,
    cx: &App,
) -> Div {
    v_flex()
        .flex_1()
        .min_w_0()
        .px_3()
        .py_2p5()
        .gap_0p5()
        .rounded(cx.theme().radius)
        .bg(cx.theme().muted)
        .child(titre_section(libelle, cx))
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .child(valeur.into()),
        )
}

/// Numéro d'étape dans un rond teinté.
pub(super) fn numero(n: usize, cx: &App) -> Div {
    let primaire = cx.theme().primary;
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(px(22.))
        .rounded_full()
        .bg(primaire.opacity(0.14))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(primaire)
        .child(n.to_string())
}
