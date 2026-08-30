use crate::item_tag::{ItemTag, TagCategory};

/// An item in the shape of a rod (long, rigid)
pub struct ShapeRod;

impl ItemTag for ShapeRod {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Shape)
    }
}

/// An item in the shape of rope (long, thin, flexible)
pub struct ShapeRope;

impl ItemTag for ShapeRope {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Shape)
    }
}
