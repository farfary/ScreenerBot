//! Section ids the import preview emits and the dialog's frozen label map.

use super::types::CONFIG_SECTIONS;
use crate::i18n::format_en;

/// The preview sends `name` only, never a label. The dialog names each section
/// from the catalog: `config-section-<id with _ as ->` (`pages/config/field_text.js`
/// `sectionLabel`), except `gui`, which uses `system-config-section-gui`.
#[test]
fn every_previewed_section_id_has_a_label_key() {
    for section in CONFIG_SECTIONS {
        let id = if *section == "gui" {
            "system-config-section-gui".to_owned()
        } else {
            format!("config-section-{}", section.replace('_', "-"))
        };
        assert_ne!(format_en(&id, None), id, "section {section}: missing {id}");
    }
}
