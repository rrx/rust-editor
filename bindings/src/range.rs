use log::*;
use std::convert::From;
use super::helpers::*;
use editor_core::{MacroId, Command, Register, Motion, Mode, Macros};
use nom::combinator;
use crossterm::event::Event;
use std::convert::TryFrom;

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub enum Elem {
    Char(char),
    Alt(char),
    Control(char),
    Resize(usize, usize),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Backspace,
    Delete,
    Tab,
}

impl Elem {
    fn is_digit(&self, radix: u32) -> bool {
        match self {
            Self::Char(c) if c.is_digit(radix) => true,
            _ => false,
        }
    }

    fn into_char(&self) -> Option<char> {
        match self {
            Self::Char(c) => Some(*c),
            //Self::Enter => Some('\n'),
            Self::Tab => Some('\t'),
            _ => None,
        }
    }
}

pub fn range_enter(s: &str) -> Vec<Elem> {
    let mut v = range_string(s);
    v.push(Elem::Enter);
    v
}

pub fn range_string(s: &str) -> Vec<Elem> {
    s.chars().map(|x| Elem::Char(x)).collect::<Vec<Elem>>()
}

pub type Range<'a> = &'a [Elem];

pub struct R<'a>(Range<'a>);

impl<'a> R<'a> {
    pub fn take(count: usize) -> impl FnMut(Range) -> IResult<Range, Range> {
        move |i: Range| {
            if i.len() >= count {
                Ok((&i[count..], &i[..count]))
            } else {
                info!("take - incomplete: {:?}", i);
                Err(Err::Incomplete(Needed::Unknown))
            }
        }
    }

    // get a single char
    pub fn char() -> impl FnMut(Range) -> IResult<Range, char> {
        |i| Self::p_char(i)
    }

    pub fn p_char(i: Range) -> IResult<Range, char> {
        if i.len() == 0 {
            info!("char - incomplete: {:?}", i);
            return Err(Err::Incomplete(Needed::new(1)));
        }
        match i[0] {
            Elem::Char(ch) => Ok((&i[1..], ch)),
            _ => Err(Err::Error(Error::new(i, ErrorKind::Tag))),
        }
        //let s: String = i.iter().map(|t| t.into_char())
        //.take_while(|t| t.is_some())
        //.map(|t| t.unwrap())
        //.take(1)
        //.collect::<String>();
        //match s.chars().next() {
        //Some(c) => Ok((&i[1..], c)),
        //None => {
        //info!("char - incomplete: {:?}", s);
        //Err(Err::Incomplete(Needed::new(1)))
        //}
        //}
    }

    pub fn take_string(count: usize) -> impl FnMut(Range) -> IResult<Range, String> {
        move |i: Range| {
            let c = count.clone();
            let s = i
                .iter()
                .map(|t| t.into_char())
                .take_while(|t| t.is_some())
                .map(|t| t.unwrap())
                .take(c)
                .collect::<String>();
            Ok((&i[s.len()..], s))
        }
    }

    pub fn p_string(i: Range<'a>) -> IResult<Range<'a>, String> {
        let p = nom::multi::many0(R::char());
        combinator::map(p, |v| v.iter().collect::<String>())(i)
    }

    //fn p_string2(i: Range<'a>) -> IResult<Range<'a>, String> {
        //Self::take_string_while(|_x| true)(i)
    //}

    pub fn string() -> impl Fn(Range<'a>) -> IResult<Range<'a>, String> {
        |i| Self::p_string(i)
    }

    pub fn take_string_while(
        f: impl Fn(&char) -> bool,
    ) -> impl Fn(Range<'a>) -> IResult<Range<'a>, String> {
        move |i| Self::p_string_while(i, |x| f(x))
    }

    pub fn p_string_while(i: Range, f: impl Fn(&char) -> bool) -> IResult<Range, String> {
        let s = i
            .iter()
            .map(|t| t.into_char())
            .take_while(|t| t.is_some())
            .map(|t| t.unwrap())
            .take_while(f)
            .collect::<String>();
        Ok((&i[s.len()..], s))
    }

    pub fn tag_string(r: &str) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Range<'a>> {
        Self::tag_elem(range_string(r))
    }

    pub fn tag_elem(r: Vec<Elem>) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Range<'a>> {
        let s = r.clone();
        move |i| {
            let len = s.len();
            let s_incomplete = &s[..std::cmp::min(len, i.len())];
            if i.len() < s.len() && s_incomplete == i {
                info!("tag_elem - incomplete: {:?}", s);
                Err(Err::Incomplete(Needed::new(s.len() - i.len())))
            } else if i.len() >= s.len() && &i[..len] == r {
                Ok((&i[len..], &i[..len]))
            } else {
                Err(Err::Error(Error::new(i, ErrorKind::Tag)))
            }
        }
    }

    pub fn tag(r: Range<'a>) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Range<'a>> {
        let s = &(*r);
        move |i| {
            let len = s.len();
            let s_incomplete = &s[..std::cmp::min(len, i.len())];
            if i.len() < s.len() && s_incomplete == i {
                info!("tag - incomplete: {:?}", s);
                Err(Err::Incomplete(Needed::new(s.len() - i.len())))
            } else if i.len() >= s.len() && &i[..len] == r {
                Ok((&i[len..], &i[..len]))
            } else {
                Err(Err::Error(Error::new(i, ErrorKind::Tag)))
            }
        }
    }

    pub fn number<O>() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, O>
    where
        O: std::str::FromStr,
    {
        |i| Self::p_number(i)
    }
    pub fn p_number<O>(i: Range<'a>) -> IResult<Range<'a>, O>
    where
        O: std::str::FromStr,
    {
        let digit = Self::take_string_while(|e| e.is_digit(10));
        map_opt(digit, |s: String| s.parse::<O>().ok())(i)
    }

    pub fn oneof(choices: Range<'a>) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Elem> {
        move |i: Range| {
            let ch = &(*choices);
            match i.iter().next().map(|c| (c, ch.iter().find(|e| *e == c))) {
                None => {
                    //info!("oneof - incomplete: {:?}", &choices);
                    Err(Err::Incomplete(Needed::new(1)))
                }
                Some((_, None)) => Err(Err::Error(Error::new(i, ErrorKind::OneOf))),
                Some((_, Some(_))) => Ok((&i[1..], i[0])),
            }
        }
    }

    pub fn string_until(until: Range<'a>) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, String> {
        move |i| {
            match tuple((R::string(), R::tag(until)))(i) {
                Ok((rest, (s, _x))) => Ok((rest, s)),
                Err(e) => Err(e),
            }
            //Self::p_string_until(i, until)
        }
    }

    //fn p_string_until(i: Range, until: Elem) -> IResult<Range, String> {
    //match tuple((R::string(), R::tag(&[until])))(i) {
    //Ok((rest, (s, _))) => Ok((rest, s)),
    //Err(e) => Err(e)
    //}

    ////let s = i.iter()
    ////.take_while(|t| **t != until)
    ////.filter_map(|t| t.into_char())
    ////.collect::<String>();
    ////Ok((&i[s.len()..], s))
    //}
    //
}
