//! Page « Liste de courses » : progression du panier, articles par rayon,
//! articles ajoutés à la main.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Sizable as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::Input,
    progress::Progress,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::Mijot;
use super::style::{carte, entete, pastille, visuel_rayon};
use crate::plan::{euros, liste_de_courses, quantite_lisible};

impl Mijot {
    pub(super) fn page_courses(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let (discret, bordure, succes) = (theme.muted_foreground, theme.border, theme.success);
        let menu = self.etat.menu_indices(&self.cat);
        let liste = liste_de_courses(&self.cat, &menu, self.etat.personnes);
        let total: f64 = liste.iter().flat_map(|(_, l)| l).map(|l| l.prix).sum();
        let articles = liste.iter().map(|(_, l)| l.len()).sum::<usize>() + self.etat.extras.len();
        let coches = self.etat.coches.len() + self.etat.extras.iter().filter(|e| e.coche).count();
        let personnes = match self.etat.personnes {
            1 => "1 personne".to_owned(),
            n => format!("{n} personnes"),
        };

        let mut page = v_flex().max_w(px(760.)).gap_5().child(
            h_flex()
                .items_start()
                .gap_3()
                .child(entete(
                    "Liste de courses",
                    format!("Pour {personnes} · total estimé {}", euros(total)),
                    cx,
                ))
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("decocher")
                                .ghost()
                                .label("Tout décocher")
                                .disabled(coches == 0)
                                .on_click(cx.listener(|this, _, _, cx| this.tout_decocher(cx))),
                        )
                        .child(
                            Button::new("copier")
                                .outline()
                                .icon(IconName::Copy)
                                .label("Copier la liste")
                                .disabled(articles == 0)
                                .on_click(cx.listener(|this, _, _, cx| this.copier(cx))),
                        ),
                ),
        );

        if articles > 0 {
            let fini = coches == articles;
            let phrase = if fini {
                "Tout est dans le panier, bon appétit !".to_owned()
            } else if coches == 0 {
                format!("{articles} articles à trouver : coche-les au fil des rayons.")
            } else {
                format!("{coches} sur {articles} articles dans le panier")
            };
            page = page.child(
                carte(cx)
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .text_sm()
                            .when(fini, |ligne| {
                                ligne
                                    .text_color(succes)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(Icon::new(IconName::CircleCheck).small())
                            })
                            .child(phrase),
                    )
                    .child(
                        Progress::new("panier")
                            .value(coches as f32 / articles as f32 * 100.)
                            .when(fini, |jauge| jauge.color(succes)),
                    ),
            );
        }

        if liste.is_empty() {
            page = page.child(
                div()
                    .text_sm()
                    .text_color(discret)
                    .child("Le menu est vide : génère un menu pour remplir la liste."),
            );
        }

        for (rayon, lignes) in &liste {
            let sous_total: f64 = lignes.iter().map(|l| l.prix).sum();
            let (icone, teinte) = visuel_rayon(*rayon);
            let mut section = carte(cx).gap_1().child(
                h_flex()
                    .gap_3()
                    .pb_2()
                    .child(pastille(icone, teinte, 32., cx))
                    .child(
                        div()
                            .flex_1()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(rayon.nom()),
                    )
                    .child(div().text_sm().text_color(discret).child(euros(sous_total))),
            );
            for ligne in lignes {
                let i = &self.cat.ingredients[ligne.ingredient];
                let id = i.id;
                let coche = self.etat.coches.contains(&id);
                section = section.child(
                    h_flex()
                        .justify_between()
                        .py_1p5()
                        .border_t_1()
                        .border_color(bordure)
                        .when(coche, |row| row.opacity(0.45))
                        .child(
                            Checkbox::new(("ingredient", ligne.ingredient))
                                .checked(coche)
                                .label(i.nom.clone())
                                .on_click(cx.listener(move |this, on: &bool, _, cx| {
                                    this.cocher(id, *on, cx)
                                })),
                        )
                        .child(
                            h_flex()
                                .gap_4()
                                .text_sm()
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(quantite_lisible(ligne.quantite, i.unite)),
                                )
                                .child(
                                    div()
                                        .w(px(64.))
                                        .text_right()
                                        .text_color(discret)
                                        .child(euros(ligne.prix)),
                                ),
                        ),
                );
            }
            page = page.child(section);
        }

        let mut autres = carte(cx).gap_1().child(
            h_flex()
                .gap_3()
                .pb_2()
                .child(pastille(IconName::ShoppingBasket, 262., 32., cx))
                .child(
                    v_flex()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Autres articles"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(discret)
                                .child("Ce qui n'est pas dans les recettes : café, lessive…"),
                        ),
                ),
        );
        for (position, extra) in self.etat.extras.iter().enumerate() {
            autres = autres.child(
                h_flex()
                    .justify_between()
                    .py_1()
                    .border_t_1()
                    .border_color(bordure)
                    .child(
                        div().when(extra.coche, |d| d.opacity(0.45)).child(
                            Checkbox::new(("extra", position))
                                .checked(extra.coche)
                                .label(extra.nom.clone())
                                .on_click(cx.listener(move |this, on: &bool, _, cx| {
                                    if let Some(e) = this.etat.extras.get_mut(position) {
                                        e.coche = *on;
                                        this.sauver(cx);
                                    }
                                })),
                        ),
                    )
                    .child(
                        Button::new(("supprimer-extra", position))
                            .small()
                            .ghost()
                            .icon(IconName::Trash)
                            .tooltip("Supprimer")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if position < this.etat.extras.len() {
                                    this.etat.extras.remove(position);
                                    this.sauver(cx);
                                }
                            })),
                    ),
            );
        }
        autres = autres.child(
            h_flex()
                .gap_2()
                .pt_2()
                .child(div().flex_1().child(Input::new(&self.extra)))
                .child(
                    Button::new("ajouter-extra")
                        .outline()
                        .icon(IconName::Plus)
                        .label("Ajouter")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.ajouter_extra(window, cx)),
                        ),
                ),
        );

        page.child(autres).child(
            div().text_xs().text_color(discret).child(
                "Quantités cumulées de tous les repas ; les articles à la pièce sont arrondis à l'unité supérieure.",
            ),
        )
    }
}
