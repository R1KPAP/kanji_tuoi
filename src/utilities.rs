use crate::app::RangeFilter;

pub fn format_range<T: std::fmt::Display>(range: Option<&RangeFilter<T>>) -> (String, String) {
    let min = range
        .and_then(|r| r.min.as_ref())
        .map_or("_".to_string(), |v| v.to_string());
    let max = range
        .and_then(|r| r.max.as_ref())
        .map_or("_".to_string(), |v| v.to_string());
    (min, max)
}

pub fn backspace_numeric<T: std::fmt::Display + std::str::FromStr>(
    val: &mut Option<T>,
    cursor_pos: usize,
) -> bool {
    if cursor_pos == 0 {
        return false;
    }

    if let Some(current) = val {
        let mut s = current.to_string();
        let char_idx = cursor_pos - 1;

        if let Some((byte_idx, _)) = s.char_indices().nth(char_idx) {
            s.remove(byte_idx);

            if s.is_empty() {
                *val = None;
            } else if let Ok(new_val) = s.parse::<T>() {
                *val = Some(new_val);
            }
            return true;
        }
    }
    false
}

pub fn delete_numeric<T: std::fmt::Display + std::str::FromStr>(
    val: &mut Option<T>,
    cursor_pos: usize,
) {
    if let Some(current) = val {
        let mut s = current.to_string();

        if let Some((byte_idx, _)) = s.char_indices().nth(cursor_pos) {
            s.remove(byte_idx);

            if s.is_empty() {
                *val = None;
            } else if let Ok(new_val) = s.parse::<T>() {
                *val = Some(new_val);
            }
        }
    }
}