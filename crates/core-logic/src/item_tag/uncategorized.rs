use crate::item_tag::{ItemTag, TagCategory};

/// An item that is an adhesive, like glue
pub struct Adhesive;

impl ItemTag for Adhesive {
    fn category(&self) -> Option<TagCategory> {
        None
    }
}
