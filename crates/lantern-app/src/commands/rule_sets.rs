//! Rule-set editor: on-disk rule sets and the treatment catalogue.

use std::path::Path;

use lantern_core::sanitize::pass::RuleSet;
use lantern_core::sanitize::treatment::{Treatment, TreatmentCategory};
use lantern_core::sanitize::treatments::{
    AffiliateTreatment, AuthorSuffixTreatment, ClickIdsTreatment, CustomQpTreatment,
    DemobilizeTreatment, EmailTreatment, EmptyFoldersTreatment, ExactUrlDuplicatesTreatment,
    FolderHtmlEntitiesTreatment, FolderWhitespaceTreatment, FragmentTrackingTreatment,
    HandleTreatment, HtmlEntitiesTreatment, HttpsUpgradeTreatment, NearUrlDuplicatesTreatment,
    RegexFolderTreatment, RegexTitleTreatment, SearchTokensTreatment, SessionTreatment,
    StripFragmentTreatment, UnshortenOfflineTreatment, UserSegmentTreatment, UtmTreatment,
    WhitespaceTreatment,
};
use lantern_io::rulestore;

use crate::error::{CommandResult, UiError};
use crate::state::AppState;
use crate::types::*;

/// Built-in rule sets first, then user sets alphabetically.
#[tauri::command]
pub async fn list_rule_sets(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<RuleSetSummary>> {
    let raw = rulestore::list_rule_sets(&state.rules_dir).map_err(io_err)?;
    Ok(raw
        .into_iter()
        .map(|s| RuleSetSummary {
            name: s.name,
            treatment_count: s.treatment_count as u32,
            is_builtin: s.is_builtin,
            path: s.path.to_string_lossy().into_owned(),
        })
        .collect())
}

#[tauri::command]
pub async fn get_rule_set(
    name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuleSetDetail> {
    load_detail(&state.rules_dir, &name)
}

/// Create or overwrite a rule set from treatment ids alone.
#[tauri::command]
pub async fn save_rule_set(
    name: String,
    treatment_ids: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuleSetDetail> {
    let refs: Vec<&str> = treatment_ids.iter().map(String::as_str).collect();
    rulestore::save_rule_set(&state.rules_dir, &name, &refs).map_err(io_err)?;
    load_detail(&state.rules_dir, &name)
}

/// Create or overwrite a rule set whose treatments carry configuration
/// (custom query parameters, regex pattern and replacement, ...).
#[tauri::command]
pub async fn save_rule_set_with_configs(
    name: String,
    treatments: Vec<RuleSetTreatment>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuleSetDetail> {
    let entries = treatments
        .into_iter()
        .map(|t| {
            let config = t
                .config
                .map(|json| json_to_toml(&json).map_err(UiError::InvalidOperation))
                .transpose()?;
            Ok((t.id, config))
        })
        .collect::<CommandResult<Vec<_>>>()?;
    rulestore::save_rule_set_with_configs(&state.rules_dir, &name, &entries).map_err(io_err)?;
    load_detail(&state.rules_dir, &name)
}

/// Delete a user rule set.  Built-ins are refused.
#[tauri::command]
pub async fn delete_rule_set(name: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    rulestore::delete_rule_set(&state.rules_dir, &name)
        .map_err(|e| UiError::InvalidOperation(e.to_string()))
}

/// Copy rule set `source` to a new user rule set called `new_name`.
#[tauri::command]
pub async fn duplicate_rule_set(
    source: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuleSetDetail> {
    rulestore::duplicate_rule_set(&state.rules_dir, &source, &new_name)
        .map_err(|e| UiError::InvalidOperation(e.to_string()))?;
    load_detail(&state.rules_dir, &new_name)
}

/// Every built-in treatment, for the editor's "Add treatment" picker.
#[tauri::command]
pub async fn list_treatments() -> CommandResult<Vec<TreatmentInfo>> {
    let catalogue: Vec<Box<dyn Treatment>> = vec![
        Box::new(UtmTreatment),
        Box::new(ClickIdsTreatment),
        Box::new(SessionTreatment),
        Box::new(AffiliateTreatment),
        Box::new(SearchTokensTreatment),
        Box::new(CustomQpTreatment::empty()),
        Box::new(UserSegmentTreatment),
        Box::new(StripFragmentTreatment),
        Box::new(FragmentTrackingTreatment),
        Box::new(HttpsUpgradeTreatment),
        Box::new(DemobilizeTreatment),
        Box::new(UnshortenOfflineTreatment),
        Box::new(WhitespaceTreatment),
        Box::new(HtmlEntitiesTreatment),
        Box::new(EmailTreatment),
        Box::new(HandleTreatment),
        Box::new(AuthorSuffixTreatment),
        Box::new(RegexTitleTreatment::empty()),
        Box::new(FolderWhitespaceTreatment),
        Box::new(FolderHtmlEntitiesTreatment),
        Box::new(RegexFolderTreatment::empty()),
        Box::new(EmptyFoldersTreatment),
        Box::new(ExactUrlDuplicatesTreatment),
        Box::new(NearUrlDuplicatesTreatment),
    ];
    Ok(catalogue
        .iter()
        .map(|t| TreatmentInfo {
            id: t.id().to_owned(),
            name: t.name().to_owned(),
            category: category_id(t.category()).to_owned(),
            destructive: t.is_destructive(),
        })
        .collect())
}

fn io_err(e: lantern_io::IoError) -> UiError {
    UiError::Io(e.to_string())
}

fn load_detail(rules_dir: &Path, name: &str) -> CommandResult<RuleSetDetail> {
    let rs = rulestore::load_rule_set(rules_dir, name).map_err(io_err)?;
    Ok(detail(&rs, &rulestore::file_path_for(rules_dir, &rs.name)))
}

fn detail(rs: &RuleSet, path: &Path) -> RuleSetDetail {
    RuleSetDetail {
        name: rs.name.clone(),
        treatment_ids: rs.treatments.iter().map(|t| t.id().to_owned()).collect(),
        treatments: rs
            .treatments
            .iter()
            .map(|t| RuleSetTreatment {
                id: t.id().to_owned(),
                config: t.current_config().map(|tv| toml_to_json(&tv)),
            })
            .collect(),
        is_builtin: rulestore::is_builtin(&rs.name),
        path: path.to_string_lossy().into_owned(),
    }
}

fn category_id(c: TreatmentCategory) -> &'static str {
    match c {
        TreatmentCategory::UrlQueryParam => "url_query_param",
        TreatmentCategory::UrlPath => "url_path",
        TreatmentCategory::UrlFragment => "url_fragment",
        TreatmentCategory::UrlHost => "url_host",
        TreatmentCategory::Title => "title",
        TreatmentCategory::FolderName => "folder_name",
        TreatmentCategory::CrossField => "cross_field",
    }
}

fn toml_to_json(v: &toml::Value) -> serde_json::Value {
    use serde_json::Value as J;
    match v {
        toml::Value::String(s) => J::String(s.clone()),
        toml::Value::Integer(i) => J::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f).map_or(J::Null, J::Number),
        toml::Value::Boolean(b) => J::Bool(*b),
        toml::Value::Datetime(dt) => J::String(dt.to_string()),
        toml::Value::Array(arr) => J::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(tbl) => J::Object(
            tbl.iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect(),
        ),
    }
}

/// TOML has no null; a JSON `null` becomes an empty string.
fn json_to_toml(v: &serde_json::Value) -> Result<toml::Value, String> {
    use serde_json::Value as J;
    Ok(match v {
        J::Null => toml::Value::String(String::new()),
        J::Bool(b) => toml::Value::Boolean(*b),
        J::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => toml::Value::Integer(i),
            (None, Some(f)) => toml::Value::Float(f),
            (None, None) => return Err("number out of range".to_owned()),
        },
        J::String(s) => toml::Value::String(s.clone()),
        J::Array(arr) => {
            toml::Value::Array(arr.iter().map(json_to_toml).collect::<Result<_, _>>()?)
        }
        J::Object(obj) => toml::Value::Table(
            obj.iter()
                .map(|(k, v)| Ok((k.clone(), json_to_toml(v)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{block_on, Harness};
    use serde_json::json;

    #[test]
    fn every_listed_treatment_can_be_saved_once() {
        let catalogue = block_on(list_treatments()).unwrap();
        let mut ids: Vec<&str> = catalogue.iter().map(|t| t.id.as_str()).collect();
        for id in &ids {
            assert!(lantern_io::build_ruleset("probe", &[id]).is_ok(), "{id}");
        }
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), catalogue.len(), "duplicate treatment ids");
    }

    #[test]
    fn configs_round_trip_through_the_editor() {
        let h = Harness::new();
        let treatments = vec![
            RuleSetTreatment {
                id: "url.qp.custom".into(),
                config: Some(json!({ "params": ["fbclid", "ref"] })),
            },
            RuleSetTreatment {
                id: "title.whitespace".into(),
                config: None,
            },
        ];
        let saved = block_on(save_rule_set_with_configs(
            "Mine".into(),
            treatments,
            h.state(),
        ))
        .unwrap();
        let loaded = block_on(get_rule_set("Mine".into(), h.state())).unwrap();

        for detail in [&saved, &loaded] {
            assert_eq!(
                detail.treatment_ids,
                vec!["url.qp.custom", "title.whitespace"]
            );
            assert_eq!(
                detail.treatments[0].config,
                Some(json!({ "params": ["fbclid", "ref"] }))
            );
            assert!(!detail.is_builtin);
        }
    }

    #[test]
    fn duplicate_copies_under_the_new_name() {
        let h = Harness::new();
        let copy = block_on(duplicate_rule_set(
            "Minimal clean".into(),
            "My clean".into(),
            h.state(),
        ))
        .unwrap();
        assert_eq!(copy.name, "My clean");
        assert!(!copy.is_builtin);
        assert!(!copy.treatment_ids.is_empty());
    }
}
