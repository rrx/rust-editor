use ropey::{iter::Chunks, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// See: https://github.com/helix-editor/helix/blob/master/helix-core/src/graphemes.rs
pub fn grapheme_width(g: &str) -> usize {
    g.chars().fold(0, | acc, x | {
        if g.as_bytes()[0] <= 127 {
            // Fast-path ascii.
            // Point 1: theoretically, ascii control characters should have zero
            // width, but in our case we actually want them to have width: if they
            // show up in text, we want to treat them as textual elements that can
            // be editied.  So we can get away with making all ascii single width
            // here.
            // Point 2: we're only examining the first codepoint here, which means
            // we're ignoring graphemes formed with combining characters.  However,
            // if it starts with ascii, it's going to be a single-width grapeheme
            // regardless, so, again, we can get away with that here.
            // Point 3: we're only examining the first _byte_.  But for utf8, when
            // checking for ascii range values only, that works.
            acc + 1
        } else {
            // We use max(1) here because all grapeheme clusters--even illformed
            // ones--should have at least some width so they can be edited
            // properly.
            acc + UnicodeWidthStr::width(g).max(1)
        }
    })
}


/// An implementation of a graphemes iterator, for iterating over
/// the graphemes of a RopeSlice.
pub struct RopeGraphemes<'a> {
    text: RopeSlice<'a>,
    chunks: Chunks<'a>,
    cur_chunk: &'a str,
    cur_chunk_start: usize,
    cursor: GraphemeCursor,
}

impl<'a> RopeGraphemes<'a> {
    pub fn new<'b>(slice: &RopeSlice<'b>) -> RopeGraphemes<'b> {
        let mut chunks = slice.chunks();
        let first_chunk = chunks.next().unwrap_or("");
        RopeGraphemes {
            text: *slice,
            chunks: chunks,
            cur_chunk: first_chunk,
            cur_chunk_start: 0,
            cursor: GraphemeCursor::new(0, slice.len_bytes(), true),
        }
    }
}


impl<'a> Iterator for RopeGraphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<RopeSlice<'a>> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .next_boundary(self.cur_chunk, self.cur_chunk_start)
            {
                Ok(None) => {
                    return None;
                }
                Ok(Some(n)) => {
                    b = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    self.cur_chunk_start += self.cur_chunk.len();
                    self.cur_chunk = self.chunks.next().unwrap_or("");
                }
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx, _, _) = self.text.chunk_at_byte(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a < self.cur_chunk_start {
            let a_char = self.text.byte_to_char(a);
            let b_char = self.text.byte_to_char(b);

            Some(self.text.slice(a_char..b_char))
        } else {
            let a2 = a - self.cur_chunk_start;
            let b2 = b - self.cur_chunk_start;
            Some((&self.cur_chunk[a2..b2]).into())
        }
    }
}

#[cfg(test)]
#[rustfmt::skip] // Because of the crazy long graphemes
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn iter_huge_graphemes() {
        let r = Rope::from_str("Hẽ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃llõ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃ wõ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃rld!");
        let mut grph = RopeGraphemes::new(&r.slice(..));

        assert_eq!(grph.next().unwrap(), "H");
        assert_eq!(grph.next().unwrap(), "ẽ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃");
        assert_eq!(grph.next().unwrap(), "l");
        assert_eq!(grph.next().unwrap(), "l");
        assert_eq!(grph.next().unwrap(), "õ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃");
        assert_eq!(grph.next().unwrap(), " ");
        assert_eq!(grph.next().unwrap(), "w");
        assert_eq!(grph.next().unwrap(), "õ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃");
        assert_eq!(grph.next().unwrap(), "r");
        assert_eq!(grph.next().unwrap(), "l");
        assert_eq!(grph.next().unwrap(), "d");
        assert_eq!(grph.next().unwrap(), "!");
        assert_eq!(grph.next(), None);
    }

    #[test]
    fn iter_regional_symbols() {
        let r = Rope::from_str("🇬🇧🇯🇵🇺🇸🇫🇷🇷🇺🇨🇳🇩🇪🇪🇸🇬🇧🇯🇵🇺🇸🇫🇷🇷🇺🇨🇳🇩🇪🇪🇸🇬🇧🇯🇵🇺🇸🇫🇷🇷🇺🇨🇳🇩🇪🇪🇸");
        let mut grph = RopeGraphemes::new(&r.slice(..));

        assert_eq!(grph.next().unwrap(), "🇬🇧");
        assert_eq!(grph.next().unwrap(), "🇯🇵");
        assert_eq!(grph.next().unwrap(), "🇺🇸");
        assert_eq!(grph.next().unwrap(), "🇫🇷");
        assert_eq!(grph.next().unwrap(), "🇷🇺");
        assert_eq!(grph.next().unwrap(), "🇨🇳");
        assert_eq!(grph.next().unwrap(), "🇩🇪");
        assert_eq!(grph.next().unwrap(), "🇪🇸");
        assert_eq!(grph.next().unwrap(), "🇬🇧");
        assert_eq!(grph.next().unwrap(), "🇯🇵");
        assert_eq!(grph.next().unwrap(), "🇺🇸");
        assert_eq!(grph.next().unwrap(), "🇫🇷");
        assert_eq!(grph.next().unwrap(), "🇷🇺");
        assert_eq!(grph.next().unwrap(), "🇨🇳");
        assert_eq!(grph.next().unwrap(), "🇩🇪");
        assert_eq!(grph.next().unwrap(), "🇪🇸");
        assert_eq!(grph.next().unwrap(), "🇬🇧");
        assert_eq!(grph.next().unwrap(), "🇯🇵");
        assert_eq!(grph.next().unwrap(), "🇺🇸");
        assert_eq!(grph.next().unwrap(), "🇫🇷");
        assert_eq!(grph.next().unwrap(), "🇷🇺");
        assert_eq!(grph.next().unwrap(), "🇨🇳");
        assert_eq!(grph.next().unwrap(), "🇩🇪");
        assert_eq!(grph.next().unwrap(), "🇪🇸");
        assert_eq!(grph.next(), None);
    }
}

