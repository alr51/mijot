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

use super::style::{
    carte, echelle_score, entete, entete_volet, etat_vide, numero, pastille, pastille_score,
    titre_section, tuile, tuiles, visuel_categorie, volet,
};
use super::{Liste, Mijot, ecrire_nombre, lire_nombre, liste, quantite_recette, selecteur};
use crate::catalogue::{Catalogue, Recette};
use crate::donnees::Categorie;
use crate::nutrition::{self, grammes};
use crate::plan::{REPAS_MAX, cout_par_personne, euros};

/// Formulaire d'une recette. Rien n'est écrit en base avant « Enregistrer ».
pub(super) struct Editeur {
    /// Recette modifiée, ou `None` pour une création.
    id: Option<i64>,
    nom: Entity<InputState>,
    minutes: Entity<InputState>,
    origine: Entity<InputState>,
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
        let origine = champ(
            existante
                .as_ref()
                .map(|r| r.origine.clone())
                .unwrap_or_default(),
            "Pays ou région (facultatif)",
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
            origine,
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
            origine: editeur.origine.read(cx).value().to_string(),
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
                    || recette.origine.to_lowercase().contains(&aiguille)
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
            lignes = lignes.child(etat_vide(
                IconName::Search,
                "Aucune recette ne correspond",
                "Essaie un autre mot : un nom de plat, une catégorie, un pays ou un ingrédient.",
                cx,
            ));
        }
        for r in visibles {
            let recette = &self.cat.recettes[r];
            let id = recette.id;
            let (icone, teinte) = visuel_categorie(recette.categorie);
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
                            .flex_1()
                            .min_w_0()
                            .gap_3()
                            .child(pastille(icone, teinte, 36., cx))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(recette.nom.clone()),
                                    )
                                    .child(self.etiquettes(r)),
                            ),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .gap_3()
                            .text_sm()
                            .text_color(discret)
                            .child(format!(
                                "{} / pers.",
                                euros(cout_par_personne(&self.cat, r))
                            ))
                            .children(
                                nutrition::bilan(&self.cat, r)
                                    .map(|bilan| pastille_score(bilan.score)),
                            ),
                    ),
            );
        }

        v_flex()
            .max_w(px(860.))
            .gap_4()
            .child(entete(
                "Recettes",
                match self.cat.recettes.len() {
                    1 => "1 recette au catalogue. Clique dessus pour la voir ou la modifier.".to_owned(),
                    n => format!("{n} recettes au catalogue. Clique sur l'une d'elles pour la voir ou la modifier."),
                },
                cx,
            ))
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
                            .label(if self.compact {
                                "Nouvelle"
                            } else {
                                "Nouvelle recette"
                            })
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

    /// Nutrition d'une portion : Nutri-Score estimé et apports.
    fn carte_nutrition(&self, recette: usize, cx: &App) -> Option<Div> {
        let bilan = nutrition::bilan(&self.cat, recette)?;
        let discret = cx.theme().muted_foreground;
        let a = bilan.par_personne;
        let note = match bilan.manquants {
            0 => {
                "Estimation d'après des valeurs moyennes, hors sel ajouté à la cuisson.".to_owned()
            }
            1 => "Estimation incomplète : un ingrédient n'a pas de valeurs connues.".to_owned(),
            n => format!("Estimation incomplète : {n} ingrédients n'ont pas de valeurs connues."),
        };
        Some(
            v_flex()
                .gap_3()
                .child(
                    carte(cx)
                        .p_3()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .child(echelle_score(bilan.score))
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(format!("Nutri-Score {}", bilan.score.lettre())),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(discret)
                                        .child("Estimé pour 100 g de plat"),
                                ),
                        ),
                )
                .child(
                    tuiles(self.compact)
                        .child(tuile("Calories", format!("{:.0}", a.kcal), cx))
                        .child(tuile("Glucides", grammes(a.glucides), cx))
                        .child(tuile("Protéines", grammes(a.proteines), cx))
                        .child(tuile("Lipides", grammes(a.lipides), cx)),
                )
                .child(div().text_xs().text_color(discret).child(format!(
                    "Dont sucres {} · graisses saturées {} · fibres {} · sel {}",
                    grammes(a.sucres),
                    grammes(a.satures),
                    grammes(a.fibres),
                    grammes(a.sel),
                )))
                .child(div().text_xs().text_color(discret).child(note)),
        )
    }

    fn volet_lecture(&self, recette: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let bordure = cx.theme().border;
        let nutrition = self.carte_nutrition(recette, cx);
        let r = &self.cat.recettes[recette];
        let id = r.id;
        let (icone, teinte) = visuel_categorie(r.categorie);
        let personnes = self.etat.personnes;
        let pour = match personnes {
            1 => "Pour 1 personne".to_owned(),
            n => format!("Pour {n} personnes"),
        };
        let par_personne = cout_par_personne(&self.cat, recette);
        let au_menu = self.etat.menu.contains(&id);
        let menu_plein = self.etat.menu.len() >= REPAS_MAX;

        let mut ingredients = carte(cx).py_2();
        let mut premier = true;
        for (ingredient, quantite) in &r.ingredients {
            let Some(i) = self.cat.index_ingredient(*ingredient) else {
                continue;
            };
            let i = &self.cat.ingredients[i];
            ingredients = ingredients.child(
                h_flex()
                    .justify_between()
                    .gap_3()
                    .py_1p5()
                    .text_sm()
                    .when(!premier, |ligne| ligne.border_t_1().border_color(bordure))
                    .child(div().flex_1().min_w_0().child(i.nom.clone()))
                    .child(
                        div()
                            .flex_shrink_0()
                            .font_weight(FontWeight::MEDIUM)
                            .child(quantite_recette(quantite * f64::from(personnes), i.unite)),
                    ),
            );
            premier = false;
        }

        let mut etapes = carte(cx).gap_3();
        for (n, etape) in r.etapes.iter().enumerate() {
            etapes = etapes.child(
                div()
                    .flex()
                    .gap_3()
                    .text_sm()
                    .child(numero(n + 1, cx))
                    .child(div().flex_1().min_w_0().child(etape.clone())),
            );
        }

        volet("volet", 400., self.compact, cx)
            .child(entete_volet(
                pastille(icone, teinte, 48., cx),
                r.nom.clone(),
                if r.origine.is_empty() {
                    r.categorie.nom().to_owned()
                } else {
                    format!("{} · {}", r.categorie.nom(), r.origine)
                },
                Button::new("fermer-volet")
                    .small()
                    .ghost()
                    .icon(IconName::X)
                    .tooltip("Fermer")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.detail = None;
                        cx.notify();
                    })),
                cx,
            ))
            .when(r.vege || au_menu, |volet| {
                volet.child(
                    h_flex()
                        .gap_2()
                        .flex_wrap()
                        .when(au_menu, |ligne| {
                            ligne.child(Tag::primary().child("Au menu cette semaine"))
                        })
                        .when(r.vege, |ligne| {
                            ligne.child(Tag::success().child("Végétarien"))
                        }),
                )
            })
            .child(
                tuiles(self.compact)
                    .child(tuile(
                        format!("Pour {personnes}"),
                        format!("≈ {}", euros(par_personne * f64::from(personnes))),
                        cx,
                    ))
                    .child(tuile("Par personne", euros(par_personne), cx))
                    .child(tuile("Temps", format!("{} min", r.minutes), cx)),
            )
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .when(!au_menu, |ligne| {
                        ligne.child(
                            Button::new("ajouter-au-menu")
                                .primary()
                                .icon(IconName::Plus)
                                .label("Ajouter au menu")
                                .disabled(menu_plein)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.ajouter_au_menu(recette, cx)
                                })),
                        )
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
            .when_some(nutrition, |volet, nutrition| {
                volet
                    .child(titre_section("Nutrition · par personne", cx))
                    .child(nutrition)
            })
            .child(titre_section(
                format!("Ingrédients · {}", pour.to_lowercase()),
                cx,
            ))
            .child(ingredients)
            .when(!r.etapes.is_empty(), |volet| {
                volet.child(titre_section("Préparation", cx)).child(etapes)
            })
    }

    fn volet_edition(&self, editeur: &Editeur, cx: &mut Context<Self>) -> impl IntoElement {
        let discret = cx.theme().muted_foreground;

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
        if editeur.lignes.is_empty() {
            lignes = lignes.child(
                div()
                    .text_sm()
                    .text_color(discret)
                    .child("Aucun ingrédient pour l'instant : choisis-en un ci-dessous."),
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
                .flex_wrap()
                .child(
                    Button::new("enregistrer-recette")
                        .primary()
                        .icon(IconName::Check)
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

        let (icone, titre) = if editeur.id.is_some() {
            (IconName::Pencil, "Modifier la recette")
        } else {
            (IconName::Plus, "Nouvelle recette")
        };

        volet("volet-edition", 460., self.compact, cx)
            .child(entete_volet(
                pastille(icone, 16., 48., cx),
                titre,
                "Rien n'est enregistré avant « Enregistrer ».",
                Button::new("fermer-editeur")
                    .small()
                    .ghost()
                    .icon(IconName::X)
                    .tooltip("Annuler")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.editeur = None;
                        cx.notify();
                    })),
                cx,
            ))
            .children(
                editeur
                    .erreur
                    .as_ref()
                    .map(|erreur| Alert::error("erreur-recette", erreur.clone())),
            )
            .child(
                carte(cx)
                    .gap_3()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(titre_section("Nom", cx))
                            .child(Input::new(&editeur.nom)),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .items_end()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .child(titre_section("Catégorie", cx))
                                    .child(Select::new(&editeur.categorie)),
                            )
                            .child(
                                v_flex()
                                    .w(px(110.))
                                    .gap_1()
                                    .child(titre_section("Temps (min)", cx))
                                    .child(Input::new(&editeur.minutes)),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(titre_section("Origine", cx))
                            .child(Input::new(&editeur.origine)),
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
                    ),
            )
            .child(
                carte(cx)
                    .gap_3()
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Ingrédients"),
                            )
                            .child(titre_section("Quantité par personne", cx)),
                    )
                    .child(lignes)
                    // Encadrée : seule dans la colonne, la liste s'étirerait en hauteur.
                    .child(
                        div().child(
                            Select::new(&editeur.choix)
                                .placeholder("Ajouter un ingrédient…")
                                .search_placeholder("Chercher…"),
                        ),
                    )
                    .child(div().text_xs().text_color(discret).child(
                        "Il manque un ingrédient ? Crée-le dans « Ingrédients et prix », puis reviens ici : le formulaire reste ouvert.",
                    )),
            )
            .child(
                carte(cx)
                    .gap_3()
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Préparation"),
                            )
                            .child(titre_section("Une étape par ligne", cx)),
                    )
                    .child(Textarea::new(&editeur.etapes)),
            )
            .child(actions)
    }
}
