//! Simple implementations of the various inline storages.

mod tracking_elements;
mod non_tracking_element;
mod non_tracking_range;

pub use tracking_elements::{TrackingElement, TrackingElementHandle};
pub use non_tracking_element::NonTrackingElement;
pub use non_tracking_range::NonTrackingRange;
