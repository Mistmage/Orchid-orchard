//! Workspaces: a named window layout with geometry.
//!
//! Geometry is a hint like every other browser field. It is captured so a
//! workspace can be reopened where it was, but an OS may move the window and a
//! monitor may be gone. The rules live here, not in the extension, so the same
//! clamping applies wherever a workspace is opened.
//!
//! No display or window identifier is stored. A display is described by the
//! work area and scale factor it had; identifiers are not stable across
//! sessions, and a description still tells a person which screen was meant.

use serde::{Deserialize, Serialize};

/// A rectangle in the browser's screen coordinates (device-independent pixels).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn right(&self) -> f64 {
        self.left + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.top + self.height
    }

    fn area(&self) -> f64 {
        (self.width.max(0.0)) * (self.height.max(0.0))
    }
}

/// One visible display. `work_area` is the part a window may occupy (no
/// taskbar); `bounds` is the full screen. Firefox does not always report a work
/// area, so the two are separate instead of one being derived.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub bounds: Rect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_area: Option<Rect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(default)]
    pub primary: bool,
}

impl DisplayInfo {
    /// Where a window may actually sit: the work area, or the full bounds when
    /// the platform does not report one.
    pub fn usable(&self) -> Rect {
        self.work_area
            .clone()
            .unwrap_or_else(|| self.bounds.clone())
    }
}

/// Outer window bounds captured from the browser. Every field is optional
/// because a browser may report only some of them, or none at all.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowGeometry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    /// `normal`, `maximized`, `fullscreen` or `minimized`, as reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Scale factor of the display the window was on, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_scale: Option<f64>,
    /// Work area of the display the window was on, described rather than
    /// identified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Rect>,
}

impl WindowGeometry {
    /// The four outer bounds, when all of them were captured.
    pub fn bounds(&self) -> Option<Rect> {
        Some(Rect {
            left: self.left?,
            top: self.top?,
            width: self.width?,
            height: self.height?,
        })
    }

    /// A geometry built from a rectangle and an optional window state.
    pub fn from_rect(rect: Rect, state: Option<String>) -> Self {
        Self {
            left: Some(rect.left),
            top: Some(rect.top),
            width: Some(rect.width),
            height: Some(rect.height),
            state,
            display_scale: None,
            display: None,
        }
    }
}

/// Where a window should be opened after clamping.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Placement {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    /// True when the captured bounds did not fit the current displays.
    pub clamped: bool,
    /// A sentence for the popup; empty only when nothing was changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Placement {
    /// The placement as a rectangle, for callers that only need the numbers.
    pub fn rect(&self) -> Rect {
        Rect {
            left: self.left,
            top: self.top,
            width: self.width,
            height: self.height,
        }
    }
}

fn overlap(a: &Rect, b: &Rect) -> f64 {
    let width = (a.right().min(b.right()) - a.left.max(b.left)).max(0.0);
    let height = (a.bottom().min(b.bottom()) - a.top.max(b.top)).max(0.0);
    width * height
}

/// The display a window overlaps most. A window that touches no display gets
/// the primary one, because an off-screen window still has to open somewhere.
pub fn display_for(rect: &Rect, displays: &[DisplayInfo]) -> Option<usize> {
    if displays.is_empty() {
        return None;
    }
    let mut best: Option<(usize, f64)> = None;
    for (index, display) in displays.iter().enumerate() {
        let area = overlap(rect, &display.usable());
        if best.map(|(_, best_area)| area > best_area).unwrap_or(true) {
            best = Some((index, area));
        }
    }
    let (index, area) = best?;
    if area > 0.0 {
        return Some(index);
    }
    displays
        .iter()
        .position(|display| display.primary)
        .or(Some(index))
}

/// Pick the display a window that is currently on no display should open on.
/// The captured display is a better hint than the primary one: a workspace
/// that lived on a second monitor should return there when that monitor is
/// still connected, even if its saved origin is now off-screen.
fn display_for_offscreen(saved: Option<&Rect>, displays: &[DisplayInfo]) -> usize {
    if let Some(saved) = saved {
        let mut best: Option<(usize, f64)> = None;
        for (index, display) in displays.iter().enumerate() {
            let area = overlap(saved, &display.usable());
            if best.map(|(_, best_area)| area > best_area).unwrap_or(true) {
                best = Some((index, area));
            }
        }
        if let Some((index, area)) = best
            && area > 0.0
        {
            return index;
        }
    }
    displays
        .iter()
        .position(|display| display.primary)
        .unwrap_or(0)
}

/// Describe the display a window was on without storing an identifier.
pub fn describe_display(geometry: &WindowGeometry, displays: &[DisplayInfo]) -> WindowGeometry {
    let mut described = geometry.clone();
    let Some(rect) = geometry.bounds() else {
        return described;
    };
    let Some(index) = display_for(&rect, displays) else {
        return described;
    };
    let display = &displays[index];
    described.display = Some(display.usable());
    if described.display_scale.is_none() {
        described.display_scale = display.scale;
    }
    described
}

/// Fit captured bounds into the currently visible displays.
///
/// Returns `None` when the capture has no complete bounds to apply. A window
/// that is already fully visible on one display is returned unchanged; a
/// window on no display is centred on the primary one; a window larger than
/// the display is shrunk to fit.
pub fn place_window(geometry: &WindowGeometry, displays: &[DisplayInfo]) -> Option<Placement> {
    let rect = geometry.bounds()?;
    if displays.is_empty() {
        return None;
    }
    let index = if displays
        .iter()
        .any(|display| overlap(&rect, &display.usable()) > 0.0)
    {
        display_for(&rect, displays).unwrap_or(0)
    } else {
        display_for_offscreen(geometry.display.as_ref(), displays)
    };
    let area = displays[index].usable();
    if area.width <= 0.0 || area.height <= 0.0 {
        return None;
    }

    let width = rect.width.min(area.width).max(1.0);
    let height = rect.height.min(area.height).max(1.0);
    let mut clamped = false;

    // A window that overlaps no display is centred instead of clamped to an
    // edge: the origin it remembered is meaningless now.
    let (left, top) = if overlap(&rect, &area) <= 0.0 {
        clamped = true;
        (
            area.left + (area.width - width) / 2.0,
            area.top + (area.height - height) / 2.0,
        )
    } else {
        let left = rect
            .left
            .clamp(area.left, (area.right() - width).max(area.left));
        let top = rect
            .top
            .clamp(area.top, (area.bottom() - height).max(area.top));
        if (left - rect.left).abs() > 0.5 || (top - rect.top).abs() > 0.5 {
            clamped = true;
        }
        (left, top)
    };
    if width < rect.width - 0.5 || height < rect.height - 0.5 {
        clamped = true;
    }

    let reason = if !clamped {
        None
    } else if overlap(&rect, &area) <= 0.0 {
        Some(
            "its saved position was off-screen, so it was centred on a visible display".to_string(),
        )
    } else if width < rect.width - 0.5 || height < rect.height - 0.5 {
        Some("its saved size was larger than the display, so it was shrunk to fit".to_string())
    } else {
        Some("its saved position was partly off-screen, so it was moved into view".to_string())
    };

    Some(Placement {
        left,
        top,
        width,
        height,
        clamped,
        reason,
    })
}

/// The size a display rectangle covers, for tests and callers that need it.
pub fn display_area(display: &DisplayInfo) -> f64 {
    display.usable().area()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(left: f64, top: f64, width: f64, height: f64, primary: bool) -> DisplayInfo {
        DisplayInfo {
            bounds: Rect {
                left,
                top,
                width,
                height,
            },
            work_area: None,
            scale: Some(1.0),
            primary,
        }
    }

    fn geometry(left: f64, top: f64, width: f64, height: f64) -> WindowGeometry {
        WindowGeometry::from_rect(
            Rect {
                left,
                top,
                width,
                height,
            },
            None,
        )
    }

    #[test]
    fn a_fully_visible_window_is_unchanged() {
        let displays = [display(0.0, 0.0, 1920.0, 1080.0, true)];
        let placement = place_window(&geometry(100.0, 80.0, 1200.0, 800.0), &displays).unwrap();
        assert_eq!(
            placement.rect(),
            Rect {
                left: 100.0,
                top: 80.0,
                width: 1200.0,
                height: 800.0
            }
        );
        assert!(!placement.clamped);
        assert!(placement.reason.is_none());
    }

    #[test]
    fn an_offscreen_window_is_centred_on_the_primary_display() {
        let displays = [
            display(0.0, 0.0, 1920.0, 1080.0, true),
            display(1920.0, 0.0, 1280.0, 1024.0, false),
        ];
        let placement = place_window(&geometry(5000.0, 5000.0, 1000.0, 700.0), &displays).unwrap();
        assert_eq!(placement.left, 460.0);
        assert_eq!(placement.top, 190.0);
        assert!(placement.clamped);
        assert!(placement.reason.unwrap().contains("off-screen"));
    }

    #[test]
    fn an_offscreen_window_returns_to_the_display_it_was_saved_on() {
        let displays = [
            display(0.0, 0.0, 1920.0, 1080.0, true),
            display(1920.0, 0.0, 1280.0, 1024.0, false),
        ];
        let mut saved = geometry(5000.0, 5000.0, 1000.0, 700.0);
        saved.display = Some(Rect {
            left: 1920.0,
            top: 0.0,
            width: 1280.0,
            height: 1024.0,
        });
        let placement = place_window(&saved, &displays).unwrap();
        // Centred on the second display, not the primary one.
        assert_eq!(placement.left, 2060.0);
        assert_eq!(placement.top, 162.0);
        assert!(placement.clamped);
    }

    #[test]
    fn a_window_larger_than_its_display_is_shrunk() {
        let displays = [display(0.0, 0.0, 1280.0, 720.0, true)];
        let placement = place_window(&geometry(0.0, 0.0, 2400.0, 1400.0), &displays).unwrap();
        assert_eq!(placement.width, 1280.0);
        assert_eq!(placement.height, 720.0);
        assert!(placement.clamped);
    }

    #[test]
    fn a_window_straddling_two_displays_uses_the_one_it_covers_most() {
        let displays = [
            display(0.0, 0.0, 1920.0, 1080.0, true),
            display(1920.0, 0.0, 1920.0, 1080.0, false),
        ];
        // 200 px on the first display, 1000 px on the second.
        let placement = place_window(&geometry(1720.0, 100.0, 1200.0, 800.0), &displays).unwrap();
        assert!(placement.left >= 1920.0);
        assert!(placement.clamped);
    }

    #[test]
    fn a_display_with_a_work_area_keeps_windows_out_of_the_taskbar() {
        let displays = [DisplayInfo {
            bounds: Rect {
                left: 0.0,
                top: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            work_area: Some(Rect {
                left: 0.0,
                top: 0.0,
                width: 1920.0,
                height: 1040.0,
            }),
            scale: Some(1.5),
            primary: true,
        }];
        let placement = place_window(&geometry(0.0, 900.0, 800.0, 400.0), &displays).unwrap();
        assert_eq!(placement.top, 640.0);
        assert!(placement.clamped);
    }

    #[test]
    fn describing_a_window_records_its_display_without_an_id() {
        let displays = [
            display(0.0, 0.0, 1920.0, 1080.0, false),
            display(1920.0, 0.0, 2560.0, 1440.0, true),
        ];
        let described = describe_display(&geometry(2000.0, 100.0, 1200.0, 800.0), &displays);
        assert_eq!(
            described.display,
            Some(Rect {
                left: 1920.0,
                top: 0.0,
                width: 2560.0,
                height: 1440.0
            })
        );
        assert_eq!(described.display_scale, Some(1.0));
    }

    #[test]
    fn a_capture_without_bounds_has_nothing_to_place() {
        let displays = [display(0.0, 0.0, 1920.0, 1080.0, true)];
        let partial = WindowGeometry {
            left: Some(80.0),
            top: Some(40.0),
            ..WindowGeometry::default()
        };
        assert!(place_window(&partial, &displays).is_none());
        assert!(place_window(&geometry(0.0, 0.0, 100.0, 100.0), &[]).is_none());
    }

    #[test]
    fn display_area_uses_the_work_area() {
        let with_work_area = DisplayInfo {
            bounds: Rect {
                left: 0.0,
                top: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            work_area: Some(Rect {
                left: 0.0,
                top: 0.0,
                width: 1920.0,
                height: 1000.0,
            }),
            scale: None,
            primary: true,
        };
        assert_eq!(display_area(&with_work_area), 1920.0 * 1000.0);
    }
}
