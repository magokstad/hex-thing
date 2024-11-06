use std::str::FromStr;

#[derive(Debug, Clone, Default)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

impl FromStr for ByteRange {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 2 {
            return Err("Range must be in the format 'start-end'");
        }

        let start = usize::from_str_radix(parts[0].trim_start_matches("0x"), 16)
            .or_else(|_| usize::from_str(parts[0]));
        let end = usize::from_str_radix(parts[1].trim_start_matches("0x"), 16)
            .or_else(|_| usize::from_str(parts[1]));

        let (start, end) = match (start, end) {
            (Ok(s), Ok(e)) => (s, e),
            _ => return Err("Range entries must either be in the format '0xFF' or '255'"),
        };

        Ok(ByteRange { start, end })
    }
}
