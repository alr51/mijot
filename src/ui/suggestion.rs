//! Volet « Prix suggéré » de l'onglet des prix : choix d'une catégorie Open
//! Food Facts pour l'ingrédient, puis prix médian des relevés d'Open Prices.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::style::{carte, entete_volet, pastille, titre_section, visuel_rayon, volet};
use super::{Mijot, ecrire_nombre};
use crate::catalogue::CategorieOff;
use crate::donnees::Unite;
use crate::plan::euros;
use crate::prix_ouverts::{self, RELEVES_MIN, Releve, Suggestion, Zone};

/// Relevés détaillés sous la suggestion ; les suivants sont seulement comptés.
const RELEVES_AFFICHES: usize = 12;

pub(super) struct Volet {
    pub(super) ingredient: i64,
    terme: Entity<InputState>,
    /// Numéro de la dernière requête lancée : la réponse d'une requête plus
    /// ancienne (catégorie changée entre-temps) est ignorée.
    requete: u64,
    en_cours: bool,
    /// Choix de la catégorie affiché : aucune n'est retenue, ou l'utilisateur en change.
    choix: bool,
    /// Catégories trouvées pour le terme ; `None` tant qu'aucune recherche n'a abouti.
    categories: Option<Vec<String>>,
    /// Résultat pour la catégorie retenue ; `Some(None)` : trop peu de relevés.
    resultat: Option<Option<Suggestion>>,
    erreur: Option<String>,
    _abonnements: Vec<Subscription>,
}

/// `2026-11-12` devient `12/11/2026` ; toute autre forme est rendue telle quelle.
fn date_lisible(date: &str) -> String {
    let parties: Vec<&str> = date.split('-').collect();
    match parties.as_slice() {
        [annee, mois, jour] if annee.len() == 4 => format!("{jour}/{mois}/{annee}"),
        _ => date.to_owned(),
    }
}

impl Mijot {
    // --- Actions -------------------------------------------------------------

    pub(super) fn ouvrir_suggestion(
        &mut self,
        ingredient: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(i) = self.cat.index_ingredient(ingredient) else {
            return;
        };
        let (nom, retenue) = {
            let i = &self.cat.ingredients[i];
            (i.nom.clone(), i.off.clone())
        };
        let terme = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Terme à chercher")
                .default_value(prix_ouverts::terme_de_recherche(&nom))
        });
        let abonnements = vec![cx.subscribe(&terme, |this, _, event: &InputEvent, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.chercher_categories(cx);
            }
        })];
        self.suggestion = Some(Volet {
            ingredient,
            terme,
            requete: 0,
            en_cours: false,
            choix: retenue.is_none(),
            categories: None,
            resultat: None,
            erreur: None,
            _abonnements: abonnements,
        });
        self.avis = None;
        match retenue {
            Some(categorie) => self.charger_releves(categorie.tag, cx),
            None => self.chercher_categories(cx),
        }
        cx.notify();
    }

    /// Lance `travail` (bloquant, réseau) hors du fil de l'interface, puis passe
    /// son résultat à `suite`, sauf si le volet a été fermé ou a changé de sujet.
    fn interroger<R: Send + 'static>(
        &mut self,
        travail: impl FnOnce() -> R + Send + 'static,
        suite: impl FnOnce(&mut Self, R, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let Some(volet) = &mut self.suggestion else {
            return;
        };
        volet.requete += 1;
        volet.en_cours = true;
        volet.erreur = None;
        let (ingredient, requete) = (volet.ingredient, volet.requete);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let reponse = cx
                .background_executor()
                .spawn(async move { travail() })
                .await;
            let _ = this.update(cx, |this, cx| {
                let Some(volet) = &mut this.suggestion else {
                    return;
                };
                if volet.ingredient != ingredient || volet.requete != requete {
                    return;
                }
                volet.en_cours = false;
                suite(this, reponse, cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn chercher_categories(&mut self, cx: &mut Context<Self>) {
        let Some(volet) = &mut self.suggestion else {
            return;
        };
        let terme = volet.terme.read(cx).value().trim().to_owned();
        if terme.is_empty() {
            volet.erreur = Some("Indique un terme à chercher.".into());
            cx.notify();
            return;
        }
        self.interroger(
            move || prix_ouverts::chercher_categories(&terme),
            |this, reponse, _| {
                let Some(volet) = &mut this.suggestion else {
                    return;
                };
                match reponse {
                    Ok(categories) => volet.categories = Some(categories),
                    Err(e) => volet.erreur = Some(e.to_string()),
                }
            },
            cx,
        );
    }

    /// Retient la catégorie pour l'ingrédient, puis charge ses relevés.
    fn choisir_categorie(&mut self, nom: String, cx: &mut Context<Self>) {
        let Some((ingredient, unite)) = self.unite_du_volet() else {
            return;
        };
        self.interroger(
            move || -> Result<Option<(CategorieOff, Vec<Releve>)>, prix_ouverts::Erreur> {
                let Some(tag) = prix_ouverts::identifier_categorie(&nom)? else {
                    return Ok(None);
                };
                let releves = prix_ouverts::releves(&tag, unite)?;
                Ok(Some((CategorieOff { tag, nom }, releves)))
            },
            move |this, reponse, _| {
                let enregistrement = match &reponse {
                    Ok(Some((categorie, _))) => this
                        .cat
                        .definir_categorie_off(ingredient, Some(categorie))
                        .err()
                        .map(|e| format!("Catégorie non enregistrée : {e}")),
                    _ => None,
                };
                let Some(volet) = &mut this.suggestion else {
                    return;
                };
                match reponse {
                    Ok(Some((_, releves))) => {
                        volet.choix = false;
                        volet.resultat = Some(prix_ouverts::resumer(&releves));
                        volet.erreur = enregistrement;
                    }
                    Ok(None) => {
                        volet.erreur = Some(
                            "Open Food Facts ne reconnaît pas cette catégorie : choisis-en une autre."
                                .into(),
                        );
                    }
                    Err(e) => volet.erreur = Some(e.to_string()),
                }
            },
            cx,
        );
    }

    fn charger_releves(&mut self, tag: String, cx: &mut Context<Self>) {
        let Some((_, unite)) = self.unite_du_volet() else {
            return;
        };
        self.interroger(
            move || prix_ouverts::releves(&tag, unite),
            |this, reponse, _| {
                let Some(volet) = &mut this.suggestion else {
                    return;
                };
                match reponse {
                    Ok(releves) => volet.resultat = Some(prix_ouverts::resumer(&releves)),
                    Err(e) => volet.erreur = Some(e.to_string()),
                }
            },
            cx,
        );
    }

    /// Ingrédient du volet ouvert et son unité.
    fn unite_du_volet(&self) -> Option<(i64, Unite)> {
        let ingredient = self.suggestion.as_ref()?.ingredient;
        let i = self.cat.index_ingredient(ingredient)?;
        Some((ingredient, self.cat.ingredients[i].unite))
    }

    /// Reporte un prix issu de la suggestion dans le champ de l'ingrédient et l'enregistre.
    fn appliquer_suggestion(&mut self, prix: f64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(volet) = &self.suggestion else {
            return;
        };
        let ingredient = volet.ingredient;
        let Some(i) = self.cat.index_ingredient(ingredient) else {
            return;
        };
        let texte = ecrire_nombre(prix);
        self.definir_champ_prix(ingredient, &texte, window, cx);
        self.changer_prix(ingredient, &texte, cx);
        // `changer_prix` a pu laisser un message d'erreur : ne pas le recouvrir.
        if self.avis.is_none() {
            let i = &self.cat.ingredients[i];
            self.avis = Some((
                true,
                format!(
                    "Prix de « {} » mis à jour : {} {}.",
                    i.nom,
                    ecrire_nombre(i.prix),
                    i.unite.prix_par()
                ),
            ));
        }
        self.suggestion = None;
        cx.notify();
    }

    // --- Volet ---------------------------------------------------------------

    pub(super) fn volet_suggestion(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let etat = self.suggestion.as_ref()?;
        let i = &self.cat.ingredients[self.cat.index_ingredient(etat.ingredient)?];
        let theme = cx.theme();
        let (discret, bordure, survol, rayon, primaire) = (
            theme.muted_foreground,
            theme.border,
            theme.accent,
            theme.radius,
            theme.primary,
        );
        let par = i.unite.prix_par();
        let (icone, teinte) = visuel_rayon(i.rayon);

        let mut panneau = volet("volet-suggestion", 420., cx)
            .child(entete_volet(
                pastille(icone, teinte, 48., cx),
                i.nom.clone(),
                format!("Prix actuel : {} {par}", ecrire_nombre(i.prix)),
                Button::new("fermer-suggestion")
                    .small()
                    .ghost()
                    .icon(IconName::X)
                    .tooltip("Fermer")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.suggestion = None;
                        cx.notify();
                    })),
                cx,
            ))
            .children(
                etat.erreur
                    .as_ref()
                    .map(|erreur| Alert::warning("erreur-suggestion", erreur.clone())),
            );

        if etat.choix {
            let mut choix = carte(cx)
                .gap_3()
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Quelle catégorie ?"),
                        )
                        .child(div().text_sm().text_color(discret).child(
                            "Choisis la catégorie Open Food Facts la plus proche de l'ingrédient. Elle est retenue pour les prochaines fois.",
                        )),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(div().flex_1().child(Input::new(&etat.terme)))
                        .child(
                            Button::new("chercher-categories")
                                .outline()
                                .icon(IconName::Search)
                                .label("Chercher")
                                .loading(etat.en_cours)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.chercher_categories(cx)
                                })),
                        ),
                );
            match &etat.categories {
                Some(categories) if categories.is_empty() => {
                    choix = choix.child(
                        div()
                            .text_sm()
                            .text_color(discret)
                            .child("Aucune catégorie trouvée : essaie un terme plus général."),
                    );
                }
                Some(categories) => {
                    let mut liste = v_flex().gap_1();
                    for (position, nom) in categories.iter().enumerate() {
                        let choisie = nom.clone();
                        liste = liste.child(
                            h_flex()
                                .id(("categorie-off", position))
                                .justify_between()
                                .gap_2()
                                .px_3()
                                .py_2()
                                .rounded(rayon)
                                .border_1()
                                .border_color(bordure)
                                .text_sm()
                                .cursor_pointer()
                                .hover(move |ligne| {
                                    ligne.bg(survol).border_color(primaire.opacity(0.45))
                                })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.choisir_categorie(choisie.clone(), cx)
                                }))
                                .child(div().flex_1().min_w_0().child(nom.clone()))
                                .child(
                                    Icon::new(IconName::ChevronRight)
                                        .small()
                                        .text_color(discret),
                                ),
                        );
                    }
                    choix = choix.child(liste);
                }
                None => {}
            }
            panneau = panneau.child(choix);
        } else {
            panneau = panneau.child(
                carte(cx).p_3().child(
                    h_flex()
                        .justify_between()
                        .gap_2()
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .child(titre_section("Catégorie", cx))
                                .child(div().text_sm().child(match &i.off {
                                    Some(categorie) => categorie.nom.clone(),
                                    None => "Non choisie".to_owned(),
                                })),
                        )
                        .child(
                            Button::new("changer-categorie")
                                .small()
                                .outline()
                                .label("Changer")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let Some(etat) = &mut this.suggestion else {
                                        return;
                                    };
                                    etat.choix = true;
                                    etat.erreur = None;
                                    // Première ouverture du choix : chercher d'après le nom.
                                    if etat.categories.is_none() {
                                        this.chercher_categories(cx);
                                    }
                                    cx.notify();
                                })),
                        ),
                ),
            );
            if etat.en_cours {
                panneau = panneau.child(
                    div()
                        .text_sm()
                        .text_color(discret)
                        .child("Interrogation d'Open Prices…"),
                );
            }
            match &etat.resultat {
                Some(Some(suggestion)) => {
                    let n = suggestion.releves.len();
                    let origine = match suggestion.zone {
                        Zone::France => format!("{n} relevés en France"),
                        Zone::Euro => {
                            format!("{n} relevés en zone euro (trop peu en France)")
                        }
                    };
                    // Triés du plus récent au plus ancien.
                    let periode = match (suggestion.releves.last(), suggestion.releves.first()) {
                        (Some(ancien), Some(recent)) if ancien.date != recent.date => format!(
                            ", du {} au {}",
                            date_lisible(&ancien.date),
                            date_lisible(&recent.date)
                        ),
                        _ => String::new(),
                    };
                    let (mediane, bas) = (suggestion.mediane, suggestion.bas);
                    panneau = panneau.child(
                        carte(cx)
                            .gap_3()
                            .child(
                                v_flex()
                                    .child(titre_section("Prix médian relevé", cx))
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_end()
                                            .child(
                                                div()
                                                    .text_3xl()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(primaire)
                                                    .child(euros(mediane)),
                                            )
                                            .child(
                                                div()
                                                    .pb_1()
                                                    .text_sm()
                                                    .text_color(discret)
                                                    .child(par.trim_start_matches("€ ")),
                                            ),
                                    ),
                            )
                            .child(div().text_sm().text_color(discret).child(format!(
                                "{origine}{periode}. La moitié est entre {} et {}.",
                                euros(suggestion.bas),
                                euros(suggestion.haut)
                            )))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .flex_wrap()
                                    .child(
                                        Button::new("appliquer-mediane")
                                            .primary()
                                            .icon(IconName::Check)
                                            .label(format!("Appliquer {}", euros(mediane)))
                                            .on_click(cx.listener(
                                                move |this, _, window, cx| {
                                                    this.appliquer_suggestion(mediane, window, cx)
                                                },
                                            )),
                                    )
                                    // Le premier quartile : plus proche d'un premier prix.
                                    .when(bas < mediane, |ligne| {
                                        ligne.child(
                                            Button::new("appliquer-bas")
                                                .outline()
                                                .label(format!("Prix bas : {}", euros(bas)))
                                                .tooltip("Un quart des relevés est en dessous")
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        this.appliquer_suggestion(bas, window, cx)
                                                    },
                                                )),
                                        )
                                    }),
                            )
                            .child(div().text_xs().text_color(discret).child(
                                "Une catégorie large mêle premiers prix, marques et bio : affine-la avec « Changer » si la fourchette est trop étendue.",
                            )),
                    );

                    let mut liste = carte(cx).py_2();
                    for (position, releve) in
                        suggestion.releves.iter().take(RELEVES_AFFICHES).enumerate()
                    {
                        liste = liste.child(
                            h_flex()
                                .justify_between()
                                .items_start()
                                .gap_3()
                                .py_1p5()
                                .text_sm()
                                .when(position > 0, |ligne| {
                                    ligne.border_t_1().border_color(bordure)
                                })
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .child(div().truncate().child(releve.libelle.clone()))
                                        .child(
                                            div().text_xs().text_color(discret).truncate().child(
                                                format!(
                                                    "{} · {}",
                                                    releve.magasin,
                                                    date_lisible(&releve.date)
                                                ),
                                            ),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(euros(releve.prix)),
                                ),
                        );
                    }
                    panneau = panneau
                        .child(titre_section(
                            if n > RELEVES_AFFICHES {
                                format!(
                                    "Relevés utilisés · les {RELEVES_AFFICHES} plus récents sur {n}"
                                )
                            } else {
                                "Relevés utilisés".to_owned()
                            },
                            cx,
                        ))
                        .child(liste);
                }
                Some(None) => {
                    let mut vide = carte(cx).gap_2().child(div().text_sm().child(format!(
                        "Pas assez de relevés exploitables pour cette catégorie (il en faut au moins {RELEVES_MIN})."
                    )));
                    if i.unite == Unite::Piece {
                        vide = vide.child(div().text_sm().text_color(discret).child(
                            "Pour un ingrédient compté à la pièce, seuls les produits relevés à l'unité (fruits et légumes surtout) sont utilisables : un paquet ne dit pas combien de pièces il contient.",
                        ));
                    }
                    panneau = panneau.child(vide);
                }
                None => {}
            }
        }

        Some(
            panneau
                .child(div().text_xs().text_color(discret).child(
                    "Source : Open Prices (Open Food Facts), relevés contributifs sous licence ODbL. Seuls le terme cherché et la catégorie choisie sont envoyés.",
                ))
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::date_lisible;

    #[test]
    fn dates() {
        assert_eq!(date_lisible("2026-11-12"), "12/11/2026");
        assert_eq!(date_lisible(""), "");
        assert_eq!(date_lisible("hier"), "hier");
    }
}
