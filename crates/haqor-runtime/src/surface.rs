//! Shared normalization for pointed Hebrew surface forms.

/// Canonical combining class for the niqqud points [`normalize_surface`] keeps.
fn combining_class(c: char) -> u8 {
    match c as u32 {
        0x05B0 => 10,
        0x05B1 => 11,
        0x05B2 => 12,
        0x05B3 => 13,
        0x05B4 => 14,
        0x05B5 => 15,
        0x05B6 => 16,
        0x05B7 => 17,
        0x05B8 | 0x05C7 => 18,
        0x05B9 => 19,
        0x05BB => 20,
        0x05BC => 21,
        0x05C1 => 24,
        0x05C2 => 25,
        _ => 0,
    }
}

/// Reduce a raw token to the letters and niqqud consumed by the morphology
/// parser. Cantillation and other marks are dropped, and combining marks are
/// put in canonical order so equivalent source spellings share one key.
pub fn normalize_surface(token: &str) -> String {
    let kept: Vec<char> = token
        .chars()
        .filter(|&c| {
            let n = c as u32;
            (0x05D0..=0x05EA).contains(&n)
                || matches!(
                    n,
                    0x05B0..=0x05B9 | 0x05BB | 0x05BC | 0x05C1 | 0x05C2 | 0x05C7
                )
        })
        .collect();

    let mut out = String::with_capacity(kept.len() * 2);
    let mut i = 0;
    while i < kept.len() {
        if combining_class(kept[i]) == 0 {
            out.push(kept[i]);
            i += 1;
        } else {
            let start = i;
            while i < kept.len() && combining_class(kept[i]) != 0 {
                i += 1;
            }
            let mut run = kept[start..i].to_vec();
            run.sort_by_key(|&c| combining_class(c));
            out.extend(run);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::normalize_surface;

    #[test]
    fn removes_cantillation_and_orders_niqqud() {
        assert_eq!(normalize_surface("שָׁלַ֖ח"), "שָׁלַח");
    }
}
