use crate::item_tag::{ItemTag, TagCategory};

pub struct SizeSmall;

//TODO make a proc macro to auto-derive this
impl ItemTag for SizeSmall {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

pub struct SizeMedium;

impl ItemTag for SizeMedium {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

pub struct SizeLarge;

impl ItemTag for SizeLarge {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}
