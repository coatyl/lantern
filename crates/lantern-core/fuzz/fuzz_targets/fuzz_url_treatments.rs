//! Fuzz target for URL treatments.
//!
//! Pairs arbitrary URL-shaped input with each URL-mutating treatment and
//! asserts the treatment never panics.  Shallow on purpose: the parser fuzz
//! target covers the deeper state space.
//!
//! Run with:
//!
//! ```bash
//! cd crates/lantern-core/fuzz
//! cargo +nightly fuzz run fuzz_url_treatments
//! ```

#![no_main]

use lantern_core::model::ids::next_document_id;
use lantern_core::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Node};
use lantern_core::sanitize::treatment::{PassContext, Treatment};
use lantern_core::sanitize::treatments::{
    ClickIdsTreatment, DemobilizeTreatment, FragmentTrackingTreatment, HttpsUpgradeTreatment,
    SessionTreatment, StripFragmentTreatment, UserSegmentTreatment, UtmTreatment,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Reject non-UTF-8 input: the URL parser requires UTF-8 anyway and every
    // bookmark file we've seen is UTF-8.
    let Ok(s) = std::str::from_utf8(data) else { return; };
    let Ok(url) = url::Url::parse(s) else { return; };

    let bookmark = Node::Bookmark(Bookmark {
        id: 1,
        title: String::new(),
        url: BookmarkUrl::Valid(url),
        add_date: None,
        last_modified: None,
        icon_blob: None,
        description: None,
        attrs: AttrMap::default(),
        flags: BookmarkFlags::default(),
    });

    let ctx = PassContext {
        document_id: next_document_id(),
    };

    let treatments: Vec<Box<dyn Treatment>> = vec![
        Box::new(UtmTreatment),
        Box::new(ClickIdsTreatment),
        Box::new(SessionTreatment),
        Box::new(StripFragmentTreatment),
        Box::new(FragmentTrackingTreatment),
        Box::new(HttpsUpgradeTreatment),
        Box::new(DemobilizeTreatment),
        Box::new(UserSegmentTreatment),
    ];
    for t in &treatments {
        let _ = t.propose(&bookmark, &ctx);
    }
});
