//! Onglet « Recettes » et volet de droite : lecture d'une recette, création,
//! modification et suppression.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Sizable as _,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState, Textarea, TextareaState},
    select::{Select, SelectEvent, SelectState},
    switch::Switch,
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{Liste, Mijot, ecrire_nombre, lire_nombre, liste, quantite_recette, selecteur};
use crate::catalogue::{Catalogue, Recette};
use crate::donnees::Categorie;
use crate::plan::{REPAS_MAX, cout_par_personne, euros};

/// Formulaire d'une recette. Rien n'est écrit en base avant « Enregistrer ».
pub(super) struct Editeur {
    /// Recette modifiée, ou `None` pour une création.
    id: Option<i64>,
    nom: Entity<InputState>,
    minutes: Entity<InputState>,
    categorie: Entity<SelectState<Liste<Categorie>>>,
    vege: bool,
    /// (identifiant d'ingrédient, champ de quantité par personne).
    lignes: Vec<(i64, Entity<InputState>)>,
    /// Liste des ingrédients à ajouter ; un choix crée une ligne.
    choix: Entity<SelectState<Liste<i64>>>,
    /// Une étape par ligne.
    etapes: Entity<TextareaState>,
    /// Premier clic sur « Supprimer » : demande de confirmation affichée.
    confirmer_suppression: bool,
    erreur: Option<String>,
    _abonnements: Vec<Subscription>,
}

/// Ingrédients proposés dans l'éditeur, par ordre alphabétique.
fn choix_ingredients(cat: &Catalogue) -> Liste<i64> {
    let mut choix: Vec<(String, i64)> = cat
        .ingredients
        .iter()
        .map(|i| (format!("{} ({})", i.nom, i.unite.abrege()), i.id))
        .collect();
    choix.sort_by_key(|(nom, _)| nom.to_lowercase());
    liste(choix)
}

impl Mijot {
    // --- Actions -------------------------------------------------------------

    /// Ouvre le formulaire sur une recette existante, ou vide pour une création.
    fn ouvrir_editeur(
        &mut self,
        recette: Option<i64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let existante = recette
            .and_then(|id| self.cat.index_recette(id))
            .map(|i| self.cat.recettes[i].clone());
        let champ = |texte: String,
                     indication: &'static str,
                     window: &mut Window,
                     cx: &mut Context<Self>| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(indication)
                    .default_value(texte)
            })
        };
        let nom = champ(
            existante
                .as_ref()
                .map(|r| r.nom.clone())
                .unwrap_or_default(),
            "Nom de la recette",
            window,
            cx,
        );
        let minutes = champ(
            existante
                .as_ref()
                .map(|r| r.minutes.to_string())
                .unwrap_or_else(|| "30".into()),
            "30",
            window,
            cx,
        );
        let categorie = selecteur(
            liste(Categorie::TOUTES.map(|c| (c.nom().to_owned(), c))),
            Some(
                &existante
                    .as_ref()
                    .map_or(Categorie::Express, |r| r.categorie),
            ),
            false,
            window,
            cx,
        );
        let choix = selecteur(choix_ingredients(&self.cat), None, true, window, cx);
        let texte_etapes = existante
            .as_ref()
            .map(|r| r.etapes.join("\n"))
            .unwrap_or_default();
        let etapes = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(4, 14)
                .placeholder("Une étape par ligne")
                .default_value(texte_etapes)
        });
        let abonnements = vec![cx.subscribe_in(
            &choix,
            window,
            |this, choix, event: &SelectEvent<Liste<i64>>, window, cx| {
                let SelectEvent::Confirm(Some(ingredient)) = event else {
                    return;
                };
                this.ajouter_ligne(*ingredient, None, window, cx);
                // La liste redevient vide, prête pour l'ingrédient suivant.
                choix.update(cx, |s, cx| s.set_selected_index(None, window, cx));
            },
        )];
        self.editeur = Some(Editeur {
            id: existante.as_ref().map(|r| r.id),
            nom,
            minutes,
            categorie,
            vege: existante.as_ref().is_some_and(|r| r.vege),
            lignes: Vec::new(),
            choix,
            etapes,
            confirmer_suppression: false,
            erreur: None,
            _abonnements: abonnements,
        });
        for (ingredient, quantite) in existante.map(|r| r.ingredients).unwrap_or_default() {
            self.ajouter_ligne(ingredient, Some(quantite), window, cx);
        }
        self.avis = None;
        cx.notify();
    }

    fn ajouter_ligne(
        &mut self,
        ingredient: i64,
        quantite: Option<f64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editeur) = &mut self.editeur else {
            return;
        };
        if editeur.lignes.iter().any(|(id, _)| *id == ingredient) {
            editeur.erreur = Some("Cet ingrédient est déjà dans la recette.".into());
        } else {
            let champ = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Quantité")
                    .default_value(quantite.map(ecrire_nombre).unwrap_or_default())
            });
            editeur.lignes.push((ingredient, champ));
            editeur.erreur = None;
        }
        cx.notify();
    }

    /// Le catalogue d'ingrédients a changé : met à jour le formulaire ouvert.
    pub(super) fn rafraichir_editeur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editeur) = &mut self.editeur else {
            return;
        };
        editeur
            .lignes
            .retain(|(id, _)| self.cat.index_ingredient(*id).is_some());
        let choix = choix_ingredients(&self.cat);
        editeur
            .choix
            .update(cx, |s, cx| s.set_items(choix, window, cx));
    }

    /// Lit le formulaire ; `Err` porte le message à afficher.
    fn lire_editeur(&self, cx: &App) -> Result<Recette, String> {
        let Some(editeur) = &self.editeur else {
            return Err("Aucune recette en cours de modification.".into());
        };
        let minutes = editeur.minutes.read(cx).value();
        let Ok(minutes) = minutes.trim().parse::<u32>() else {
            return Err("Indique le temps en minutes (un nombre entier).".into());
        };
        let mut ingredients = Vec::with_capacity(editeur.lignes.len());
        for (id, champ) in &editeur.lignes {
            let nom = self
                .cat
                .index_ingredient(*id)
                .map(|i| self.cat.ingredients[i].nom.as_str())
                .unwrap_or("cet ingrédient");
            match lire_nombre(&champ.read(cx).value()) {
                Some(quantite) if quantite > 0. => ingredients.push((*id, quantite)),
                _ => return Err(format!("Indique une quantité positive pour « {nom} ».")),
            }
        }
        Ok(Recette {
            id: editeur.id.unwrap_or(0),
            nom: editeur.nom.read(cx).value().to_string(),
            categorie: editeur
                .categorie
                .read(cx)
                .selected_value()
                .copied()
                .unwrap_or(Categorie::Express),
            vege: editeur.vege,
            minutes,
            ingredients,
            etapes: editeur
                .etapes
                .read(cx)
                .value()
                .lines()
                .map(str::to_owned)
                .collect(),
        })
    }

    fn enregistrer_recette(&mut self, cx: &mut Context<Self>) {
        let resultat = self.lire_editeur(cx).and_then(|recette| {
            self.cat
                .enregistrer_recette(&recette)
                .map(|id| (id, recette.nom.trim().to_owned()))
                .map_err(|e| e.to_string())
        });
        match resultat {
            Ok((id, nom)) => {
                self.editeur = None;
                self.detail = Some(id);
                // Les ingrédients d'une recette du menu ont pu changer.
                self.etat.elaguer(&self.cat);
                self.avis = Some((true, format!("« {nom} » enregistrée.")));
                self.sauver(cx);
            }
            Err(message) => {
                if let Some(editeur) = &mut self.editeur {
                    editeur.erreur = Some(message);
                }
                cx.notify();
            }
        }
    }

    fn supprimer_recette(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.editeur.as_ref().and_then(|e| e.id) else {
            return;
        };
        let nom = self
            .cat
            .index_recette(id)
            .map(|i| self.cat.recettes[i].nom.clone())
            .unwrap_or_default();
        match self.cat.supprimer_recette(id) {
            Ok(()) => {
                self.editeur = None;
                self.detail = None;
                self.etat.elaguer(&self.cat);
                self.avis = Some((true, format!("« {nom} » supprimée.")));
                self.sauver(cx);
            }
            Err(e) => {
                if let Some(editeur) = &mut self.editeur {
                    editeur.erreur = Some(e.to_string());
                }
                cx.notify();
            }
        }
    }

    // --- Liste des recettes --------------------------------------------------

    pub(super) fn page_recettes(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let (discret, survol, selection, rayon) = (
            theme.muted_foreground,
            theme.accent.opacity(0.5),
            theme.accent,
            theme.radius,
        );
        let aiguille = self.filtre.read(cx).value().trim().to_lowercase();
        let visibles: Vec<usize> = (0..self.cat.recettes.len())
            .filter(|&r| {
                let recette = &self.cat.recettes[r];
                aiguille.is_empty()
                    || recette.nom.to_lowercase().contains(&aiguille)
                    || recette.categorie.nom().to_lowercase().contains(&aiguille)
                    || recette.ingredients.iter().any(|(id, _)| {
                        self.cat.index_ingredient(*id).is_some_and(|i| {
                            self.cat.ingredients[i]
                                .nom
                                .to_lowercase()
                                .contains(&aiguille)
                        })
                    })
            })
            .collect();

        let mut lignes = v_flex().gap_1();
        if visibles.is_empty() {
            lignes = lignes.child(
                div()
                    .py_6()
                    .text_sm()
                    .text_color(discret)
                    .child("Aucune recette ne correspond."),
            );
        }
        for r in visibles {
            let recette = &self.cat.recettes[r];
            let id = recette.id;
            let ouverte = self.detail == Some(id);
            lignes = lignes.child(
                h_flex()
                    .id(("recette", r))
                    .justify_between()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .rounded(rayon)
                    .cursor_pointer()
                    .when(ouverte, |row| row.bg(selection))
                    .when(!ouverte, |row| row.hover(move |row| row.bg(survol)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.detail = Some(id);
                        cx.notify();
                    }))
                    .child(
                        h_flex()
                            .gap_3()
                            .flex_wrap()
                            .child(div().child(recette.nom.clone()))
                            .child(self.etiquettes(r)),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_sm()
                            .text_color(discret)
                            .child(format!(
                                "{} / pers.",
                                euros(cout_par_personne(&self.cat, r))
                            )),
                    ),
            );
        }

        v_flex()
            .max_w(px(820.))
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div().flex_1().child(
                            Input::new(&self.filtre)
                                .prefix(Icon::new(IconName::Search).small())
                                .cleanable(true),
                        ),
                    )
                    .child(
                        Button::new("nouvelle-recette")
                            .primary()
                            .icon(IconName::Plus)
                            .label("Nouvelle recette")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.ouvrir_editeur(None, window, cx)
                            })),
                    ),
            )
            .child(lignes)
    }

    // --- Volet ---------------------------------------------------------------

    /// Volet de droite : le formulaire s'il est ouvert, sinon la recette consultée.
    pub(super) fn volet(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if let Some(editeur) = &self.editeur {
            return Some(self.volet_edition(editeur, cx).into_any_element());
        }
        let recette = self.detail.and_then(|id| self.cat.index_recette(id))?;
        Some(self.volet_lecture(recette, cx).into_any_element())
    }

    fn volet_lecture(&self, recette: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (discret, bordure) = (theme.muted_foreground, theme.border);
        let r = &self.cat.recettes[recette];
        let id = r.id;
        let personnes = self.etat.personnes;
        let par_personne = cout_par_personne(&self.cat, recette);
        let au_menu = self.etat.menu.contains(&id);
        let menu_plein = self.etat.menu.len() >= REPAS_MAX;

        let mut ingredients = v_flex().gap_1();
        for (ingredient, quantite) in &r.ingredients {
            let Some(i) = self.cat.index_ingredient(*ingredient) else {
                continue;
            };
            let i = &self.cat.ingredients[i];
            ingredients = ingredients.child(
                h_flex()
                    .justify_between()
                    .gap_3()
                    .text_sm()
                    .child(div().child(i.nom.clone()))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_color(discret)
                            .child(quantite_recette(quantite * f64::from(personnes), i.unite)),
                    ),
            );
        }

        let mut etapes = v_flex().gap_3();
        for (n, etape) in r.etapes.iter().enumerate() {
            etapes = etapes.child(
                div()
                    .flex()
                    .gap_2()
                    .text_sm()
                    .child(
                        div()
                            .flex_shrink_0()
                            .w(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(discret)
                            .child(format!("{}.", n + 1)),
                    )
                    .child(div().flex_1().min_w_0().child(etape.clone())),
            );
        }

        let titre = |texte: String| {
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(texte)
        };

        v_flex()
            .id("volet")
            .w(px(380.))
            .flex_shrink_0()
            .overflow_y_scroll()
            .border_l_1()
            .border_color(bordure)
            .p_5()
            .gap_4()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(r.nom.clone()),
                    )
                    .child(
                        Button::new("fermer-volet")
                            .small()
                            .ghost()
                            .icon(IconName::X)
                            .tooltip("Fermer")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.detail = None;
                                cx.notify();
                            })),
                    ),
            )
            .child(self.etiquettes(recette))
            .child(div().text_sm().text_color(discret).child(format!(
                "≈ {} pour {personnes} · {} par personne",
                euros(par_personne * f64::from(personnes)),
                euros(par_personne)
            )))
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(if au_menu {
                        Tag::primary()
                            .child("Au menu cette semaine")
                            .into_any_element()
                    } else {
                        Button::new("ajouter-au-menu")
                            .outline()
                            .icon(IconName::Plus)
                            .label("Ajouter au menu")
                            .disabled(menu_plein)
                            .on_click(
                                cx.listener(move |this, _, _, cx| {
                                    this.ajouter_au_menu(recette, cx)
                                }),
                            )
                            .into_any_element()
                    })
                    .child(
                        Button::new("modifier-recette")
                            .outline()
                            .icon(IconName::Pencil)
                            .label("Modifier")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.ouvrir_editeur(Some(id), window, cx)
                            })),
                    ),
            )
            .child(titre(format!("Ingrédients pour {personnes}")))
            .child(ingredients)
            .when(!r.etapes.is_empty(), |volet| {
                volet.child(titre("Préparation".into())).child(etapes)
            })
    }

    fn volet_edition(&self, editeur: &Editeur, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (discret, bordure) = (theme.muted_foreground, theme.border);
        let libelle = |texte: &'static str| div().text_sm().text_color(discret).child(texte);

        let mut lignes = v_flex().gap_2();
        for (position, (ingredient, champ)) in editeur.lignes.iter().enumerate() {
            let Some(i) = self.cat.index_ingredient(*ingredient) else {
                continue;
            };
            let i = &self.cat.ingredients[i];
            lignes = lignes.child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().min_w_0().text_sm().child(i.nom.clone()))
                    .child(div().w(px(84.)).child(Input::new(champ)))
                    .child(
                        div()
                            .w(px(40.))
                            .text_sm()
                            .text_color(discret)
                            .child(i.unite.abrege()),
                    )
                    .child(
                        Button::new(("retirer-ligne", position))
                            .small()
                            .ghost()
                            .icon(IconName::X)
                            .tooltip("Retirer de la recette")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(editeur) = &mut this.editeur
                                    && position < editeur.lignes.len()
                                {
                                    editeur.lignes.remove(position);
                                    cx.notify();
                                }
                            })),
                    ),
            );
        }

        let actions = if editeur.confirmer_suppression {
            h_flex()
                .gap_2()
                .flex_wrap()
                .child(
                    div()
                        .text_sm()
                        .child("Supprimer définitivement cette recette ?"),
                )
                .child(
                    Button::new("confirmer-suppression")
                        .danger()
                        .label("Supprimer")
                        .on_click(cx.listener(|this, _, _, cx| this.supprimer_recette(cx))),
                )
                .child(
                    Button::new("annuler-suppression")
                        .ghost()
                        .label("Garder")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(editeur) = &mut this.editeur {
                                editeur.confirmer_suppression = false;
                                cx.notify();
                            }
                        })),
                )
        } else {
            h_flex()
                .gap_2()
                .child(
                    Button::new("enregistrer-recette")
                        .primary()
                        .label("Enregistrer")
                        .on_click(cx.listener(|this, _, _, cx| this.enregistrer_recette(cx))),
                )
                .child(
                    Button::new("annuler-recette")
                        .ghost()
                        .label("Annuler")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.editeur = None;
                            cx.notify();
                        })),
                )
                .child(div().flex_1())
                .when(editeur.id.is_some(), |row| {
                    row.child(
                        Button::new("supprimer-recette")
                            .ghost()
                            .icon(IconName::Trash)
                            .label("Supprimer")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(editeur) = &mut this.editeur {
                                    editeur.confirmer_suppression = true;
                                    cx.notify();
                                }
                            })),
                    )
                })
        };

        v_flex()
            .id("volet-edition")
            .w(px(440.))
            .flex_shrink_0()
            .overflow_y_scroll()
            .border_l_1()
            .border_color(bordure)
            .p_5()
            .gap_3()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(if editeur.id.is_some() {
                        "Modifier la recette"
                    } else {
                        "Nouvelle recette"
                    }),
            )
            .children(
                editeur
                    .erreur
                    .as_ref()
                    .map(|erreur| Alert::error("erreur-recette", erreur.clone())),
            )
            .child(libelle("Nom"))
            .child(Input::new(&editeur.nom))
            .child(
                h_flex()
                    .gap_3()
                    .items_end()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(libelle("Catégorie"))
                            .child(Select::new(&editeur.categorie)),
                    )
                    .child(
                        v_flex()
                            .w(px(110.))
                            .gap_1()
                            .child(libelle("Temps (min)"))
                            .child(Input::new(&editeur.minutes)),
                    ),
            )
            .child(
                Switch::new("recette-vege")
                    .checked(editeur.vege)
                    .label("Végétarienne")
                    .on_click(cx.listener(|this, on: &bool, _, cx| {
                        if let Some(editeur) = &mut this.editeur {
                            editeur.vege = *on;
                            cx.notify();
                        }
                    })),
            )
            .child(libelle("Ingrédients — quantité par personne"))
            .child(lignes)
            .child(
                // Encadrée : seule dans la colonne, la liste s'étirerait en hauteur.
 div().child(Select::new(&editeur.choix)
                    .placeholder("Ajouter un ingrédient…")
                    .search_placeholder("Chercher…")),
            )
            .child(div().text_xs().text_color(discret).child(
                "Il manque un ingrédient ? Crée-le dans l'onglet « Ingrédients et prix », puis reviens ici : le formulaire reste ouvert.",
            ))
            .child(libelle("Préparation — une étape par ligne"))
            .child(Textarea::new(&editeur.etapes))
            .child(div().pt_2().child(actions))
    }
}
