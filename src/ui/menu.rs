//! Page « Cette semaine » : réglages, bilan du budget et repas du menu.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    progress::Progress,
    switch::Switch,
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::style::{carte, entete, etat_vide, pastille, visuel_categorie};
use super::{Compteur, Mijot, libelle_repas};
use crate::plan::{cout_menu, cout_par_personne, euros};

impl Mijot {
    fn compteur(&self, compteur: Compteur, cx: &mut Context<Self>) -> Div {
        let (libelle, valeur, ids) = match compteur {
            Compteur::Personnes => (
                "Personnes",
                self.etat.personnes as usize,
                ("p-moins", "p-plus"),
            ),
            Compteur::Repas => ("Repas", self.etat.repas, ("r-moins", "r-plus")),
        };
        h_flex()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(libelle),
            )
            .child(
                Button::new(ids.0)
                    .small()
                    .outline()
                    .icon(IconName::Minus)
                    .on_click(cx.listener(move |this, _, _, cx| this.ajuster(compteur, -1, cx))),
            )
            .child(
                div()
                    .w(px(24.))
                    .text_center()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(valeur.to_string()),
            )
            .child(
                Button::new(ids.1)
                    .small()
                    .outline()
                    .icon(IconName::Plus)
                    .on_click(cx.listener(move |this, _, _, cx| this.ajuster(compteur, 1, cx))),
            )
    }

    /// Réglages du menu : ils s'appliquent au prochain « Nouveau menu ».
    fn reglages(&self, cx: &mut Context<Self>) -> Div {
        let discret = cx.theme().muted_foreground;
        carte(cx).child(
            h_flex()
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
                ),
        )
    }

    /// Où en est le budget, en une phrase et une jauge.
    fn bilan(&self, menu: &[usize], cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let discret = theme.muted_foreground;
        let cout = cout_menu(&self.cat, menu, self.etat.personnes);
        let budget = self.etat.budget;
        let depasse = cout > budget + 1e-9;
        let couleur = if depasse { theme.danger } else { theme.success };
        let portions = (menu.len() as f64 * f64::from(self.etat.personnes)).max(1.);
        let phrase = if depasse {
            format!("Le menu dépasse ton budget de {}.", euros(cout - budget))
        } else {
            format!("Il te reste {} sur ton budget.", euros(budget - cout))
        };
        let part = if budget > 0. {
            (cout / budget * 100.).min(100.) as f32
        } else {
            100.
        };
        carte(cx)
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_end()
                    .flex_wrap()
                    .gap_3()
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(discret)
                                    .child("Coût estimé de la semaine"),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_end()
                                    .child(
                                        div()
                                            .text_3xl()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(couleur)
                                            .child(euros(cout)),
                                    )
                                    .child(
                                        div()
                                            .pb_1()
                                            .text_sm()
                                            .text_color(discret)
                                            .child(format!("sur {}", euros(budget))),
                                    ),
                            ),
                    )
                    .child(
                        v_flex()
                            .items_end()
                            .child(div().text_sm().child(phrase))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(discret)
                                    .child(format!("Soit {} par assiette", euros(cout / portions))),
                            ),
                    ),
            )
            .child(Progress::new("budget").value(part).color(couleur))
    }

    fn carte_repas(
        &self,
        position: usize,
        total: usize,
        recette: usize,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = cx.theme();
        let (discret, primaire, bordure) = (theme.muted_foreground, theme.primary, theme.border);
        let r = &self.cat.recettes[recette];
        let id = r.id;
        let (icone, teinte) = visuel_categorie(r.categorie);
        let par_personne = cout_par_personne(&self.cat, recette);
        let ouverte = self.detail == Some(id);
        carte(cx)
            .id(("carte", position))
            .w(px(272.))
            .gap_3()
            .cursor_pointer()
            .when(ouverte, |c| c.border_color(primaire))
            .when(!ouverte, |c| {
                c.hover(move |c| c.border_color(primaire.opacity(0.45)))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.detail = Some(id);
                cx.notify();
            }))
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(pastille(icone, teinte, 44., cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                // Le jour, puis l'origine du plat si elle est connue.
                                h_flex()
                                    .gap_1()
                                    .text_xs()
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(primaire)
                                            .child(libelle_repas(position, total)),
                                    )
                                    .when(!r.origine.is_empty(), |ligne| {
                                        ligne.child(
                                            div()
                                                .min_w_0()
                                                .truncate()
                                                .text_color(discret)
                                                .child(format!("· {}", r.origine)),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .h(px(44.))
                                    .line_height(px(22.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .line_clamp(2)
                                    .text_ellipsis()
                                    .child(r.nom.clone()),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .text_xs()
                    .text_color(discret)
                    .child(Icon::new(IconName::Clock).xsmall())
                    .child(format!("{} min · {}", r.minutes, r.categorie.nom()))
                    .when(r.vege, |ligne| {
                        ligne.child(Tag::success().small().child("Végé"))
                    }),
            )
            .child(
                h_flex()
                    .justify_between()
                    .items_end()
                    .pt_2()
                    .border_t_1()
                    .border_color(bordure)
                    .child(
                        v_flex()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child(format!(
                                "≈ {}",
                                euros(par_personne * f64::from(self.etat.personnes))
                            )))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(discret)
                                    .child(format!("{} par personne", euros(par_personne))),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new(("changer", position))
                                    .small()
                                    .outline()
                                    .icon(IconName::RefreshCw)
                                    .label("Changer")
                                    .tooltip("Proposer un autre plat qui tient dans le budget")
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
            )
    }

    pub(super) fn page_menu(&self, cx: &mut Context<Self>) -> Div {
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
                h_flex()
                    .items_start()
                    .gap_3()
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
        let mut cartes = h_flex().flex_wrap().items_stretch().gap_4();
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
