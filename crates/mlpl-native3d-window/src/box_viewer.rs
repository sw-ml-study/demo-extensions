//! Bounded, application-neutral presentation state for filled-box viewers.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

const SCHEMA: &str = "sw-ml-study.native3d.box-presentation";
const MAX_ITEMS: usize = 100_000;
const MAX_MODES: usize = 9;
const MAX_LEGEND: usize = 32;
const MAX_TEXT: usize = 256;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    version: u32,
    title: String,
    rotation_speed: f32,
    ids: Vec<u64>,
    labels: Vec<String>,
    details: Vec<String>,
    mode_labels: Vec<String>,
    mode_colors: Vec<Vec<[f32; 4]>>,
    view_labels: Vec<String>,
    view_centers: Vec<Vec<[f32; 3]>>,
    view_sizes: Vec<Vec<[f32; 3]>>,
    legend_mode: Vec<usize>,
    legend_labels: Vec<String>,
    legend_colors: Vec<[f32; 4]>,
}

/// Validated presentation and current interaction state.
#[derive(Clone, Debug)]
pub struct BoxViewer {
    document: Document,
    item_by_id: BTreeMap<u64, usize>,
    mode: usize,
    view: usize,
    highlight: Option<usize>,
    rotating: bool,
    selected: Option<u64>,
}

impl BoxViewer {
    /// Parses bounded generic labels, legends, and parallel color modes.
    ///
    /// # Errors
    ///
    /// Rejects malformed, unsupported, unbounded, non-finite, or misaligned data.
    pub fn parse(source: &str, scene_ids: &[u64]) -> Result<Self, String> {
        let document: Document = serde_json::from_str(source)
            .map_err(|error| format!("malformed box presentation: {error}"))?;
        if document.schema != SCHEMA || document.version != 1 {
            return Err("unsupported box presentation schema or version".into());
        }
        check_text(&document.title)?;
        if !document.rotation_speed.is_finite() || document.rotation_speed < 0.0 {
            return Err("rotation_speed must be finite and nonnegative".into());
        }
        if document.ids.len() != scene_ids.len()
            || document.labels.len() != scene_ids.len()
            || document.details.len() != scene_ids.len()
            || document.ids.len() > MAX_ITEMS
        {
            return Err("presentation items must align with scene ids".into());
        }
        if document.mode_labels.is_empty()
            || document.mode_labels.len() > MAX_MODES
            || document.mode_colors.len() != document.mode_labels.len()
        {
            return Err("presentation requires one to nine color modes".into());
        }
        if document.view_labels.is_empty()
            || document.view_labels.len() > MAX_MODES
            || document.view_centers.len() != document.view_labels.len()
            || document.view_sizes.len() != document.view_labels.len()
        {
            return Err("presentation requires one to nine aligned views".into());
        }
        for ((label, centers), sizes) in document
            .view_labels
            .iter()
            .zip(&document.view_centers)
            .zip(&document.view_sizes)
        {
            check_text(label)?;
            if centers.len() != scene_ids.len() || sizes.len() != scene_ids.len() {
                return Err("view geometry must align with boxes".into());
            }
            if centers.iter().flatten().any(|value| !value.is_finite())
                || sizes
                    .iter()
                    .flatten()
                    .any(|value| !value.is_finite() || *value <= 0.0)
            {
                return Err("view geometry must be finite with positive sizes".into());
            }
        }
        let expected: BTreeSet<_> = scene_ids.iter().copied().collect();
        let actual: BTreeSet<_> = document.ids.iter().copied().collect();
        if actual.len() != document.ids.len() || actual != expected {
            return Err("presentation item ids must uniquely match scene ids".into());
        }
        for text in document.labels.iter().chain(&document.details) {
            check_text(text)?;
        }
        for (label, colors) in document.mode_labels.iter().zip(&document.mode_colors) {
            check_text(label)?;
            if colors.len() != scene_ids.len() {
                return Err("color modes must align with boxes".into());
            }
            for color in colors {
                if color
                    .iter()
                    .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
                {
                    return Err("presentation colors must be finite RGBA values".into());
                }
            }
        }
        if document.legend_mode.len() != document.legend_labels.len()
            || document.legend_mode.len() != document.legend_colors.len()
            || document.legend_mode.len() > MAX_LEGEND
            || document
                .legend_mode
                .iter()
                .any(|mode| *mode >= document.mode_labels.len())
        {
            return Err("legend columns must align and reference known modes".into());
        }
        for (label, color) in document.legend_labels.iter().zip(&document.legend_colors) {
            check_text(label)?;
            if color
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            {
                return Err("presentation colors must be finite RGBA values".into());
            }
        }
        let item_by_id = item_index(&document.ids);
        Ok(Self {
            document,
            item_by_id,
            mode: 0,
            view: 0,
            highlight: None,
            rotating: false,
            selected: None,
        })
    }

    #[must_use]
    pub fn colors(&self) -> Vec<[f32; 4]> {
        let mut colors = self.document.mode_colors[self.mode].clone();
        let Some(highlight) = self.highlight else {
            return colors;
        };
        let target = self.legend()[highlight].1;
        for color in &mut colors {
            if color[..3] != target[..3] {
                color[3] = color[3].min(0.12);
            }
        }
        colors
    }

    #[must_use]
    pub fn geometry(&self) -> (&[[f32; 3]], &[[f32; 3]]) {
        (
            &self.document.view_centers[self.view],
            &self.document.view_sizes[self.view],
        )
    }

    #[must_use]
    pub fn legend(&self) -> Vec<(String, [f32; 4])> {
        self.document
            .legend_mode
            .iter()
            .zip(&self.document.legend_labels)
            .zip(&self.document.legend_colors)
            .filter(|((mode, _), _)| **mode == self.mode)
            .map(|((_, label), color)| (label.clone(), *color))
            .collect()
    }

    #[must_use]
    pub fn rotation_speed(&self) -> f32 {
        if self.rotating {
            self.document.rotation_speed
        } else {
            0.0
        }
    }

    pub fn select(&mut self, id: Option<u64>) {
        self.selected = id.filter(|candidate| self.item_by_id.contains_key(candidate));
    }

    /// Applies a mode digit or rotation toggle, returning whether it was handled.
    pub fn key(&mut self, key: &str) -> bool {
        if key == "r" {
            self.rotating = !self.rotating;
            return true;
        }
        if key == "v" {
            self.view = (self.view + 1) % self.document.view_labels.len();
            return true;
        }
        if key == "h" {
            let count = self.legend().len();
            if count > 0 {
                self.highlight = Some(self.highlight.map_or(0, |index| (index + 1) % count));
            }
            return true;
        }
        if key == "c" {
            self.highlight = None;
            return true;
        }
        let Some(index) = key
            .parse::<usize>()
            .ok()
            .and_then(|value| value.checked_sub(1))
        else {
            return false;
        };
        if index < self.document.mode_labels.len() {
            self.mode = index;
            self.highlight = None;
            true
        } else {
            false
        }
    }

    /// Hit-tests the visible top-row controls in physical window pixels.
    pub fn click(&mut self, point: [f64; 2]) -> bool {
        if (10.0..=38.0).contains(&point[1]) {
            let mut left = 12.0;
            for (index, label) in self.document.mode_labels.iter().enumerate() {
                let width = u32::try_from(label.chars().count()).unwrap_or(u32::MAX);
                let right = left + 34.0 + f64::from(width) * 9.0;
                if (left..=right).contains(&point[0]) {
                    self.mode = index;
                    return true;
                }
                left = right + 8.0;
            }
            let right = left + 124.0;
            if (left..=right).contains(&point[0]) {
                self.rotating = !self.rotating;
                return true;
            }
        }
        if !(42.0..=70.0).contains(&point[1]) {
            return false;
        }
        let mut left = 12.0;
        for (index, label) in self.document.view_labels.iter().enumerate() {
            let width = u32::try_from(label.chars().count()).unwrap_or(u32::MAX);
            let right = left + 34.0 + f64::from(width) * 9.0;
            if (left..=right).contains(&point[0]) {
                self.view = index;
                return true;
            }
            left = right + 8.0;
        }
        false
    }

    #[must_use]
    pub fn overlay(&self) -> String {
        let buttons = self
            .document
            .mode_labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                if index == self.mode {
                    format!("[{} {}*]", index + 1, label)
                } else {
                    format!("[{} {}]", index + 1, label)
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let rotation = if self.rotating { "ON" } else { "OFF" };
        let views = self
            .document
            .view_labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                if index == self.view {
                    format!("[{label}*]")
                } else {
                    format!("[{label}]")
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let selection = self
            .selected
            .and_then(|id| self.item_by_id.get(&id))
            .map_or_else(
                || "SELECTED: none — click a box".to_owned(),
                |index| {
                    format!(
                        "SELECTED {}: {}\n{}",
                        self.document.ids[*index],
                        self.document.labels[*index],
                        self.document.details[*index]
                    )
                },
            );
        let emphasis = self
            .highlight
            .map_or_else(|| "all".to_owned(), |index| self.legend()[index].0.clone());
        format!(
            "{}\nCOLOR {} [H HIGHLIGHT: {}] [C CLEAR] [R ROTATE: {}]\nVIEW {} [V NEXT]\n{}\nDRAG ORBIT | SHIFT+DRAG PAN | WHEEL ZOOM | ESC CLOSE",
            self.document.title, buttons, emphasis, rotation, views, selection
        )
    }
}

fn check_text(text: &str) -> Result<(), String> {
    if text.is_empty() || text.chars().count() > MAX_TEXT || text.chars().any(char::is_control) {
        return Err("presentation text must be nonempty, printable, and bounded".into());
    }
    Ok(())
}

fn item_index(ids: &[u64]) -> BTreeMap<u64, usize> {
    ids.iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::BoxViewer;

    const VALID: &str = r#"{"schema":"sw-ml-study.native3d.box-presentation","version":1,"title":"Layout","rotation_speed":0.25,"ids":[7,9],"labels":["Header","Image"],"details":["8 bytes","48 bytes"],"mode_labels":["Kind","Owner"],"mode_colors":[[[1,0,0,1],[0,1,0,1]],[[0,0,1,1],[0,0,1,1]]],"view_labels":["Overview","Physical"],"view_centers":[[[0,0,0],[1,0,0]],[[0,0,0],[0,1,0]]],"view_sizes":[[[1,1,1],[1,1,1]],[[1,1,1],[1,2,1]]],"legend_mode":[0,1],"legend_labels":["header","kernel"],"legend_colors":[[1,0,0,1],[0,0,1,1]]}"#;

    #[test]
    fn defaults_static_and_exposes_selection_legend_and_modes() {
        let mut viewer = BoxViewer::parse(VALID, &[7, 9]).unwrap();
        assert!(viewer.rotation_speed().abs() < f32::EPSILON);
        assert_eq!(viewer.legend()[0].0, "header");
        viewer.select(Some(9));
        assert!(viewer.overlay().contains("SELECTED 9: Image\n48 bytes"));
        assert!(viewer.key("2"));
        assert!(
            viewer.colors()[0]
                .into_iter()
                .zip([0.0, 0.0, 1.0, 1.0])
                .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
        );
        assert!(viewer.key("r"));
        assert!((viewer.rotation_speed() - 0.25).abs() < f32::EPSILON);
        assert!(viewer.key("v"));
        assert!(
            viewer.geometry().1[1]
                .into_iter()
                .zip([1.0, 2.0, 1.0])
                .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
        );
        assert!(viewer.key("1"));
        assert!(viewer.key("h"));
        assert!((viewer.colors()[1][3] - 0.12).abs() < f32::EPSILON);
        assert!(viewer.overlay().contains("H HIGHLIGHT: header"));
        assert!(viewer.key("c"));
        assert!((viewer.colors()[1][3] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn visible_controls_are_clickable_and_bad_contracts_fail_closed() {
        let mut viewer = BoxViewer::parse(VALID, &[7, 9]).unwrap();
        assert!(viewer.click([120.0, 20.0]));
        assert!(viewer.overlay().contains("[2 Owner*]"));
        assert!(!viewer.click([120.0, 80.0]));
        assert!(viewer.click([200.0, 55.0]));
        assert!(viewer.overlay().contains("[Physical*]"));
        assert!(BoxViewer::parse(VALID, &[7]).is_err());
        assert!(BoxViewer::parse(&VALID.replace("0.25", "-1"), &[7, 9]).is_err());
    }
}
