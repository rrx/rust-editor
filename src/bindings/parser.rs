use std::convert::From;
use log::*;

use super::helpers::*;
use crate::ism::{Mode, Command};

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub enum Elem {
    Char(char),
    Control(char),
    Resize(usize,usize),
    Enter,
    Esc,
    Backspace,
    Delete,
    Tab
}
impl Elem {
    fn is_digit(&self, radix: u32) -> bool {
       match self {
           Self::Char(c) if c.is_digit(radix) => true,
           _ => false
       }
    }

    fn into_char(&self) -> Option<char> {
        match self {
            Self::Char(c) => Some(*c),
            //Self::Enter => Some('\n'),
            Self::Tab => Some('\t'),
            _ => None
        }
    }
}


#[derive(Debug)]
pub struct TokenError {}

impl TryInto<Command> for Event {
    type Error = TokenError;
    fn try_into(self) -> Result<Command, TokenError> {
        use crossterm::event::*;
        match self {
            Event::Resize(x,y) => {
                Ok(Command::Resize(x, y))
            }
            Event::Mouse(MouseEvent {kind, column, row, modifiers}) => {
                match kind {
                    MouseEventKind::ScrollUp => {
                        Ok(Command::Scroll(1))
                    }
                    MouseEventKind::ScrollDown => {
                        Ok(Command::Scroll(-1))
                    }
                    MouseEventKind::Moved => {
                        Ok(Command::Mouse(column, row))
                    }
                    _ => Err(TokenError{})
                }
            }
            Event::Key(KeyEvent { code: KeyCode::Char('m'), modifiers: KeyModifiers::CONTROL }) => {
                Ok(Command::Test)
            }
            Event::Key(KeyEvent { code: KeyCode::Char('r'), modifiers: KeyModifiers::CONTROL }) => {
                Ok(Command::Refresh)
            }
            Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL }) => {
                Ok(Command::Quit)
            }
            Event::Key(KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::CONTROL }) => {
                Ok(Command::Save)
            }
            _ => Err(TokenError{})
        }
    }
}

impl TryInto<Elem> for Event {
    type Error = TokenError;
    fn try_into(self) -> Result<Elem, TokenError> {
        match self {
            Event::Key(KeyEvent { code: KeyCode::Char(c), modifiers: KeyModifiers::CONTROL }) => {
                Ok(Elem::Control(c))
            }
            Event::Key(KeyEvent { code: KeyCode::Char(c), modifiers: KeyModifiers::NONE }) => {
                Ok(Elem::Char(c))
            }
            Event::Key(KeyEvent { code: KeyCode::Char(c), modifiers: KeyModifiers::SHIFT }) => {
                if c.is_ascii() {
                    Ok(Elem::Char(c.to_ascii_uppercase()))
                } else {
                    Ok(Elem::Char(c))
                }
            }
            Event::Key(KeyEvent { code, modifiers}) => {
                match code {
                    KeyCode::Enter => Ok(Elem::Enter),
                    KeyCode::Esc => Ok(Elem::Esc),
                    KeyCode::Backspace => Ok(Elem::Backspace),
                    KeyCode::Delete => Ok(Elem::Delete),
                    KeyCode::Tab => Ok(Elem::Tab),
                    _ => Err(TokenError{})
                }
            }
            _ => Err(TokenError{})
        }
    }
}
impl TryInto<Elem> for &Event {
    type Error = TokenError;
    fn try_into(self) -> Result<Elem, TokenError> {
        let event: Event = *self;
        event.try_into()
    }
}

type Range<'a> = &'a [Elem];

struct R<'a>(Range<'a>);

impl<'a> R<'a> {
    fn take(count: usize) -> impl FnMut(Range) -> IResult<Range, Range> {
        move |i: Range| {
            if i.len() >= count {
                Ok((&i[count..], &i[..count]))
            } else {
                Err(Err::Incomplete(Needed::Unknown))
            }
        }
    }

    fn char() -> impl FnMut(Range) -> IResult<Range, char> {
        |i| Self::p_char(i)
    }

    fn p_char(i: Range) -> IResult<Range, char> {
        let s: String = i.iter().map(|t| t.into_char())
            .take_while(|t| t.is_some())
            .map(|t| t.unwrap())
            .take(1)
            .collect::<String>();
        match s.chars().next() {
            Some(c) => Ok((&i[1..], c)),
            None => Err(Err::Incomplete(Needed::new(1)))
        }
    }

    fn take_string(count: usize) -> impl FnMut(Range) -> IResult<Range, String> {
        move |i: Range| {
            let c = count.clone();
            let s = i.iter()
                .map(|t| t.into_char())
                .take_while(|t| t.is_some())
                .map(|t| t.unwrap())
                .take(c)
                .collect::<String>();
            Ok((&i[s.len()..], s))
        }
    }

    fn p_string(i: Range<'a>) -> IResult<Range<'a>, String> {
        Self::take_string_while(|x| true)(i)
    }

    fn string() -> impl Fn(Range<'a>) -> IResult<Range<'a>, String> {
        |i| Self::p_string(i)
    }

    fn take_string_while(f: impl Fn(&char) -> bool) -> impl Fn(Range<'a>) -> IResult<Range<'a>, String> {
        move |i| Self::p_string_while(i, |x| f(x))
    }

    fn p_string_while(i: Range, f: impl Fn(&char) -> bool) -> IResult<Range, String> {
        let s = i.iter()
            .map(|t| t.into_char())
            .take_while(|t| t.is_some())
            .map(|t| t.unwrap())
            .take_while(f)
            .collect::<String>();
        Ok((&i[s.len()..], s))
    }


    fn tag(r: Range<'a>) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Range<'a>> {
        let s = r.clone();
        move |i| {
            let len = s.len();
            let s_incomplete = &s[..std::cmp::min(len, i.len())];
            if i.len() < s.len() && s_incomplete == i {
                Err(Err::Incomplete(Needed::new(s.len() - i.len())))
            } else if i.len() >= s.len() && &i[..len] == r {
                Ok((&i[len..], &i[..len]))
            } else {
                Err(Err::Error(Error::new(i, ErrorKind::Tag)))
            }
        }
    }

    pub fn number<O>() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, O>
    where O: std::str::FromStr
    {
        |i| {
            Self::p_number(i)
        }
    }
    pub fn p_number<O>(i: Range<'a>) -> IResult<Range<'a>, O>
    where O: std::str::FromStr
    {
        let digit = Self::take_string_while(|e| e.is_digit(10));
        map_opt(digit, |s: String| s.parse::<O>().ok())(i)
    }

    fn oneof(choices: Range<'a>) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Range<'a>> {
        move |i: Range|{
            let ch = choices.clone();
            match i.iter().next().map(|c| (c, ch.iter().find(|e| *e == c))) {
                None => Err(Err::Incomplete(Needed::new(1))),
                Some((_, None)) => Err(Err::Error(Error::new(i, ErrorKind::OneOf))),
                Some((_, Some(_))) => Ok((&i[1..], &i[0..1])),
            }
        }
    }

}

impl<'a> Mode {
    fn normal() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Command> {
        |i| Self::p_normal(i)
    }

    fn p_common(i: Range<'a>) -> IResult<Range<'a>, Command> {
        alt((
                value(Command::LineNav(0), R::oneof(&[Elem::Char('^'), Elem::Control('a')])),
                value(Command::LineNav(-1), R::oneof(&[Elem::Char('$'), Elem::Control('e')])),
                value(Command::ScrollPage(-1), R::oneof(&[Elem::Control('u')])),
                value(Command::ScrollPage(1), R::oneof(&[Elem::Control('d')])),
                value(Command::Scroll(-1), R::oneof(&[Elem::Control('f')])),
                value(Command::Scroll(1), R::oneof(&[Elem::Control('b')])),
        ))(i)
    }

    fn p_normal(i: Range<'a>) -> IResult<Range<'a>, Command> {
        alt((
                map(tuple((R::number(), R::oneof(&[Elem::Enter, Elem::Char('G')]))), |x| Command::Line(x.0)),
                map_opt(tuple((T::range(), R::oneof(&[Elem::Enter]))), |(x, _)| {
                    match x {
                        T::Range(_, b) => Some(Command::Line(b as i64)),
                        _ => None
                    }
                }),
                value(Command::Mode(Mode::Insert), R::tag(&[Elem::Char('i')])),
                value(Command::Line(0), R::tag(&[Elem::Char('G')])),
                value(Command::Line(1), R::tag(&[Elem::Char('g'), Elem::Char('g')])),
                value(Command::BufferNext, R::tag(&[Elem::Char(']')])),
                |i| Mode::p_common(i),
                T::motion(),
                value(Command::Quit, R::oneof(&[Elem::Char('q'), Elem::Control('c')]))
        ))(i)
    }

    fn insert() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Command> {
        |i| Self::p_insert(i)
    }

    fn p_insert(i: Range<'a>) -> IResult<Range<'a>, Command> {
        alt((
                map(complete(R::char()), |x| Command::Insert(x).into()),
                value(Command::Quit.into(), R::oneof(&[Elem::Char('q'), Elem::Control('c')])),
                value(Command::Mode(Mode::Normal).into(), R::oneof(&[Elem::Esc])),
                value(Command::Insert('\n'), R::tag(&[Elem::Enter])),
                map(R::take(1), |x| Command::Insert('x').into()),
        ))(i)
    }

    pub fn command(&self) -> impl FnMut(Range<'a>) -> IResult<Range<'a>, Command> {
        match self {
            Self::Normal => |i| Self::p_normal(i),
            Self::Insert => |i| Self::p_insert(i),
            Self::Easy => |i| Self::p_normal(i)
        }
    }
}


#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum Motion {
    Left, Right, Up, Down,
    // Line
    EOL, SOL, Line, AbsLine,
    NextWord, EOW, PrevWord, SOW,
    // start and end of buffer
    SOB, EOB
}
impl Motion {
    fn _next(i: Range) -> Option<Motion> {
        match i.iter().next() {
            Some(Elem::Char(c)) => {
                match c {
                    'h' => Some(Motion::Left),
                    'j' => Some(Motion::Down),
                    'k' => Some(Motion::Up),
                    'l' => Some(Motion::Right),
                    _ => None
                }
            }
            _ => None
        }
    }

    fn p_motion(i: Range) -> IResult<Range, Self> {
        map_opt(R::take(1), Self::_next)(i)
    }

    fn motion() -> impl FnMut(Range) -> IResult<Range, Self> {
        |i| Self::p_motion(i)
    }

}

#[derive(Eq, PartialEq, Debug, Clone)]
enum T {
    Number(usize),
    Char(char),
    Command(Command),
    Range(usize, usize),
    //Motion(usize, Motion)
}
impl From<Command> for T {
    fn from(item: Command) -> Self {
        T::Command(item)
    }
}

impl<'a> T {
    fn range() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, T> {
        |i| Self::p_range(i)
    }

    fn p_range(r: Range<'a>) -> IResult<Range<'a>, T> {
        // [number],[number]
        match tuple((R::number(), R::tag(&[Elem::Char(',')]), R::number()))(r) {
            Ok((rest, v)) => Ok((rest, T::Range(v.0, v.2))),
            Err(e) => Err(e)
        }
    }

    pub fn _number() -> impl FnMut(Range<'a>) -> IResult<Range<'a>, T>
    {
        |i| map(R::number(), |n: usize| T::Number(n))(i)
    }

    fn motion() -> impl FnMut(Range) -> IResult<Range, Command> {
        |i: Range| {
            match tuple((opt(R::number()), Motion::motion()))(i) {
                Ok((rest, (d1, d2))) => {
                    let d: Option<usize> = d1;
                    let reps: usize = d1.unwrap_or(1);
                    use Motion::*;
                    let command = match d2 {
                        Up => Command::MoveCursorY(-1 * reps as i32),
                        Down => Command::MoveCursorY(reps as i32),
                        Left => Command::MoveCursorX(-1 * reps as i32),
                        Right => Command::MoveCursorX(reps as i32),
                        _ => Command::MoveCursorX(reps as i32),
                    };
                    Ok((rest, command))
                }
                Err(e) => Err(e)
            }
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    Incomplete,
    Invalid
}

pub struct Reader {
    pub buf: Vec<Elem>,
    pub mode: Mode,
}

impl Default for Reader {
    fn default() -> Self {
        let mode = Mode::default();
        Self {
            buf: Vec::new(), mode: mode
        }
    }
}

impl Reader {
    fn handle(&mut self, c: Command) {
        let start = self.mode;
        match c {
            Command::Mode(m) => {
                self.mode = m
            }
            _ => ()
        }
        if start != self.mode {
            info!("Mode: {:?} => {:?}\r", start, self.mode);
        }

    }

    pub fn process() -> Result<(), ParseError> {
        let mut reader = Self::default();
        //let mut p = reader.kb.command();
        loop {
            let event = crossterm::event::read().unwrap();
            match event.try_into() {
                Ok(e) => {
                    reader.buf.push(e);
                    let result = reader.mode.command()(reader.buf.as_slice());
                    match result {
                        Ok((_, Command::Quit)) => return Ok(()),
                        Ok((_, x)) => {
                            info!("[{:?}] Ok: {:?}\r", &reader.mode, &x);
                            reader.buf.clear();
                            reader.handle(x);
                        }
                        Err(Err::Incomplete(_)) => {
                            info!("Incomplete: {:?}\r", (reader.buf));
                        }
                        Err(e) => {
                            info!("Error: {:?}\r", (e, &reader.buf));
                            reader.buf.clear();
                        }
                    }
                }
                Err(err) => {
                    info!("ERR: {:?}\r", (err));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range_enter(s: &str) -> Vec<Elem> {
        let mut v = range_string(s);
        v.push(Elem::Enter);
        v
    }

    fn range_string(s: &str) -> Vec<Elem> {
        s.chars().map(|x| Elem::Char(x)).collect::<Vec<Elem>>()
    }

    #[test]
    fn test_7_c() {
        let n = T::Number;
        let mut r = range_enter("1234a");
        //let mut out = Vec::new();
        let (rest, v) = R::number::<u64>()(r.as_slice()).unwrap();
        assert_eq!(rest, &[Elem::Char('a'), Elem::Enter]);
        assert_eq!(v, 1234);
    }

    #[test]
    fn test_7_c2() {
        let n = T::Number;
        let mut r = range_enter("a");
        //let mut out = Vec::new();
        let result = R::number::<u64>()(r.as_slice());
        assert_eq!(result.is_err(), true);
        //println!("R: {:?}", (result.finish()));
    }

    #[test]
    fn test_7_d() {
        let mut r = range_enter("1234");
        let (rest, v) = R::string()(r.as_slice()).unwrap();
        println!("R: {:?}", (&rest, &v));
        assert_eq!(rest, &[Elem::Enter]);
        assert_eq!(v, "1234");
    }

    #[test]
    fn test_7_1() {
        let i = range_enter("1234,4321");
        let (r, v) = T::range()(i.as_slice()).unwrap();
        assert_eq!(v, T::Range(1234, 4321));
    }

    #[test]
    fn test_7_3() {
        let i = range_enter("100j");
        let (r, v) = Mode::p_normal(i.as_slice()).unwrap();
        assert_eq!(v, Command::MoveCursorY(100));
    }

    #[test]
    fn test_7_4() {
        let i = range_enter("1234");
        let (r, v) = Mode::p_normal(i.as_slice()).unwrap();
        assert_eq!(v, Command::Line(1234));
    }
}


