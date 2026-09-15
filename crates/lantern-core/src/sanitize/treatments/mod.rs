//! Built-in treatment implementations.
//!
//! Each sub-module corresponds to a category of treatments defined in
//! PRD §8.3.2.

pub mod cross;
pub mod folder_name;
pub mod title;
pub mod url_misc;
pub mod url_qp;

// ---------------------------------------------------------------------------
// Convenience re-exports for the default rule-set builder
// ---------------------------------------------------------------------------

pub use cross::{DeduplicateTreatment, EmptyFoldersTreatment};
pub use folder_name::{
    FolderHtmlEntitiesTreatment, FolderWhitespaceTreatment, RegexFolderTreatment,
};
pub use title::{
    AuthorSuffixTreatment, EmailTreatment, HandleTreatment, HtmlEntitiesTreatment,
    RegexTitleTreatment, WhitespaceTreatment,
};
pub use url_misc::{
    is_known_shortener_host, DemobilizeTreatment, FragmentTrackingTreatment, HttpsUpgradeTreatment,
    StripFragmentTreatment, UnshortenOfflineTreatment, UserSegmentTreatment,
};
pub use url_qp::{
    AffiliateTreatment, ClickIdsTreatment, CustomQpTreatment, SearchTokensTreatment,
    SessionTreatment, UtmTreatment,
};
