//! Simple implementations of the various inline storages.

mod non_tracking_element;
mod non_tracking_range;
mod tracking_elements;

pub use non_tracking_element::NonTrackingElement;
pub use non_tracking_range::NonTrackingRange;
pub use tracking_elements::{TrackingElement, TrackingElementHandle};
