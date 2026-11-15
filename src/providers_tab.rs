//! The Providers tab: fills it from the config, applies edits back, and
//! reads the whole config out of the UI.

use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{
    providers::{
        detect_screens,
        schedule::{fmt_duration, floor, now_secs, Interval, Runs, ScheduleConfig, Unit},
        KeyUse, ProviderId, ProviderSettings, ProvidersConfig, Resolution, ResolutionFilter,
        ResolutionPolicy,
    },
    saver::Saver,
    MainWindow, ProviderCard, ProvidersState, TargetSize,
};

/// Providers in alphabetical order, as the tab lists them.
fn sorted_ids() -> Vec<ProviderId> {
    let mut ids = ProviderId::ALL.to_vec();
    ids.sort_by_key(|id| id.label().to_lowercase());
    ids
}

fn shared(items: &[String]) -> Vec<SharedString> {
    items.iter().map(|s| SharedString::from(s.as_str())).collect()
}

fn size(r: Resolution) -> TargetSize {
    TargetSize { width: r.width as i32, height: r.height as i32 }
}

fn non_empty(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_owned())
}

fn min_interval_text(cfg: &ProvidersConfig, id: ProviderId) -> SharedString {
    format!("minimum {}", fmt_duration(floor(cfg, id))).into()
}

/// The provider's setup help, plus a note when it ignores keywords.
fn note_for(caps: &crate::providers::Capabilities) -> String {
    let mut parts = Vec::new();
    if !caps.help.is_empty() {
        parts.push(caps.help);
    }
    if !caps.keywords {
        parts.push("Doesn't use keywords.");
    }
    parts.join(" ")
}

fn card_from(cfg: &ProvidersConfig, id: ProviderId, status: String) -> ProviderCard {
    let settings = cfg.settings_for(id);
    let caps = id.source().capabilities();
    let interval = settings.interval.unwrap_or(cfg.schedule.interval);

    let sort_options: Vec<SharedString> = if caps.sorts.is_empty() {
        Vec::new()
    } else {
        std::iter::once("Default")
            .chain(caps.sorts.iter().map(|s| s.label()))
            .map(SharedString::from)
            .collect()
    };
    let sort_index = settings
        .sort
        .and_then(|s| caps.sorts.iter().position(|&c| c == s))
        .map_or(0, |p| p as i32 + 1);

    ProviderCard {
        key: id.key().into(),
        name: id.label().into(),
        enabled: settings.enabled,
        own_interval: settings.interval.is_some(),
        every: interval.every.clamp(1, 9999) as i32,
        unit: interval.unit.index(),
        min_interval: min_interval_text(cfg, id),
        sort_options: ModelRc::new(VecModel::from(sort_options)),
        sort_index,
        has_key: caps.api_key != KeyUse::None,
        key_label: caps.key_label.into(),
        key_required: caps.api_key == KeyUse::Required,
        api_key: settings.api_key.unwrap_or_default().into(),
        has_secret: caps.secret_label.is_some(),
        secret_label: caps.secret_label.unwrap_or_default().into(),
        api_secret: settings.api_secret.unwrap_or_default().into(),
        has_market: caps.market,
        market: settings.market.unwrap_or_default().into(),
        has_purity: caps.purity,
        allow_sketchy: settings.allow_sketchy,
        allow_nsfw: settings.allow_nsfw,
        note: note_for(&caps).into(),
        status: status.into(),
    }
}

fn settings_from_card(card: &ProviderCard) -> Option<(ProviderId, ProviderSettings)> {
    let id = ProviderId::from_key(&card.key)?;
    let caps = id.source().capabilities();
    let sort = usize::try_from(card.sort_index - 1).ok().and_then(|i| caps.sorts.get(i).copied());
    let settings = ProviderSettings {
        enabled: card.enabled,
        interval: card.own_interval.then(|| Interval {
            every: card.every.clamp(1, 9999) as u32,
            unit: Unit::from_index(card.unit),
        }),
        sort,
        api_key: non_empty(&card.api_key),
        api_secret: non_empty(&card.api_secret),
        market: non_empty(&card.market),
        allow_sketchy: card.allow_sketchy,
        allow_nsfw: card.allow_nsfw,
    };
    Some((id, settings))
}

/// Everything the tab currently shows, as a config.
pub fn read_config(window: &MainWindow) -> ProvidersConfig {
    let s = window.global::<ProvidersState>();
    ProvidersConfig {
        download_dir: s.get_download_dir().to_string(),
        max_folder_size_mb: s.get_max_size_mb().max(0) as u64,
        schedule: ScheduleConfig {
            enabled: s.get_schedule_enabled(),
            interval: Interval {
                every: s.get_schedule_every().clamp(1, 9999) as u32,
                unit: Unit::from_index(s.get_schedule_unit()),
            },
        },
        resolution: ResolutionFilter {
            policy: ResolutionPolicy::from_index(s.get_resolution_policy()),
            targets: s
                .get_targets()
                .iter()
                .map(|t| Resolution::new(t.width.max(1) as u32, t.height.max(1) as u32))
                .collect(),
        },
        keywords: s.get_keywords().iter().map(|k| k.to_string()).collect(),
        forbidden_tags: s.get_forbidden_tags().iter().map(|k| k.to_string()).collect(),
        settings: s
            .get_providers()
            .iter()
            .filter_map(|card| settings_from_card(&card))
            .map(|(id, settings)| (id.key().to_owned(), settings))
            .collect(),
    }
}

/// Update each provider's status line and minimum interval.
pub fn refresh_status(window: &MainWindow, runs: &Runs, cfg: &ProvidersConfig) {
    let model = window.global::<ProvidersState>().get_providers();
    let now = now_secs();
    for i in 0..model.row_count() {
        let Some(mut row) = model.row_data(i) else { continue };
        let Some(id) = ProviderId::from_key(&row.key) else { continue };
        row.status = runs.describe(cfg, id, now).into();
        row.min_interval = min_interval_text(cfg, id);
        model.set_row_data(i, row);
    }
}

pub fn setup(window: &MainWindow, saver: &Rc<Saver>, initial: &ProvidersConfig) {
    let s = window.global::<ProvidersState>();

    s.set_schedule_enabled(initial.schedule.enabled);
    s.set_schedule_every(initial.schedule.interval.every.clamp(1, 9999) as i32);
    s.set_schedule_unit(initial.schedule.interval.unit.index());
    s.set_resolution_policy(initial.resolution.policy.index());
    s.set_download_dir(initial.download_dir.as_str().into());
    s.set_max_size_mb(initial.max_folder_size_mb.min(1_048_576) as i32);

    let keywords = Rc::new(VecModel::from(shared(&initial.keywords)));
    let forbidden = Rc::new(VecModel::from(shared(&initial.forbidden_tags)));
    let targets = Rc::new(VecModel::from(
        initial.resolution.targets.iter().copied().map(size).collect::<Vec<_>>(),
    ));
    let cards = Rc::new(VecModel::from(
        sorted_ids().into_iter().map(|id| card_from(initial, id, String::new())).collect::<Vec<_>>(),
    ));
    s.set_keywords(ModelRc::from(keywords.clone()));
    s.set_forbidden_tags(ModelRc::from(forbidden.clone()));
    s.set_targets(ModelRc::from(targets.clone()));
    s.set_providers(ModelRc::from(cards.clone()));

    // Every edit ends up here: scalar controls call it directly, the handlers
    // below call it after changing a model.
    s.on_changed({
        let saver = saver.clone();
        move || saver.mark_dirty()
    });
    let notify: Rc<dyn Fn()> = {
        let weak = window.as_weak();
        Rc::new(move || {
            if let Some(w) = weak.upgrade() {
                w.global::<ProvidersState>().invoke_changed();
            }
        })
    };

    wire_lists(window, &keywords, &forbidden, &notify);
    wire_targets(window, &targets, &notify);
    wire_storage(window, &notify);
    wire_cards(window, &cards, &notify);
}

fn add_unique(model: &VecModel<SharedString>, text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() || model.iter().any(|t| t.eq_ignore_ascii_case(text)) {
        return false;
    }
    model.push(text.into());
    true
}

fn remove_row<T: Clone + 'static>(model: &VecModel<T>, index: i32) -> bool {
    match usize::try_from(index) {
        Ok(i) if i < model.row_count() => {
            model.remove(i);
            true
        }
        _ => false,
    }
}

fn wire_lists(
    window: &MainWindow,
    keywords: &Rc<VecModel<SharedString>>,
    forbidden: &Rc<VecModel<SharedString>>,
    notify: &Rc<dyn Fn()>,
) {
    let s = window.global::<ProvidersState>();
    s.on_add_keyword({
        let (model, notify) = (keywords.clone(), notify.clone());
        move |text| {
            if add_unique(&model, &text) {
                notify();
            }
        }
    });
    s.on_remove_keyword({
        let (model, notify) = (keywords.clone(), notify.clone());
        move |i| {
            if remove_row(&model, i) {
                notify();
            }
        }
    });
    s.on_add_forbidden({
        let (model, notify) = (forbidden.clone(), notify.clone());
        move |text| {
            if add_unique(&model, &text) {
                notify();
            }
        }
    });
    s.on_remove_forbidden({
        let (model, notify) = (forbidden.clone(), notify.clone());
        move |i| {
            if remove_row(&model, i) {
                notify();
            }
        }
    });
}

fn wire_targets(window: &MainWindow, targets: &Rc<VecModel<TargetSize>>, notify: &Rc<dyn Fn()>) {
    let s = window.global::<ProvidersState>();
    s.on_add_target({
        let (model, notify) = (targets.clone(), notify.clone());
        move || {
            model.push(TargetSize { width: 1920, height: 1080 });
            notify();
        }
    });
    s.on_edit_target({
        let (model, notify) = (targets.clone(), notify.clone());
        move |i, width, height| {
            if let Ok(i) = usize::try_from(i) {
                if i < model.row_count() {
                    model.set_row_data(
                        i,
                        TargetSize { width: width.clamp(1, 16384), height: height.clamp(1, 16384) },
                    );
                    notify();
                }
            }
        }
    });
    s.on_remove_target({
        let (model, notify) = (targets.clone(), notify.clone());
        move |i| {
            if remove_row(&model, i) {
                notify();
            }
        }
    });
    s.on_reset_targets({
        let (model, notify) = (targets.clone(), notify.clone());
        move || {
            model.set_vec(detect_screens().into_iter().map(size).collect::<Vec<_>>());
            notify();
        }
    });
}

fn wire_storage(window: &MainWindow, notify: &Rc<dyn Fn()>) {
    let weak = window.as_weak();
    let notify = notify.clone();
    window.global::<ProvidersState>().on_browse_download_dir(move || {
        let Some(path) = rfd::FileDialog::new().pick_folder() else { return };
        if let Some(w) = weak.upgrade() {
            w.global::<ProvidersState>().set_download_dir(path.to_string_lossy().as_ref().into());
            notify();
        }
    });
}

/// Change one card in place. The closure says whether it understood the edit.
fn edit_card(model: &VecModel<ProviderCard>, index: i32, edit: impl FnOnce(&mut ProviderCard) -> bool) -> bool {
    let Ok(i) = usize::try_from(index) else { return false };
    let Some(mut card) = model.row_data(i) else { return false };
    if !edit(&mut card) {
        return false;
    }
    model.set_row_data(i, card);
    true
}

fn wire_cards(window: &MainWindow, cards: &Rc<VecModel<ProviderCard>>, notify: &Rc<dyn Fn()>) {
    let s = window.global::<ProvidersState>();
    s.on_card_bool({
        let (model, notify) = (cards.clone(), notify.clone());
        move |i, field, value| {
            let done = edit_card(&model, i, |c| {
                match field.as_str() {
                    "enabled" => c.enabled = value,
                    "own-interval" => c.own_interval = value,
                    "allow-sketchy" => c.allow_sketchy = value,
                    "allow-nsfw" => c.allow_nsfw = value,
                    _ => return false,
                }
                true
            });
            if done {
                notify();
            }
        }
    });
    s.on_card_int({
        let (model, notify) = (cards.clone(), notify.clone());
        move |i, field, value| {
            let done = edit_card(&model, i, |c| {
                match field.as_str() {
                    "every" => c.every = value.clamp(1, 9999),
                    "unit" => c.unit = value.clamp(0, 2),
                    "sort" => c.sort_index = value.max(0),
                    _ => return false,
                }
                true
            });
            if done {
                notify();
            }
        }
    });
    s.on_card_text({
        let (model, notify) = (cards.clone(), notify.clone());
        move |i, field, value| {
            let done = edit_card(&model, i, |c| {
                match field.as_str() {
                    "api-key" => c.api_key = value.clone(),
                    "api-secret" => c.api_secret = value.clone(),
                    "market" => c.market = value.clone(),
                    _ => return false,
                }
                true
            });
            if done {
                notify();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Sort;

    #[test]
    fn providers_are_listed_alphabetically() {
        let labels: Vec<_> = sorted_ids().into_iter().map(|id| id.label()).collect();
        assert_eq!(
            labels,
            vec![
                "Bing Photo of the Day",
                "DeviantArt",
                "Dual Monitor Backgrounds",
                "Social Wallpapering",
                "Unsplash",
                "Wallhaven",
                "WallpaperFusion",
            ]
        );
    }

    #[test]
    fn a_card_round_trips_to_the_same_settings() {
        let mut cfg = ProvidersConfig::default();
        cfg.settings.insert(
            "wallhaven".into(),
            ProviderSettings {
                enabled: true,
                interval: Some(Interval { every: 2, unit: Unit::Days }),
                sort: Some(Sort::Newest),
                api_key: Some("secret".into()),
                api_secret: Some("shh".into()),
                market: None,
                allow_sketchy: true,
                allow_nsfw: false,
            },
        );
        cfg.settings.insert(
            "bing".into(),
            ProviderSettings { enabled: true, market: Some("de-DE".into()), ..Default::default() },
        );

        for id in ProviderId::ALL {
            let card = card_from(&cfg, id, String::new());
            let (back_id, back) = settings_from_card(&card).unwrap();
            assert_eq!(back_id, id);
            assert_eq!(format!("{back:?}"), format!("{:?}", cfg.settings_for(id)), "{}", id.label());
        }
    }

    #[test]
    fn sort_options_follow_the_providers_capabilities() {
        let cfg = ProvidersConfig::default();
        let wallhaven = card_from(&cfg, ProviderId::Wallhaven, String::new());
        let bing = card_from(&cfg, ProviderId::Bing, String::new());
        assert_eq!(wallhaven.sort_options.row_count(), 7); // "Default" + 6 sorts
        assert_eq!(bing.sort_options.row_count(), 0);
        assert!(wallhaven.has_key && wallhaven.has_purity && !wallhaven.has_market);
        assert!(bing.has_market && !bing.has_key && !bing.has_purity);
        assert_eq!(bing.note, "Doesn't use keywords.");
        assert_eq!(wallhaven.note, "An API key is optional; NSFW content needs one.");
        assert_eq!(wallhaven.key_label, "API key");
        assert!(!wallhaven.key_required && !wallhaven.has_secret);

        let deviantart = card_from(&cfg, ProviderId::DeviantArt, String::new());
        assert!(deviantart.has_key && deviantart.key_required && deviantart.has_secret);
        assert_eq!((deviantart.key_label.as_str(), deviantart.secret_label.as_str()), ("Client ID", "Client secret"));
        assert_eq!(deviantart.sort_options.row_count(), 3); // "Default" + Top + Newest

        let unsplash = card_from(&cfg, ProviderId::Unsplash, String::new());
        assert!(unsplash.key_required && !unsplash.has_secret);
        assert_eq!(unsplash.key_label, "Access key");
    }

    #[test]
    fn add_unique_ignores_blanks_and_duplicates() {
        let model = VecModel::from(vec![SharedString::from("Mountains")]);
        assert!(!add_unique(&model, "  "));
        assert!(!add_unique(&model, "mountains"));
        assert!(add_unique(&model, " ocean "));
        assert_eq!(model.row_data(1).unwrap(), "ocean");
    }

    #[test]
    fn removing_out_of_range_rows_is_harmless() {
        let model = VecModel::from(vec![1, 2, 3]);
        assert!(!remove_row(&model, -1));
        assert!(!remove_row(&model, 3));
        assert!(remove_row(&model, 1));
        assert_eq!(model.row_count(), 2);
    }
}
