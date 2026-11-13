//! Onglet « Ingrédients et prix » : prix modifiables, ajout et suppression d'ingrédients.

use std::collections::HashMap;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    select::{Select, SelectEvent, SelectState},
    v_flex,
};
use gpui_kit::*;

use super::style::{carte, entete, pastille, visuel_rayon};
use super::{Liste, Mijot, ecrire_nombre, lire_nombre, liste, selecteur};
use crate::catalogue::Catalogue;
use crate::donnees::{Rayon, Unite};

/// Champs de l'onglet. Chaque ingrédient a son champ de prix, enregistré à la frappe.
pub(super) struct Prix {
    filtre: Entity<InputState>,
    /// Champ de prix de chaque ingrédient, par identifiant, avec son abonnement.
    champs: HashMap<i64, (Entity<InputState>, Subscription)>,
    nom: Entity<InputState>,
    prix: Entity<InputState>,
    rayon: Entity<SelectState<Liste<Rayon>>>,
    unite: Entity<SelectState<Liste<Unite>>>,
    _abonnements: Vec<Subscription>,
}

impl Prix {
    pub(super) fn new(cat: &Catalogue, window: &mut Window, cx: &mut Context<Mijot>) -> Self {
        let filtre =
            cx.new(|cx| InputState::new(window, cx).placeholder("Chercher un ingrédient…"));
        let nom = cx.new(|cx| InputState::new(window, cx).placeholder("Nom du nouvel ingrédient"));
        let prix = cx.new(|cx| InputState::new(window, cx).placeholder("Prix"));
        let rayon = selecteur(
            liste(Rayon::TOUS.map(|r| (r.nom().to_owned(), r))),
            Some(&Rayon::Epicerie),
            false,
            window,
            cx,
        );
        let unite = selecteur(
            liste(Unite::TOUTES.map(|u| (u.nom().to_owned(), u))),
            Some(&Unite::G),
            false,
            window,
            cx,
        );
        let abonnements = vec![
            cx.subscribe(&filtre, |_, _, _: &InputEvent, cx| cx.notify()),
            // L'unité choisie change le libellé du prix (« € / kg », « € / pièce »…).
            cx.subscribe(&unite, |_, _, _: &SelectEvent<Liste<Unite>>, cx| {
                cx.notify()
            }),
            cx.subscribe_in(&prix, window, |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    this.ajouter_ingredient(window, cx);
                }
            }),
        ];
        let champs = cat
            .ingredients
            .iter()
            .map(|i| (i.id, champ_prix(i.id, i.prix, window, cx)))
            .collect();
        Self {
            filtre,
            champs,
            nom,
            prix,
            rayon,
            unite,
            _abonnements: abonnements,
        }
    }
}

fn champ_prix(
    ingredient: i64,
    prix: f64,
    window: &mut Window,
    cx: &mut Context<Mijot>,
) -> (Entity<InputState>, Subscription) {
    let champ = cx.new(|cx| InputState::new(window, cx).default_value(ecrire_nombre(prix)));
    let abonnement = cx.subscribe(&champ, move |this, champ, event: &InputEvent, cx| {
        if let InputEvent::Change = event {
            let texte = champ.read(cx).value();
            this.changer_prix(ingredient, &texte, cx);
        }
    });
    (champ, abonnement)
}

impl Mijot {
    /// Enregistre le prix dès qu'il est lisible ; une saisie en cours (« 2, ») est ignorée.
    /// Écrit un prix dans le champ d'un ingrédient, comme une saisie.
    pub(super) fn definir_champ_prix(
        &mut self,
        ingredient: i64,
        texte: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((champ, _)) = self.prix.champs.get(&ingredient) {
            champ.update(cx, |s, cx| s.set_value(texte.to_owned(), window, cx));
        }
    }

    pub(super) fn changer_prix(&mut self, ingredient: i64, texte: &str, cx: &mut Context<Self>) {
        let Some(prix) = lire_nombre(texte) else {
            return;
        };
        let Some(i) = self.cat.index_ingredient(ingredient) else {
            return;
        };
        if self.cat.ingredients[i].prix == prix {
            return;
        }
        self.avis = match self.cat.definir_prix(ingredient, prix) {
            Ok(()) => None,
            Err(e) => Some((false, format!("Prix non enregistré : {e}"))),
        };
        cx.notify();
    }

    fn ajouter_ingredient(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let nom = self.prix.nom.read(cx).value().to_string();
        let Some(prix) = lire_nombre(&self.prix.prix.read(cx).value()) else {
            self.avis = Some((false, "Indique le prix du nouvel ingrédient.".into()));
            cx.notify();
            return;
        };
        let rayon = self
            .prix
            .rayon
            .read(cx)
            .selected_value()
            .copied()
            .unwrap_or(Rayon::Epicerie);
        let unite = self
            .prix
            .unite
            .read(cx)
            .selected_value()
            .copied()
            .unwrap_or(Unite::G);
        match self.cat.ajouter_ingredient(&nom, rayon, unite, prix) {
            Ok(id) => {
                self.prix
                    .champs
                    .insert(id, champ_prix(id, prix, window, cx));
                self.prix
                    .nom
                    .update(cx, |s, cx| s.set_value("", window, cx));
                self.prix
                    .prix
                    .update(cx, |s, cx| s.set_value("", window, cx));
                self.rafraichir_editeur(window, cx);
                self.avis = Some((true, format!("« {} » ajouté aux ingrédients.", nom.trim())));
            }
            Err(e) => self.avis = Some((false, e.to_string())),
        }
        cx.notify();
    }

    fn supprimer_ingredient(
        &mut self,
        ingredient: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.cat.supprimer_ingredient(ingredient) {
            Ok(()) => {
                self.prix.champs.remove(&ingredient);
                if self
                    .suggestion
                    .as_ref()
                    .is_some_and(|s| s.ingredient == ingredient)
                {
                    self.suggestion = None;
                }
                self.etat.elaguer(&self.cat);
                self.rafraichir_editeur(window, cx);
                self.avis = None;
                self.sauver(cx);
            }
            Err(e) => {
                self.avis = Some((false, e.to_string()));
                cx.notify();
            }
        }
    }

    pub(super) fn page_prix(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let (discret, bordure) = (theme.muted_foreground, theme.border);
        let aiguille = self.prix.filtre.read(cx).value().trim().to_lowercase();
        let unite_choisie = self
            .prix
            .unite
            .read(cx)
            .selected_value()
            .copied()
            .unwrap_or(Unite::G);

        let nouveau = carte(cx)
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Nouvel ingrédient"),
            )
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(180.))
                            .child(Input::new(&self.prix.nom)),
                    )
                    .child(div().w(px(190.)).child(Select::new(&self.prix.rayon)))
                    .child(div().w(px(160.)).child(Select::new(&self.prix.unite)))
                    .child(div().w(px(80.)).child(Input::new(&self.prix.prix)))
                    .child(
                        div()
                            .w(px(64.))
                            .text_sm()
                            .text_color(discret)
                            .child(unite_choisie.prix_par()),
                    )
                    .child(
                        Button::new("ajouter-ingredient")
                            .primary()
                            .icon(IconName::Plus)
                            .label("Ajouter")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.ajouter_ingredient(window, cx)
                            })),
                    ),
            );

        let mut page = v_flex()
            .max_w(px(860.))
            .gap_5()
            .child(entete(
                "Ingrédients et prix",
                "Les prix servent à estimer le coût des recettes et des courses. Une modification est enregistrée dès la saisie ; « Suggérer » propose un prix d'après les relevés d'Open Prices.",
                cx,
            ))
            .child(nouveau)
            .child(
                Input::new(&self.prix.filtre)
                    .prefix(Icon::new(IconName::Search).small())
                    .cleanable(true),
            );

        let mut trouves = 0;
        for rayon in Rayon::TOUS {
            let ingredients: Vec<_> = self
                .cat
                .ingredients
                .iter()
                .enumerate()
                .filter(|(_, i)| {
                    i.rayon == rayon
                        && (aiguille.is_empty() || i.nom.to_lowercase().contains(&aiguille))
                })
                .collect();
            if ingredients.is_empty() {
                continue;
            }
            trouves += ingredients.len();
            let (icone, teinte) = visuel_rayon(rayon);
            let mut section = carte(cx).gap_1().child(
                h_flex()
                    .gap_3()
                    .pb_2()
                    .child(pastille(icone, teinte, 32., cx))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(rayon.nom())),
            );
            for (position, i) in ingredients {
                let id = i.id;
                let utilisations = self.cat.utilisations(id);
                let usage = if utilisations > 0 {
                    div()
                        .text_xs()
                        .text_color(discret)
                        .child(match utilisations {
                            1 => "1 recette".to_owned(),
                            n => format!("{n} recettes"),
                        })
                        .into_any_element()
                } else {
                    Button::new(("supprimer-ingredient", position))
                        .small()
                        .ghost()
                        .icon(IconName::Trash)
                        .tooltip("Supprimer (aucune recette ne l'utilise)")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.supprimer_ingredient(id, window, cx)
                        }))
                        .into_any_element()
                };
                section = section.child(
                    h_flex()
                        .gap_3()
                        .py_1()
                        .border_t_1()
                        .border_color(bordure)
                        .child(div().flex_1().min_w_0().child(i.nom.clone()))
                        .child(usage)
                        .child(
                            Button::new(("suggerer-prix", position))
                                .small()
                                .ghost()
                                .icon(IconName::Sparkles)
                                .label("Suggérer")
                                .tooltip("Suggérer un prix d'après Open Prices")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.ouvrir_suggestion(id, window, cx)
                                })),
                        )
                        .children(
                            self.prix
                                .champs
                                .get(&id)
                                .map(|(champ, _)| div().w(px(88.)).child(Input::new(champ))),
                        )
                        .child(
                            div()
                                .w(px(64.))
                                .text_sm()
                                .text_color(discret)
                                .child(i.unite.prix_par()),
                        ),
                );
            }
            page = page.child(section);
        }
        if trouves == 0 {
            page = page.child(
                div()
                    .py_6()
                    .text_sm()
                    .text_color(discret)
                    .child("Aucun ingrédient ne correspond."),
            );
        }
        page
    }
}
