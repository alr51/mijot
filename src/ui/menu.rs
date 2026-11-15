//! Page « Cette semaine » : réglages, bilan du budget et repas du menu.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, Size,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    switch::Switch,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::style::{
    bandeau, carte, degrade, entete, etat_vide, haut_de_page, jauge, ombre, pastille_score, pilule,
    trait_teinte, tuile, tuiles, visuel_categorie,
};
use super::{Compteur, Mijot, libelle_repas};
use crate::nutrition;
use crate::plan::{cout_menu, cout_par_personne, euros};

impl Mijot {
    /// Compteur : libellé, puis moins, valeur et plus réunis dans une pilule.
    fn compteur(&self, compteur: Compteur, cx: &mut Context<Self>) -> Div {
        let (libelle, valeur, ids) = match compteur {
            Compteur::Personnes => (
                "Personnes",
                self.etat.personnes as usize,
                ("p-moins", "p-plus"),
            ),
            Compteur::Repas => ("Repas", self.etat.repas, ("r-moins", "r-plus")),
        };
        let theme = cx.theme();
        let (discret, surface, bordure) = (theme.muted_foreground, theme.group_box, theme.border);
        h_flex()
            .gap_2()
            .child(div().text_sm().text_color(discret).child(libelle))
            .child(
                h_flex()
                    .p_0p5()
                    .rounded_full()
                    .border_1()
                    .border_color(bordure)
                    .bg(surface)
                    .child(
                        Button::new(ids.0)
                            .small()
                            .ghost()
                            .rounded_full()
                            .icon(IconName::Minus)
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.ajuster(compteur, -1, cx)),
                            ),
                    )
                    .child(
                        div()
                            .w(px(28.))
                            .text_center()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(valeur.to_string()),
                    )
                    .child(
                        Button::new(ids.1)
                            .small()
                            .ghost()
                            .rounded_full()
                            .icon(IconName::Plus)
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.ajuster(compteur, 1, cx)),
                            ),
                    ),
            )
    }

    /// Réglages du menu : ils s'appliquent au prochain « Nouveau menu ».
    fn reglages(&self, cx: &mut Context<Self>) -> Div {
        let discret = cx.theme().muted_foreground;
        bandeau(cx)
            .flex_wrap()
            .gap_x_8()
            .gap_y_3()
            .child(
                h_flex()
                    .gap_2()
                    .child(div().text_sm().text_color(discret).child("Budget"))
                    .child(div().w(px(88.)).child(Input::new(&self.budget)))
                    .child(div().text_sm().text_color(discret).child("€ / semaine")),
            )
            .child(self.compteur(Compteur::Personnes, cx))
            .child(self.compteur(Compteur::Repas, cx))
            .child(
                Switch::new("vege")
                    .checked(self.etat.vege)
                    .label("Végétarien")
                    .on_click(cx.listener(|this, on: &bool, _, cx| {
                        this.etat.vege = *on;
                        this.avis = None;
                        this.sauver(cx);
                    })),
            )
    }

    /// Où en est le budget : le coût en grand, une jauge, trois chiffres clés.
    fn bilan(&self, menu: &[usize], cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let discret = theme.muted_foreground;
        let cout = cout_menu(&self.cat, menu, self.etat.personnes);
        let budget = self.etat.budget;
        let depasse = cout > budget + 1e-9;
        let couleur = if depasse { theme.danger } else { theme.success };
        let portions = (menu.len() as f64 * f64::from(self.etat.personnes)).max(1.);
        let part = if budget > 0. {
            (cout / budget * 100.).min(100.) as f32
        } else {
            100.
        };
        let (libelle_ecart, ecart) = if depasse {
            ("Dépassement", euros(cout - budget))
        } else {
            ("Reste", euros(budget - cout))
        };
        carte(cx).p_5().shadow(ombre(cx)).child(
            h_flex()
                .flex_wrap()
                .items_end()
                .gap_x_8()
                .gap_y_4()
                .child(
                    v_flex()
                        .flex_1()
                        .min_w(px(if self.compact { 0. } else { 280. }))
                        .gap_3()
                        .child(
                            h_flex()
                                .gap_3()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(discret)
                                        .child("Coût estimé de la semaine"),
                                )
                                .child(pilule(
                                    if depasse {
                                        "Budget dépassé"
                                    } else {
                                        "Dans le budget"
                                    },
                                    couleur,
                                )),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .items_end()
                                .child(
                                    div()
                                        .text_size(px(40.))
                                        .line_height(px(44.))
                                        .font_weight(FontWeight::BOLD)
                                        .child(euros(cout)),
                                )
                                .child(
                                    div()
                                        .pb_1p5()
                                        .text_color(discret)
                                        .child(format!("sur {}", euros(budget))),
                                ),
                        )
                        .child(jauge(part, couleur, cx)),
                )
                .child(
                    tuiles(self.compact)
                        .w(px(440.))
                        .max_w_full()
                        .child(tuile(libelle_ecart, ecart, cx))
                        .child(tuile("Par assiette", euros(cout / portions), cx))
                        .child(tuile(
                            "Énergie par assiette",
                            nutrition::moyenne_menu(&self.cat, menu)
                                .map_or("—".to_owned(), |moyenne| nutrition::kcal(moyenne.kcal)),
                            cx,
                        )),
                ),
        )
    }

    fn carte_repas(
        &self,
        position: usize,
        total: usize,
        recette: usize,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = cx.theme();
        let (discret, primaire, bordure, surface, rayon) = (
            theme.muted_foreground,
            theme.primary,
            theme.border,
            theme.group_box,
            theme.radius_lg,
        );
        let r = &self.cat.recettes[recette];
        let id = r.id;
        let (icone, teinte) = visuel_categorie(r.categorie);
        let trait_ = trait_teinte(teinte, cx);
        let par_personne = cout_par_personne(&self.cat, recette);
        let ouverte = self.detail == Some(id);
        let ombre_survol = ombre(cx);
        // Origine, durée et catégorie sur une seule ligne discrète.
        let mut precisions = Vec::new();
        if !r.origine.is_empty() {
            precisions.push(r.origine.clone());
        }
        precisions.push(format!("{} min", r.minutes));
        // La catégorie se lit déjà dans le bandeau : place à l'énergie.
        let bilan = nutrition::bilan(&self.cat, recette);
        match &bilan {
            Some(bilan) => precisions.push(nutrition::kcal(bilan.par_personne.kcal)),
            None => precisions.push(r.categorie.nom().to_owned()),
        }
        v_flex()
            .id(("carte", position))
            .min_w_0()
            .rounded(rayon)
            .border_1()
            .border_color(bordure)
            .bg(surface)
            .cursor_pointer()
            .when(ouverte, |c| c.border_color(primaire).shadow(ombre(cx)))
            .when(!ouverte, |c| {
                c.hover(move |c| c.border_color(primaire.opacity(0.45)).shadow(ombre_survol))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.detail = Some(id);
                cx.notify();
            }))
            .child(
                // Bandeau aux couleurs de la catégorie : le jour et son pictogramme.
                h_flex()
                    .h(px(72.))
                    .px_4()
                    .justify_between()
                    .rounded_t(rayon - px(1.))
                    .bg(degrade(teinte, cx))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .px_2p5()
                                    .py_0p5()
                                    .rounded_full()
                                    .bg(surface.opacity(0.75))
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(trait_)
                                    .child(libelle_repas(position, total)),
                            )
                            .children(bilan.map(|bilan| pastille_score(bilan.score))),
                    )
                    .child(
                        Icon::new(icone)
                            .with_size(Size::Size(px(34.)))
                            .text_color(trait_.opacity(0.75)),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .p_4()
                    .gap_1()
                    .child(
                        div()
                            .h(px(44.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .line_clamp(2)
                            .text_ellipsis()
                            .child(r.nom.clone()),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .text_xs()
                            .text_color(discret)
                            .child(div().min_w_0().truncate().child(precisions.join(" · ")))
                            .when(r.vege, |ligne| {
                                ligne.child(pilule("Végé", cx.theme().success))
                            }),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_end()
                            .pt_3()
                            .child(
                                v_flex()
                                    .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(
                                        euros(par_personne * f64::from(self.etat.personnes)),
                                    ))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(discret)
                                            .child(format!("{} par personne", euros(par_personne))),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .child(
                                        Button::new(("changer", position))
                                            .small()
                                            .ghost()
                                            .icon(IconName::RefreshCw)
                                            .label("Changer")
                                            .tooltip(
                                                "Proposer un autre plat qui tient dans le budget",
                                            )
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                this.changer(position, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new(("retirer", position))
                                            .small()
                                            .ghost()
                                            .icon(IconName::X)
                                            .tooltip("Retirer du menu")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                this.retirer(position, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
    }

    /// `colonnes` : nombre de cartes de repas par rangée.
    pub(super) fn page_menu(&self, colonnes: u16, cx: &mut Context<Self>) -> Div {
        let menu = self.etat.menu_indices(&self.cat);
        let discret = cx.theme().muted_foreground;
        let personnes = match self.etat.personnes {
            1 => "1 personne".to_owned(),
            n => format!("{n} personnes"),
        };
        let sous_titre = if menu.is_empty() {
            "Compose un menu qui tient dans ton budget.".to_owned()
        } else {
            format!("{} repas pour {personnes}", menu.len())
        };
        let mut page = v_flex()
            .gap_5()
            .child(
                haut_de_page(self.compact)
                    .child(entete("Cette semaine", sous_titre, cx))
                    .child(
                        Button::new("generer")
                            .primary()
                            .icon(IconName::Sparkles)
                            .label(if menu.is_empty() {
                                "Générer le menu"
                            } else {
                                "Nouveau menu"
                            })
                            .on_click(cx.listener(|this, _, _, cx| this.generer(cx))),
                    ),
            )
            .child(self.reglages(cx));
        if menu.is_empty() {
            return page.child(etat_vide(
                IconName::Utensils,
                "Pas encore de menu",
                "Indique ton budget, le nombre de personnes et de repas, puis clique sur « Générer le menu ».",
                cx,
            ));
        }
        page = page.child(self.bilan(&menu, cx));
        let total = menu.len();
        let mut cartes = div().grid().grid_cols(colonnes).gap_4();
        for (position, &recette) in menu.iter().enumerate() {
            cartes = cartes.child(self.carte_repas(position, total, recette, cx));
        }
        page.child(cartes).child(
            div().text_xs().text_color(discret).child(
                "Prix au prorata des quantités utilisées, d'après l'onglet « Ingrédients et prix » : un repère pour tenir le budget, pas un ticket de caisse.",
            ),
        )
    }
}
