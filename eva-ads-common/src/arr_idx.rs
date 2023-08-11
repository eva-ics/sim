use crate::types::AdsError;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct ArrayIndex {
    index: u32,
    length: Option<u32>,
}

impl ArrayIndex {
    pub fn index(&self) -> u32 {
        self.index
    }
    pub fn length(&self) -> Option<u32> {
        self.length
    }
}

pub fn parse_array_index(path: &str) -> Result<(&str, Option<ArrayIndex>), AdsError> {
    if let Some(p) = path.strip_suffix(']') {
        if let Some(pos) = path.rfind('[') {
            let mut sp = p[pos + 1..].split('-');
            let idx_start = sp.next().unwrap();
            let idx_end_o = sp.next();
            let start = idx_start.parse().map_err(|_| AdsError::InvalidArrayIndex)?;
            let (end_o, default_length) = if let Some(idx_end) = idx_end_o {
                (
                    Some(
                        idx_end
                            .parse::<u32>()
                            .map_err(|_| AdsError::InvalidArrayIndex)?,
                    ),
                    None,
                )
            } else {
                (None, Some(1))
            };
            let length: Option<u32> = if let Some(end) = end_o {
                Some(if end < start { 0 } else { end + 1 - start })
            } else {
                default_length
            };
            Ok((
                &p[..pos],
                Some(ArrayIndex {
                    index: start,
                    length,
                }),
            ))
        } else {
            Err(AdsError::InvalidArrayIndex)
        }
    } else {
        Ok((path, None))
    }
}

#[cfg(test)]
mod test {
    use super::parse_array_index;
    #[test]
    fn array_index() {
        let path = "some.var";
        let (name, idx) = parse_array_index(path).unwrap();
        assert_eq!(name, "some.var");
        assert_eq!(idx, None);
        let path = "some.var[4]";
        let (name, idx) = parse_array_index(path).unwrap();
        assert_eq!(name, "some.var");
        assert_eq!(idx.unwrap().index, 4);
        assert_eq!(idx.unwrap().length, Some(1));
        let path = "some.var[0-4]";
        let (name, idx) = parse_array_index(path).unwrap();
        assert_eq!(name, "some.var");
        assert_eq!(idx.unwrap().index, 0);
        assert_eq!(idx.unwrap().length, Some(5));
        let path = "some.var[4-4]";
        let (name, idx) = parse_array_index(path).unwrap();
        assert_eq!(name, "some.var");
        assert_eq!(idx.unwrap().index, 4);
        assert_eq!(idx.unwrap().length, Some(1));
        let path = "some.var[4-8]";
        let (name, idx) = parse_array_index(path).unwrap();
        assert_eq!(name, "some.var");
        assert_eq!(idx.unwrap().index, 4);
        assert_eq!(idx.unwrap().length, Some(5));
    }
}
