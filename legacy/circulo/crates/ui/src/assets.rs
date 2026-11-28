//! The asset source GPUI asks for when it loads an icon or a font.

use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};

use crate::icons;

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(icons::bytes(path).map(Cow::Borrowed))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
