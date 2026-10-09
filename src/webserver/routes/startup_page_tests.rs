// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The root renders every page the Default Page setting can name, never Home in its place.

use super::page_content;
use crate::config::schemas::{default_tabs, STARTUP_PAGE_HOME};
use crate::webserver::templates;

#[test]
fn every_navigation_tab_renders_its_own_page_at_the_root() {
    let home = templates::home_content();
    for tab in default_tabs() {
        let content = page_content(&tab.id)
            .unwrap_or_else(|| panic!("the root has no markup for the tab {}", tab.id));
        if tab.id != STARTUP_PAGE_HOME {
            assert_ne!(content, home, "the tab {} renders Home", tab.id);
        }
    }
}
