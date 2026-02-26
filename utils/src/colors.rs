use rand::seq::IndexedRandom;
use std::sync::{
    LazyLock,
    atomic::{AtomicUsize, Ordering},
};

// ANSI extended color range:
// https://www.ditig.com/publications/256-colors-cheat-sheet
//
// The following ANSI color codes are exactly the color codes that have a contrast ratio of
// at least 4.0 on both white and black backgrounds, as defined by WCAG 2.2:
// https://www.w3.org/TR/WCAG22/#dfn-contrast-ratio
// This ensures that the colors are legible in both light and dark mode.
// (WCAG 2.2 requires a contrast ratio of 4.5 for accessibility, but there are too few colors
// that meet that requirement on both white and black backgrounds.)
const MID_COLORS: [u8; 22] = [
    27, 28, 29, 30, 31, 62, 63, 64, 65, 96, 97, 98, 99, 129, 130, 131, 132, 133, 161, 162, 163, 164,
];

/// Measures taxicab distance between two colors in the 6x6x6 ANSI 8-bit color cube.
///
/// The output is only meaningful if the inputs are both in the range `16..232`.
const fn ansi_color_diff(color1: u8, color2: u8) -> u8 {
    let diff = color1.abs_diff(color2);
    diff % 6 + (diff / 6) % 6 + diff / 36
}

// Ensure each selected color differs from the last `RECENT_COLOR_WINDOW` selected colors
// by at least `MAX_REQUIRED_COLOR_DIFF` units in the 6x6x6 ANSI 8-bit color cube if
// possible, or if not possible, differs by as much as possible.
const RECENT_COLOR_WINDOW: usize = 3;
const MAX_REQUIRED_COLOR_DIFF: u8 = 4;

fn shuffled_mid_colors() -> Vec<u8> {
    let mut selected_colors = Vec::with_capacity(MID_COLORS.len());
    let mut remaining_colors = Vec::from(&MID_COLORS);
    let mut rng = rand::rng();
    // Temporary buffer to hold possible color choices at each step.
    let mut choice_buf = Vec::with_capacity(MID_COLORS.len());
    for i in 0..MID_COLORS.len() {
        let recent_colors = &selected_colors[i.saturating_sub(RECENT_COLOR_WINDOW)..i];
        // Minimum distance from recently chosen colors. This starts at `MAX_REQUIRED_COLOR_DIFF`
        // and decreases only if necessary to prevent the list of choices from being empty.
        let min_color_diff = remaining_colors
            .iter()
            .map(|remaining| {
                recent_colors
                    .iter()
                    .map(|recent| ansi_color_diff(*remaining, *recent))
                    .min()
                    .unwrap_or(u8::MAX)
            })
            .max()
            .expect("list of remaining colors should not be empty")
            .min(MAX_REQUIRED_COLOR_DIFF);
        choice_buf.extend(remaining_colors.iter().copied().filter(|candidate| {
            recent_colors
                .iter()
                .all(|recent_color| ansi_color_diff(*candidate, *recent_color) >= min_color_diff)
        }));
        let selection = *choice_buf
            .choose(&mut rng)
            .expect("there should be at least one color choice");
        choice_buf.clear();
        selected_colors.push(selection);
        remaining_colors.retain(|color| *color != selection);
    }
    selected_colors
}

static SHUFFLED_COLORS: LazyLock<Vec<u8>> = LazyLock::new(shuffled_mid_colors);

/// Generate random ANSI colors that are legible on both light and dark backgrounds.
///
/// More precisely, all generated colors have a contrast ratio of at least 4.0 (as defined by
/// WCAG 2.2) on both white and black backgrounds.
///
/// This function internally keeps track of state and will cycle through all such colors in a
/// random order before repeating colors. The ordering is not *uniformly* random, but is chosen so
/// that each color is not overly similar to the previous few colors returned.
#[must_use]
pub fn gen_random_ansi_color() -> u8 {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let index = COUNTER.fetch_add(1, Ordering::Relaxed) % MID_COLORS.len();
    SHUFFLED_COLORS[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    const NUM_TRIES: u32 = 100;

    #[test]
    fn all_mid_colors_selected() {
        for _ in 0..NUM_TRIES {
            let mut shuffled_colors = shuffled_mid_colors();
            shuffled_colors.sort_unstable();
            assert_eq!(shuffled_colors, MID_COLORS);
        }
    }

    #[test]
    fn consecutive_colors_not_too_similar() {
        for _ in 0..NUM_TRIES {
            let shuffled_colors = shuffled_mid_colors();
            for i in 0..MID_COLORS.len() / 3 {
                for j in i.saturating_sub(RECENT_COLOR_WINDOW)..i {
                    let color_diff = ansi_color_diff(shuffled_colors[i], shuffled_colors[j]);
                    assert!(color_diff >= MAX_REQUIRED_COLOR_DIFF);
                }
            }
        }
    }

    #[test]
    fn colors_cycle() {
        for i in 0..MID_COLORS.len() * 2 {
            let next_color = SHUFFLED_COLORS[i % MID_COLORS.len()];
            assert_eq!(next_color, gen_random_ansi_color());
        }
    }
}
